import flask
from flask import request, jsonify
import requests
from bs4 import BeautifulSoup
import csv
import os
import time
import re
from urllib.parse import urljoin, quote  # ייבוא של פונקציית הקידוד
import json

# --- הגדרות ---
SCRAPE_DO_API_KEY = "token"  # החלף במפתח שלך
BASE_SCRAPE_URL = f"http://api.scrape.do?token={SCRAPE_DO_API_KEY}&render=true&geoCode=us"
OUTPUT_DIR = 'output_csvs'
DEBUG_DIR = 'debug_html'
os.makedirs(OUTPUT_DIR, exist_ok=True)
os.makedirs(DEBUG_DIR, exist_ok=True)

app = flask.Flask(__name__)

def sanitize_url_to_filename(url):
    filename = re.sub(r'https?://(www\.)?', '', url)
    filename = re.sub(r'[\\/:*?"<>|&%=]', '_', filename)
    return filename[:150] + '.html'

def fetch_and_get_next_data(url, session_id):
    """
    מבצע בקשה עם session ID, מבצע קידוד ל-URL המטרה, שומר HTML, ומחזיר את ה-__NEXT_DATA__.
    """
    # --- התיקון הקריטי: קידוד כתובת ה-URL של וולמארט ---
    encoded_url = quote(url)
    
    # בניית ה-URL המלא לבקשה עם ה-URL המקודד
    session_url = f"{BASE_SCRAPE_URL}&session={session_id}&url={encoded_url}"
    
    try:
        print(f"Fetching URL: {url} with session: {session_id}")
        response = requests.get(session_url, timeout=120)
        response.raise_for_status() # יזרוק שגיאה אם הסטטוס הוא 4xx או 5xx
        html_content = response.text

        debug_filename = sanitize_url_to_filename(url)
        debug_filepath = os.path.join(DEBUG_DIR, debug_filename)
        with open(debug_filepath, 'w', encoding='utf-8') as f:
            f.write(html_content)
        print(f"Saved debug HTML to {debug_filepath}")

        soup = BeautifulSoup(html_content, 'html.parser')
        next_data_script = soup.find('script', {'id': '__NEXT_DATA__'})
        if not next_data_script:
            print("Error: __NEXT_DATA__ script tag not found.")
            return None
        
        return json.loads(next_data_script.string)
    except Exception as e:
        print(f"An error occurred during fetch or JSON parsing: {e}")
        return None

def find_all_products_recursively(data_node):
    all_products = []
    def search(node):
        if isinstance(node, dict):
            for key in ['items', 'products', 'results']:
                if key in node and isinstance(node[key], list):
                    potential_products = node[key]
                    if potential_products and isinstance(potential_products[0], dict) and 'name' in potential_products[0]:
                        all_products.extend(potential_products)
            for value in node.values():
                search(value)
        elif isinstance(node, list):
            for item in node:
                search(item)
    search(data_node)
    return all_products

def get_base_seller_url(url):
    match = re.search(r'(https://www\.walmart\.com/global/seller/\d+)', url)
    return match.group(1) if match else None

@app.route('/scrape', methods=['POST'])
def scrape():
    original_url = request.json.get('url')
    base_seller_url = get_base_seller_url(original_url)
    if not base_seller_url:
        return jsonify({"error": "Could not parse a valid seller URL."}), 400

    print(f"Starting scrape for base seller URL: {base_seller_url}")
    session_id = f"walmart_scrape_{int(time.time())}"
    print(f"Generated Session ID: {session_id}")

    all_product_data = []
    page = 1
    
    while True:
        if page > 50:
            print("Reached page limit (50). Stopping.")
            break

        shop_all_url = f"{base_seller_url}/shopall?page={page}&ps=100"
        
        next_data = fetch_and_get_next_data(shop_all_url, session_id)
        if not next_data:
            print(f"Could not retrieve data for page {page}. Stopping.")
            break
            
        products_on_page = find_all_products_recursively(next_data)
        
        if not products_on_page:
            print(f"No products found on page {page}. Assuming end of results. Check latest debug HTML.")
            break
            
        print(f"Found {len(products_on_page)} products on page {page}.")
        
        seen_urls = {p['url'] for p in all_product_data}
        new_products_found = 0
        
        for product_json in products_on_page:
            canonical_url = product_json.get('canonicalUrl')
            if not canonical_url: continue
                
            full_url = urljoin("https://www.walmart.com", canonical_url)
            if full_url in seen_urls: continue

            price_info = product_json.get('priceInfo', {})
            rating_info = product_json.get('rating', {})

            product_info = {
                'name': product_json.get('name', 'N/A'),
                'price': price_info.get('linePrice') if price_info else product_json.get('price'),
                'reviews': rating_info.get('numberOfReviews', 0) if rating_info else 0,
                'stock': 'In Stock' if product_json.get('availabilityStatus') == 'IN_STOCK' else '0',
                'image_url': product_json.get('image', 'N/A'),
                'url': full_url
            }
            all_product_data.append(product_info)
            seen_urls.add(full_url)
            new_products_found += 1
        
        print(f"Added {new_products_found} new unique products to the list.")
        if new_products_found == 0 and page > 1:
            print("No new links were found on this page, stopping pagination.")
            break

        page += 1
        time.sleep(3)

    if not all_product_data:
        return jsonify({"message": "Scraping finished, but no products were found. Check the debug HTML files."})

    seller_id = base_seller_url.strip('/').split('/')[-1]
    filename = f"walmart_seller_{seller_id}_{int(time.time())}.csv"
    filepath = os.path.join(OUTPUT_DIR, filename)
    
    with open(filepath, 'w', newline='', encoding='utf-8-sig') as f:
        writer = csv.DictWriter(f, fieldnames=['name', 'price', 'reviews', 'stock', 'image_url', 'url'])
        writer.writeheader()
        writer.writerows(all_product_data)

    final_message = f"Scraping complete! {len(all_product_data)} products saved to: {filepath}"
    print(final_message)
    return jsonify({"message": final_message})

if __name__ == '__main__':
    app.run(host='127.0.0.1', port=5000, debug=False)