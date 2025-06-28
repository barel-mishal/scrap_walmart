import os
from dotenv import load_dotenv
load_dotenv()
import os, uuid
from flask import Flask, request, jsonify, send_from_directory
from flask_cors import CORS

from scraper_logic import (
    prepare_base_url
)

# import walmart_scraper;

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
    tasks[task_id] = {'status': 'pending'}
    file_name = "df"# walmart_scraper.walmart_scrap(base_url)  # Example usage of the Rust function
    return jsonify({'name': file_name}), 202

@app.route('/download/<filename>')
def download_file(filename):
    return send_from_directory(app.config['OUTPUT_FOLDER'], filename, as_attachment=True)

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5003)
