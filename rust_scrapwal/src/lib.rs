use pyo3::{prelude::*, wrap_pyfunction};
use tokio; 
use regex::Regex;
use chrono;

use futures::stream::{self, StreamExt};
use reqwest::header::{HeaderMap, USER_AGENT};
use reqwest::{Client, Response};
use scraper::{Html, Selector};
use serde::Deserialize;
use std::fmt;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use std::fs;
use std::path::Path;
use rayon::prelude::*;

const MAX_CONCURRENT_REQUESTS: usize = 100;
const REQUEST_DELAY_MS: u64 = 50;
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 500;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Product {
    name: String,
    #[serde(default)]
    image_info: ImageInfo,
    #[serde(default)]
    price_info: PriceInfo,
    #[serde(default)]
    number_of_reviews: Option<u64>,
    #[serde(default)]
    availability_status_v2: AvailabilityStatus,
    #[serde(default)]
    badges: Badges,
}

impl Product {
    fn to_csv_record(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.price_info.line_price_display.clone(),
            self.image_info.thumbnail_url.clone(),
            self.number_of_reviews.unwrap_or(0).to_string(),
            self.get_stock_status(),
        ]
    }

    fn get_stock_status(&self) -> String {
        if self.availability_status_v2.value == "OUT_OF_STOCK" {
            return "0".to_string();
        }
        
        self.badges
            .groups
            .par_iter()
            .find_first(|group| group.name == "urgency")
            .and_then(|group| group.members.get(0))
            .and_then(|member| {
                let parts: Vec<&str> = member.text.split_whitespace().collect();
                parts
                    .par_iter()
                    .find_map_first(|&part| part.parse::<i32>().ok())
                    .map(|num| num.to_string())
            })
            .unwrap_or_else(|| "Available (quantity not specified)".to_string())
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct PriceInfo {
    #[serde(default)]
    line_price_display: String,
}
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct ImageInfo {
    #[serde(default)]
    thumbnail_url: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct AvailabilityStatus {
    #[serde(default)]
    value: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct Badges {
    #[serde(default)]
    groups: Vec<BadgeGroup>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct BadgeGroup {
    name: String,
    #[serde(default)]
    members: Vec<BadgeMember>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct BadgeMember {
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageData {
    props: Props,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Props {
    page_props: PageProps,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageProps {
    initial_data: InitialData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitialData {
    content_layout: ContentLayout,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentLayout {
    modules: Vec<Module>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Module {
    r#type: String,
    #[serde(default)]
    configs: Option<Configs>,
}

#[derive(Debug, Deserialize, Default)]
struct Configs {
    #[serde(rename = "itemStacks")]
    item_stacks: Option<ItemStacksTop>,
}

#[derive(Debug, Deserialize)]
struct ItemStacksTop {
    #[serde(rename = "paginationV2")]
    pagination: Pagination,
    #[serde(rename = "itemStacks")]
    stacks: Vec<ItemStack>,
}

#[derive(Debug, Deserialize)]
struct ItemStack {
    items: Vec<Product>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pagination {
    max_page: usize,
}

#[derive(Debug)]
pub enum ScraperError {
    Request(reqwest::Error),
    Csv(csv::Error),
    Json(serde_json::Error),
    Io(std::io::Error),
    DataNotFound(String),
    MaxRetriesExceeded(String),
}

impl fmt::Display for ScraperError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ScraperError::Request(e) => write!(f, "Request error: {}", e),
            ScraperError::Csv(e) => write!(f, "CSV error: {}", e),
            ScraperError::Json(e) => write!(f, "JSON parsing error: {}", e),
            ScraperError::Io(e) => write!(f, "IO error: {}", e),
            ScraperError::DataNotFound(s) => write!(f, "Data not found: {}", s),
            ScraperError::MaxRetriesExceeded(url) => write!(f, "Max retries exceeded for URL: {}", url),
        }
    }
}

impl std::error::Error for ScraperError {}
impl From<reqwest::Error> for ScraperError { fn from(err: reqwest::Error) -> Self { ScraperError::Request(err) } }
impl From<csv::Error> for ScraperError { fn from(err: csv::Error) -> Self { ScraperError::Csv(err) } }
impl From<serde_json::Error> for ScraperError { fn from(err: serde_json::Error) -> Self { ScraperError::Json(err) } }
impl From<std::io::Error> for ScraperError { fn from(err: std::io::Error) -> Self { ScraperError::Io(err) } }

async fn fetch_with_retries(client: &Client, url: &str) -> Result<Response, ScraperError> {
    for attempt in 0..=MAX_RETRIES {
        match client.get(url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    return Ok(response);
                } else {
                    eprintln!(
                        "Warning: Request to {} failed with status: {}. Retrying (attempt {}/{})",
                        url, response.status(), attempt + 1, MAX_RETRIES
                    );
                }
            }
            Err(e) => {
                 eprintln!(
                    "Warning: Request to {} failed with error: {}. Retrying (attempt {}/{})",
                    url, e, attempt + 1, MAX_RETRIES
                );
            }
        }
        
        if attempt < MAX_RETRIES {
            let delay = Duration::from_millis(RETRY_DELAY_MS * 2_u64.pow(attempt));
            sleep(delay).await;
        }
    }

    Err(ScraperError::MaxRetriesExceeded(url.to_string()))
}

async fn get_max_pages(
    client: &Client,
    base_url: &str,
) -> Result<usize, ScraperError> {
    let page_url = format!("{}&page=1", base_url);
    println!("Discovering total pages...");

    let response = fetch_with_retries(client, &page_url).await?;
    let body = response.text().await?;
    let document = Html::parse_document(&body);
    let script_selector = Selector::parse("script#__NEXT_DATA__").unwrap();

    let script_content = document
        .select(&script_selector)
        .next()
        .map(|e| e.inner_html())
        .ok_or_else(|| ScraperError::DataNotFound("__NEXT_DATA__ script tag on page 1".to_string()))?;

    let data: PageData = serde_json::from_str(&script_content)?;

    for module in data.props.page_props.initial_data.content_layout.modules {
        if module.r#type == "ItemStack" {
            if let Some(configs) = module.configs {
                if let Some(item_stacks_top) = configs.item_stacks {
                    return Ok(item_stacks_top.pagination.max_page);
                }
            }
        }
    }

    Err(ScraperError::DataNotFound("ItemStack module on page 1".to_string()))
}

async fn scrape_page(
    client: &Client,
    base_url: &str,
    page: usize,
) -> Result<Vec<Product>, ScraperError> {
    let page_url = format!("{}&page={}", base_url, page);
    println!("Fetching page {}...", page);

    let response = fetch_with_retries(client, &page_url).await?;
    let body = response.text().await?;
    
    let document = Html::parse_document(&body);
    let script_selector = Selector::parse("script#__NEXT_DATA__").unwrap();

    let script_content = document
        .select(&script_selector)
        .next()
        .map(|e| e.inner_html())
        .ok_or_else(|| ScraperError::DataNotFound(format!("__NEXT_DATA__ script tag on page {}", page)))?;

    let data: PageData = serde_json::from_str(&script_content)?;

    let products: Vec<Product> = data.props.page_props.initial_data.content_layout.modules
        .par_iter()
        .filter(|module| module.r#type == "ItemStack")
        .filter_map(|module| module.configs.as_ref())
        .filter_map(|configs| configs.item_stacks.as_ref())
        .flat_map(|item_stacks_top| {
            item_stacks_top.stacks
                .par_iter()
                .flat_map(|stack| &stack.items)
                .cloned()
        })
        .collect();

    if !products.is_empty() {
        Ok(products)
    } else {
        Err(ScraperError::DataNotFound(format!("ItemStack module on page {}", page)))
    }
}

fn extract_seller_id(url: &str) -> String {
    let re = Regex::new(r"/seller/(\d+)").unwrap();
    if let Some(caps) = re.captures(url) {
        if let Some(seller_id) = caps.get(1) {
            return seller_id.as_str().to_string();
        }
    }
    "unknown_seller".to_string()
}

async fn setup_client() -> Result<Client, ScraperError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36"
            .parse()
            .unwrap(),
    );
    let client = Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(30))
        .build()?;
    Ok(client)
}

async fn spawn_csv_writer_task(
    output_file: &Path,
) -> Result<(mpsc::Sender<Vec<Product>>, tokio::task::JoinHandle<Result<(), ScraperError>>), ScraperError> {
    let (tx, mut rx) = mpsc::channel::<Vec<Product>>(100);
    let writer_output_file = output_file.to_path_buf();
    let writer_handle = tokio::spawn(async move {
        let mut wtr = csv::Writer::from_path(writer_output_file)?;
        wtr.write_record(&["Product Name", "Price", "Image URL", "Reviews Count", "Stock Status"])?;
        
        while let Some(products) = rx.recv().await {
            let csv_records: Vec<Vec<String>> = products
                .par_iter()
                .map(|product| product.to_csv_record())
                .collect();
            
            for record in csv_records {
                wtr.write_record(&record)?;
            }
        }
        wtr.flush()?;
        Ok::<(), ScraperError>(())
    });
    Ok((tx, writer_handle))
}

async fn scrape_all_pages(
    client: &Client,
    base_url: &str,
    tx: mpsc::Sender<Vec<Product>>,
) -> Result<usize, ScraperError> {
    let max_pages = get_max_pages(client, base_url).await?;
    
    let concurrent_limit = std::cmp::min(max_pages, MAX_CONCURRENT_REQUESTS);
    
    println!("Discovered {} total pages. Starting concurrent scraping of ALL pages with {} concurrent requests...", max_pages, concurrent_limit);

    stream::iter(1..=max_pages)
        .for_each_concurrent(concurrent_limit, |page| {
            let client = client.clone();
            let tx = tx.clone();
            async move {
                sleep(Duration::from_millis(REQUEST_DELAY_MS)).await;
                match scrape_page(&client, base_url, page).await {
                    Ok(products) => {
                        if !products.is_empty() && tx.send(products).await.is_err() {
                            eprintln!("Error: CSV writer channel closed while sending page {}.", page);
                        }
                    }
                    Err(e) => eprintln!("Failed to scrape page {}: {}", page, e),
                }
            }
        })
        .await;

    Ok(max_pages)
}

pub async fn run_scraper(walmart_seller_site: &str) -> Result<String, ScraperError> {
    let output_dir = Path::new("main_output").join("csv");
    fs::create_dir_all(&output_dir).map_err(ScraperError::Io)?;
    
    let seller_name = extract_seller_id(walmart_seller_site);
    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d_%H%M%S");

    let file_name = format!("products_{}_{}.csv", seller_name, timestamp);
    let output_file = output_dir.join(&file_name);
    let output_file_str = output_file.to_str()
        .ok_or_else(|| ScraperError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Invalid UTF-8 in output path")))?
        .to_string();

    let client = setup_client().await?;
    let (tx, writer_handle) = spawn_csv_writer_task(&output_file).await?;

    let max_pages = scrape_all_pages(&client, walmart_seller_site, tx).await?;

    match writer_handle.await {
        Ok(Ok(_)) => {
            println!("--- Scraping finished ---");
            println!("Data for all {} pages saved to {}", max_pages, output_file_str);
            Ok(output_file_str)
        },
        Ok(Err(e)) => {
            eprintln!("CSV writer task failed: {}", e);
            Err(e)
        },
        Err(e) => {
            eprintln!("Writer task panicked: {}", e);
            Err(ScraperError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Writer task panicked",
            )))
        }
    }
}

// Python Bindings
#[pyfunction]
fn rs_run_scraper(url: String) -> PyResult<String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    match rt.block_on(run_scraper(&url)) {
        Ok(s) => Ok(s), 
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
    }
}

#[pymodule]
fn rust_scrapwal(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rs_run_scraper, m)?)?;
    Ok(())
}