# Walmart Seller Page Scraper

This project is a web scraping solution to extract product data from Walmart seller pages. It consists of a Python Flask API that uses a high-performance Rust scraping library, and a simple Chrome extension to trigger the scraping process.

## Features

- **High-performance scraping**: Core scraping logic is written in Rust for speed and efficiency.
- **Web API**: A Flask server provides endpoints to start scraping tasks, check their status, and download the results.
- **Concurrent Scraping**: Scrapes multiple pages of a seller's catalog concurrently.
- **Resilient**: Includes a retry mechanism for failed network requests.
- **Structured Output**: Saves scraped data into a CSV file with a descriptive name (`products_<seller_id>_<timestamp>.csv`).
- **Optional Proxy/Render API**: Can be configured to use services like ScrapeDo for handling proxies and JavaScript rendering.

## Project Structure

```
.
├── chrome_extension/      # Chrome extension to trigger the scraper
├── python_scraper_api/    # Flask API server
└── rust_scrapwal/         # Rust library for the core scraping logic
```

## Prerequisites

Before you begin, ensure you have the following installed:

- [Python 3.12+](https://www.python.org/)
- [Rust and Cargo](https://www.rust-lang.org/tools/install)
- [uv](https://github.com/astral-sh/uv) (An extremely fast Python package installer and resolver)
- make

You can install `uv` with:
```sh
curl -LsSf https://astral.sh/uv/install.sh | sh
```

## Getting Started

### 1. Clone the Repository

```sh
git clone <your-repository-url>
cd walmart-initial-scraper
```

### 2. Setup Project

Run the following command to set up the virtual environment and install all dependencies.

```sh
make setup
```

After the setup is complete, activate the virtual environment:

```sh
source .venv/bin/activate
```

### 3. Environment Variables

The API uses an environment variable to connect to the ScrapeDo service. Create a `.env` file in the `python_scraper_api` directory:

```sh
# python_scraper_api/.env
SCRAPE_DO_TOKEN="YOUR_SCRAPE_DO_TOKEN"
```

If you do not plan to use ScrapeDo, you can leave the token as a placeholder.

### 4. Run the Application

Start the FastAPI server:

```sh
# Make sure you are in the root directory and the venv is active
cd python_scraper_api
uvicorn main:app --host=0.0.0.0 --port=5003 --reload
```

## How to Work with the Code

Once the setup is complete and the application is running, you can start making changes.

### Python API (`python_scraper_api/`)

The Python code is located in the `python_scraper_api` directory. The main application logic is in `main.py`. If you make changes to the Python code, the `uvicorn` server with the `--reload` flag will automatically restart.

### Rust Scraper (`rust_scrapwal/`)

The Rust code is in the `rust_scrapwal` directory. After making changes to the Rust code, you need to recompile it:

```sh
# from the root directory
cd rust_scrapwal
maturin develop
cd ..
```

This will build the Rust library and make it available to your Python environment.

### Chrome Extension (`chrome_extension/`)

The Chrome extension files are in the `chrome_extension` directory. To see your changes:

1.  Open Chrome and navigate to `chrome://extensions`.
2.  Enable "Developer mode".
3.  Click "Load unpacked" and select the `chrome_extension` directory.
4.  If you make changes, click the reload button for the extension on the `chrome://extensions` page.
