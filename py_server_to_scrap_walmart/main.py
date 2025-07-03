
import os

from dotenv import load_dotenv
load_dotenv()
SCRAPE_DO_TOKEN = os.getenv("SCRAPE_DO_TOKEN")
import os, uuid
from flask import Flask, request, jsonify, send_from_directory
from flask_cors import CORS
from urllib.parse import quote

from scraper_logic import (
    prepare_base_url
)

import rust_scrapwal

MAX_WORKERS = 4 
MAX_ATTEMPTS_PER_URL = 3
REQUEST_DISPATCH_INTERVAL = 0.25 
ITEMS_PER_PAGE = 40

app = Flask(__name__)
CORS(app)
os.makedirs('output', exist_ok=True)
app.config['OUTPUT_FOLDER'] = 'output'
tasks = {}



@app.route('/scrape', methods=['POST'])
def start_scrape():
    user_url = request.json.get('url')
    if not user_url: return jsonify({'error': 'URL is required'}), 400
    
    task_id = str(uuid.uuid4())
    base_url = prepare_base_url(user_url)

    # THIS IS THE ONLY LINE YOU NEED TO ADD:
    # Initialize an empty dictionary for the new task_id.
    tasks[task_id] = {}
    
    # Now the following lines will work correctly because tasks[task_id] exists.
    tasks[task_id]['status'] = 'processing'
    tasks[task_id]['progress'] = 'Scraper is running...'

    file_name = rust_scrapwal.rs_run_scraper(base_url) # Your original function call
    
    tasks[task_id]['status'] = 'completed'
    tasks[task_id]['progress'] = 'Scraping finished successfully.'
    tasks[task_id]['file'] = file_name
    
    return jsonify({'file': file_name}), 202

@app.route('/status/<task_id>', methods=['GET'])
def get_status_route(task_id):
    """
    This route is polled by the client to get the current status of a task.
    """
    task = tasks.get(task_id)
    if not task:
        return jsonify({'error': 'Task not found'}), 404
    
    # Return the current state of the task
    return jsonify({
        'status': task.get('status'),
        'progress': task.get('progress'),
        'file': task.get('file', None) # Include the filename if it exists
    })

def prepare_base_url(user_url: str) -> str:
    if "YOUR_SCRAPE_DO_TOKEN" in SCRAPE_DO_TOKEN:
        print("[FATAL ERROR] The SCRAPE_DO_TOKEN has not been configured in scraper_logic.py!")
        return None, 999

    encoded_url = quote(user_url, safe='')
    api_url = f"http://api.scrape.do?token={SCRAPE_DO_TOKEN}&url={encoded_url}&render=true"

    return api_url
        

@app.route('/download/<filename>')
def download_file(filename):
    return send_from_directory(app.config['OUTPUT_FOLDER'], filename, as_attachment=True)

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5003)
