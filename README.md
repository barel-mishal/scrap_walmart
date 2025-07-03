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

### 2. Set Up the Environment

This project uses `uv` for Python package and environment management.

```sh
# Create a virtual environment in a .venv directory
uv venv

# Activate the virtual environment
source .venv/bin/activate
```

### 3. Install Dependencies

First, install the Rust-based scraper into your Python environment. `maturin` will build the Rust code and install it as a Python package.

```sh
# Install maturin to build the Rust extension
uv pip install maturin

# Navigate to the Rust directory and build/install it
cd rust_scrapwal
maturin develop
cd ..
```

Next, install the Python dependencies for the Flask server:

```sh
# Navigate to the Python API directory
cd python_scraper_api

# Install dependencies from pyproject.toml
uv pip install -e .
cd ..
```

### 4. Environment Variables

The API uses an environment variable to connect to the ScrapeDo service. Create a `.env` file in the `python_scraper_api` directory:

```sh
# python_scraper_api/.env
SCRAPE_DO_TOKEN="YOUR_SCRAPE_DO_TOKEN"
```

If you do not plan to use ScrapeDo, you can leave the token as a placeholder.

### 5. Run the Application

Start the Flask API server:

```sh
# Make sure you are in the root directory and the venv is active
cd python_scraper_api
flask run --host=0.0.0.0 --port=5003
```

The API server will be running at `http://0.0.0.0:5003`.

## API Usage

### Start Scraping

- **Endpoint**: `POST /scrape`
- **Body** (JSON):
  ```json
  {
    "url": "https://www.walmart.com/seller/12345",
    "use_scrapedo": false
  }
  ```
  - `url`: The URL of the Walmart seller page.
  - `use_scrapedo`: (Optional) Set to `true` to use the ScrapeDo proxy.

### Check Task Status

- **Endpoint**: `GET /status/<task_id>`
- **Description**: Poll this endpoint to get the status of a scraping task.

### Download Scraped Data

- **Endpoint**: `GET /download/<filename>`
- **Description**: Download the generated CSV file.

## Chrome Extension

You can use the provided Chrome extension to easily start a scraping task.

1.  Open Chrome and navigate to `chrome://extensions`.
2.  Enable "Developer mode".
3.  Click "Load unpacked" and select the `chrome_extension` directory from this project.
4.  Navigate to a Walmart seller page, and a "Scrape this Seller" button will appear. Clicking it will start the scraping process.
