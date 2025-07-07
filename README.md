# 🛒 Walmart Seller Scraper

High-performance web scraper for Walmart seller pages built with Rust and Python. Achieves **124.9 products/sec** with concurrent processing.

## ⚡ Performance
- **Speed**: ~4.8s for 600 products (15 pages)
- **Concurrency**: Up to 100 concurrent requests
- **Success Rate**: 100% reliability
- **Architecture**: Rust core + Python API

## 🚀 Quick Start

### Prerequisites
- Python 3.12+
- Rust & Cargo
- [uv](https://github.com/astral-sh/uv) package manager

```bash
# Install uv
curl -LsSf https://astral.sh/uv/install.sh | sh
```

### Setup
```bash
git clone <your-repo-url>
cd walmart-seller-scraper

# Setup Python environment
cd python_scraper_api
uv venv
source .venv/bin/activate
uv pip install -r requirements.txt

# Build Rust library
cd ../rust_scrapwal
cargo build --release
maturin develop
```

### Run API Server
```bash
cd python_scraper_api
uvicorn main:app --host=0.0.0.0 --port=5003 --reload
```

## 📁 Project Structure
```
├── rust_scrapwal/         # High-performance Rust scraper core
├── python_scraper_api/    # Flask API wrapper
└── chrome_extension/      # Browser extension for easy triggering
```

## 🔧 API Usage

### Start Scraping
```bash
curl -X POST http://localhost:5003/scrape \
  -H "Content-Type: application/json" \
  -d '{"url": "https://www.walmart.com/seller/...", "use_scrapedo": false}'
```

### Check Status
```bash
curl http://localhost:5003/status/{task_id}
```

### Download Results
```bash
curl http://localhost:5003/download/{filename}
```

## 🎯 Features
- **Concurrent page scraping** - All pages processed simultaneously
- **Parallel data processing** - Rust + Rayon for CPU optimization
- **Retry mechanism** - Robust error handling
- **CSV export** - Structured data output
- **Chrome extension** - One-click scraping
- **Optional proxy support** - ScrapeDo integration

## ⚙️ Configuration

Create `.env` in `python_scraper_api/`:
```env
SCRAPE_DO_TOKEN=your_token_here
```

## 🧪 Development

### Rust Changes
```bash
cd rust_scrapwal
maturin develop  # Rebuild after changes
```

### Python Changes
Server auto-reloads with `--reload` flag

### Chrome Extension
1. Open `chrome://extensions`
2. Enable Developer mode
3. Load unpacked: `chrome_extension/`

## 📊 Output Format
CSV with columns: Name, Price, Stock Status, Reviews, Image URL, Availability

## 🚀 Performance Optimizations
- 100 concurrent requests
- 50ms request delays
- Parallel data processing with Rayon
- Zero-copy operations where possible
- HTTP connection reuse

Built with ❤️ using Rust for speed and Python for convenience.
