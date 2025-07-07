import os
import uuid
from flask import Flask, request, jsonify, send_from_directory
from flask_cors import CORS
from urllib.parse import quote
from dotenv import load_dotenv
import rust_scrapwal

load_dotenv()
SCRAPE_DO_TOKEN = os.getenv("SCRAPE_DO_TOKEN")

app = Flask(__name__)
CORS(app)
os.makedirs('output', exist_ok=True)
app.config['OUTPUT_FOLDER'] = 'output'
tasks = {}

@app.route('/')
def index():
    return jsonify({
        'message': 'Walmart Scraper API',
        'endpoints': {
            '/scrape': 'POST with {"url": "walmart_seller_url", "use_scrapedo": false}',
            '/status/<task_id>': 'GET task status',
            '/download/<filename>': 'GET CSV file'
        }
    })

@app.route('/scrape', methods=['POST'])
def start_scrape():
    user_url = request.json.get('url')
    use_scrapedo = request.json.get('use_scrapedo', False)
    
    if not user_url: 
        return jsonify({'error': 'URL is required'}), 400
    
    task_id = str(uuid.uuid4())
    url_to_scrape = prepare_base_url(user_url) if use_scrapedo else user_url

    tasks[task_id] = {'status': 'processing', 'progress': 'Scraper is running...'}

    try:
        file_name = rust_scrapwal.rs_run_scraper(url_to_scrape) 
        tasks[task_id].update({
            'status': 'completed',
            'progress': 'Scraping finished successfully.',
            'file': file_name
        })
        return jsonify({'task_id': task_id, 'file': file_name}), 202
    except Exception as e:
        tasks[task_id].update({
            'status': 'failed',
            'progress': f'Error: {str(e)}'
        })
        return jsonify({'error': str(e)}), 500

@app.route('/status/<task_id>', methods=['GET'])
def get_status_route(task_id):
    task = tasks.get(task_id)
    if not task:
        return jsonify({'error': 'Task not found'}), 404
    
    return jsonify({
        'status': task.get('status'),
        'progress': task.get('progress'),
        'file': task.get('file', None)
    })

def prepare_base_url(user_url: str) -> str:
    if not SCRAPE_DO_TOKEN or "YOUR_SCRAPE_DO_TOKEN" in SCRAPE_DO_TOKEN:
        raise ValueError("SCRAPE_DO_TOKEN not configured")
    
    encoded_url = quote(user_url, safe='')
    return f"http://api.scrape.do?token={SCRAPE_DO_TOKEN}&url={encoded_url}&render=true"

@app.route('/download/<filename>')
def download_file(filename):
    return send_from_directory(app.config['OUTPUT_FOLDER'], filename, as_attachment=True)

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5003)
