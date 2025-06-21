import requests, re, json, time, random
from urllib.parse import urlparse, urlunparse, parse_qs, urlencode, quote
from bs4 import BeautifulSoup
import os
SCRAPE_DO_TOKEN = os.getenv("DATABASE_URL")


def attempt_fetch_once(url: str):
    """
    שונתה: מבצעת ניסיון הורדה בודד ומדווחת על התוצאה.
    מחזירה:元組 (soup, status_code)
    status_code: 200 בהצלחה, קוד שגיאת HTTP בכישלון, 999 לשגיאת הגדרות, 0 לשגיאת רשת.
    """
    try:
        if "YOUR_SCRAPE_DO_TOKEN" in SCRAPE_DO_TOKEN:
            print("[FATAL ERROR] The SCRAPE_DO_TOKEN has not been configured in scraper_logic.py!")
            return None, 999

        encoded_url = quote(url)
        api_url = f"http://api.scrape.do?token={SCRAPE_DO_TOKEN}&url={encoded_url}&render=true"
        
        response = requests.get(api_url, timeout=120)
        response.raise_for_status()
        
        return BeautifulSoup(response.content, 'html.parser'), 200

    except requests.exceptions.HTTPError as e:
        return None, e.response.status_code
    except requests.exceptions.RequestException as e:
        print(f"--> Network Error on URL {url}: {e}")
        return None, 0

# --- שאר הפונקציות נשארות ללא שינוי ---
def find_total_items(soup: BeautifulSoup) -> int:
    try:
        script_tag = soup.find('script', {'id': '__NEXT_DATA__'})
        if not script_tag: return 0
        json_data = json.loads(script_tag.string)
        def find_key_recursively(data, key_to_find):
            if isinstance(data, dict):
                for key, value in data.items():
                    if key == key_to_find: return value
                    found = find_key_recursively(value, key_to_find)
                    if found is not None: return found
            elif isinstance(data, list):
                for item in data:
                    found = find_key_recursively(item, key_to_find)
                    if found is not None: return found
            return None
        total = find_key_recursively(json_data, 'totalItemCount')
        return int(total) if total is not None else 0
    except (ValueError, KeyError, AttributeError, TypeError):
        return 0

def scrape_products_from_soup(soup: BeautifulSoup) -> list:
    products_data = []
    product_cards = soup.find_all('div', class_=re.compile(r'\b(w-25|w-50|w-100)\b'))
    for card in product_cards:
        name_element = card.find('span', {'data-automation-id': 'product-title'})
        if not name_element: continue
        name = name_element.get_text(strip=True)
        price_text = 'N/A'
        price_container = card.find('div', {'data-automation-id': 'product-price'})
        if price_container:
            full_price_str = "".join(price_container.find_all(string=True, recursive=True))
            match = re.search(r'[\d,.]+', full_price_str)
            if match: price_text = match.group(0)
        stock_quantity = 'N/A'
        stock_element = card.find('div', {'data-automation-id': 'inventory-status'})
        if stock_element:
            status_text = stock_element.get_text(strip=True).lower()
            if "out of stock" in status_text: stock_quantity = '0'
            else:
                match = re.search(r'(\d+)', status_text)
                stock_quantity = match.group(1) if match else 'In Stock'
        image_element = card.find('img', {'data-testid': 'productTileImage'})
        image_url = image_element.get('src', 'N/A')
        reviews_count = '0'
        reviews_element = card.find('span', {'data-testid': 'product-reviews'})
        if reviews_element: reviews_count = reviews_element.get('data-value', '0')
        products_data.append({'Product Name': name, 'Price': price_text, 'Review Count': reviews_count, 'Stock Status': stock_quantity, 'Image URL': image_url})
    return products_data

def prepare_base_url(url: str) -> str:
    parsed_url = urlparse(url)
    query_params = parse_qs(parsed_url.query)
    query_params.pop('page', None)
    cleaned_query = urlencode(query_params, doseq=True)
    base_url = urlunparse(parsed_url._replace(query=cleaned_query))
    if '?' not in base_url: return f"{base_url}?"
    if not base_url.endswith('&') and not base_url.endswith('?'): return f"{base_url}&"
    return base_url
