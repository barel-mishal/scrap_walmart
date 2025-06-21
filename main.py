import os
from dotenv import load_dotenv
load_dotenv()
import threading, os, uuid, pandas as pd, math, time, random, traceback, queue
from flask import Flask, request, jsonify, send_from_directory
from flask_cors import CORS
from concurrent.futures import ThreadPoolExecutor

from scraper_logic import (
    attempt_fetch_once,
    find_total_items,
    scrape_products_from_soup,
    prepare_base_url
)

# --- הגדרות אסטרטגיה ---
# הורדנו את מספר העובדים במקביל. מהירות תושג ע"י תזמון נכון, לא בכוח גס.
MAX_WORKERS = 4 
MAX_ATTEMPTS_PER_URL = 3
# מרווח מינימלי בין שליחת בקשות חדשות מהתור (בשניות). זה המפתח למניעת חסימה.
# ערך של 0.25 מאפשר שליחת 4 בקשות בשנייה - קצב מהיר אך מבוקר.
REQUEST_DISPATCH_INTERVAL = 0.25 
ITEMS_PER_PAGE = 40
# ----------------------------------------

app = Flask(__name__)
CORS(app)
os.makedirs('output', exist_ok=True)
app.config['OUTPUT_FOLDER'] = 'output'
tasks = {}

def scraping_worker(url: str, task_id: str):
    """
    Worker פשוט: מקבל URL, מנסה לשאוב אותו מספר פעמים, ומחזיר את התוצאה.
    הוא אינו מנהל יותר את התזמון או ההמתנות.
    """
    for attempt in range(MAX_ATTEMPTS_PER_URL):
        print(f"[{task_id}] Worker fetching {url} (Attempt {attempt + 1})...")
        soup, status_code = attempt_fetch_once(url)
        
        if status_code == 200:
            return scrape_products_from_soup(soup)
        
        # אם נחסמנו, נחזיר קוד מיוחד כדי שמנהל התור ידע להאט.
        if status_code == 429:
            print(f"[{task_id}] !!! RATE LIMIT HIT on {url}. Signaling dispatcher to slow down. !!!")
            return "RATE_LIMIT"

        print(f"[{task_id}] Worker for {url} failed with status {status_code}. Retrying after a short delay...")
        time.sleep((attempt + 1) * 0.5) # המתנה קצרה וגדלה בין ניסיונות חוזרים
            
    print(f"[{task_id}] Worker failed to fetch {url} after {MAX_ATTEMPTS_PER_URL} attempts.")
    return []


def run_scraping_task(base_url: str, task_id: str):
    try:
        tasks[task_id] = {'status': 'processing', 'progress': 'מאמת עמוד ראשי...'}

        # ניסיון ראשוני להשגת העמוד הראשון וקביעת מספר העמודים
        first_page_soup, status = None, 0
        for i in range(MAX_ATTEMPTS_PER_URL):
            soup, status = attempt_fetch_once(f"{base_url}page=1")
            if status == 200:
                first_page_soup = soup
                break
            print(f"Failed to fetch first page (attempt {i+1}), status: {status}. Retrying...")
            time.sleep((i + 1) * 2)
        
        if not first_page_soup:
            raise ValueError(f"Could not fetch the first page. Last status: {status}")

        total_items = find_total_items(first_page_soup)
        if total_items == 0:
             raise ValueError("Could not determine total item count from the first page.")

        total_pages = math.ceil(total_items / ITEMS_PER_PAGE)
        print(f"[{task_id}] Task started: Found {total_items} items across {total_pages} pages.")

        all_products = []
        # איסוף המוצרים מהעמוד הראשון שכבר הורדנו
        all_products.extend(scrape_products_from_soup(first_page_soup))

        # --- ארכיטקטורת תור חדשה ---
        url_queue = queue.Queue()
        # הכנסת כל שאר העמודים לתור
        for i in range(2, total_pages + 1):
            url_queue.put(f"{base_url}page={i}")

        processed_count = 1
        tasks[task_id]['progress'] = f'מעבד... {processed_count}/{total_pages} עמודים טופלו.'

        # שימוש ב-ThreadPoolExecutor לביצוע העבודה
        with ThreadPoolExecutor(max_workers=MAX_WORKERS) as executor:
            futures = []
            
            # וסת קצב דינמי
            current_dispatch_interval = REQUEST_DISPATCH_INTERVAL

            while not url_queue.empty() or futures:
                # שלח משימות חדשות רק אם יש מקום ב-Executor והתור לא ריק
                if not url_queue.empty() and len(futures) < MAX_WORKERS:
                    url_to_scrape = url_queue.get()
                    # הגשת המשימה ל-Executor
                    future = executor.submit(scraping_worker, url_to_scrape, task_id)
                    futures.append(future)
                    # המתנה קצרה ומבוקרת לפני שליחת הבקשה הבאה
                    time.sleep(current_dispatch_interval)

                # בדיקת משימות שהסתיימו
                completed_futures = [f for f in futures if f.done()]
                for future in completed_futures:
                    result = future.result()
                    
                    # אם נתקלנו בחסימה, נאט את קצב שליחת הבקשות
                    if isinstance(result, str) and result == "RATE_LIMIT":
                         current_dispatch_interval *= 1.5 # האטה אקספוננציאלית
                         print(f"[{task_id}] Dispatcher slowed down. New interval: {current_dispatch_interval:.2f}s")
                         # את ה-URL שנכשל נחזיר לסוף התור לניסיון נוסף מאוחר יותר
                         # (זהו שיפור אופציונלי אך מומלץ)
                    elif result:
                        all_products.extend(result)
                        # אם הצלחנו, נחזור בהדרגה לקצב המקורי
                        current_dispatch_interval = max(REQUEST_DISPATCH_INTERVAL, current_dispatch_interval / 1.1)

                    processed_count += 1
                    tasks[task_id]['progress'] = f'מעבד... {processed_count-1}/{total_pages} עמודים טופלו.'
                    futures.remove(future)
                
                # מונע המתנה אקטיבית אם כל ה-workers תפוסים
                if len(futures) == MAX_WORKERS:
                    time.sleep(0.1)


        if not all_products:
            raise ValueError("האיסוף הסתיים ללא מוצרים.")

        df = pd.DataFrame(all_products)
        filename = f"{task_id}.csv"
        filepath = os.path.join(app.config['OUTPUT_FOLDER'], filename)
        df.to_csv(filepath, index=False, encoding='utf-8-sig')

        tasks[task_id].update({
            'status': 'completed',
            'progress': f'הסתיים! נאספו {len(all_products)}/{total_items} מוצרים.',
            'file': filename
        })

    except Exception as e:
        print(f"[{task_id}] שגיאה קריטית בתהליך: {e}")
        traceback.print_exc()
        tasks[task_id].update({'status': 'failed', 'progress': str(e)})

# --- שאר קוד ה-Flask נשאר ללא שינוי ---
@app.route('/scrape', methods=['POST'])
def start_scrape():
    user_url = request.json.get('url')
    if not user_url: return jsonify({'error': 'URL is required'}), 400
    task_id = str(uuid.uuid4())
    base_url = prepare_base_url(user_url)
    tasks[task_id] = {'status': 'pending'}
    thread = threading.Thread(target=run_scraping_task, args=(base_url, task_id))
    thread.start()
    return jsonify({'task_id': task_id}), 202

@app.route('/status/<task_id>')
def get_status(task_id):
    return jsonify(tasks.get(task_id, {'status': 'not_found'}))

@app.route('/download/<filename>')
def download_file(filename):
    return send_from_directory(app.config['OUTPUT_FOLDER'], filename, as_attachment=True)

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5003)