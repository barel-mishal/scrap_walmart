use futures::stream::{self, StreamExt};
use reqwest::header::{HeaderMap, USER_AGENT};
use reqwest::{Client, Response};
use scraper::{Html, Selector};
use serde::Deserialize;
use std::fmt;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

// --- Configuration ---
const CONCURRENT_REQUESTS: usize = 10;   // Number of pages to scrape at the same time
const REQUEST_DELAY_MS: u64 = 500;       // Polite delay between batches of requests
const MAX_RETRIES: u32 = 3;              // NEW: Max number of retries for a failed request
const RETRY_DELAY_MS: u64 = 500;         // NEW: Initial delay for retries (will double each time)

// --- Structs for Deserializing JSON data ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Product {
    name: String,
    #[serde(default)]
    image_info: ImageInfo,
    #[serde(default)]
    price_info: PriceInfo,
    // CHANGED: This field can now be `null` in the JSON, so we use Option<u64>.
    // serde will deserialize a number to `Some(n)` and `null` to `None`.
    #[serde(default)]
    number_of_reviews: Option<u64>,
    #[serde(default)]
    availability_status_v2: AvailabilityStatus,
    #[serde(default)]
    badges: Badges,
}

// Helper method to process raw product data into a clean record for the CSV
impl Product {
    fn to_csv_record(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.price_info.line_price_display.clone(),
            self.image_info.thumbnail_url.clone(),
            // CHANGED: Handle the Option. If it's None, default to 0.
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
            .iter()
            .find(|group| group.name == "urgency")
            .and_then(|group| group.members.get(0))
            .and_then(|member| {
                let parts: Vec<&str> = member.text.split_whitespace().collect();
                parts
                    .iter()
                    .find_map(|&part| part.parse::<i32>().ok())
                    .map(|num| num.to_string())
            })
            .unwrap_or_else(|| "Available (quantity not specified)".to_string())
    }
}

// --- No changes to the structs below this line ---

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PriceInfo {
    #[serde(default)]
    line_price_display: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ImageInfo {
    #[serde(default)]
    thumbnail_url: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AvailabilityStatus {
    #[serde(default)]
    value: String,
}

#[derive(Debug, Deserialize, Default)]
struct Badges {
    #[serde(default)]
    groups: Vec<BadgeGroup>,
}

#[derive(Debug, Deserialize, Default)]
struct BadgeGroup {
    name: String,
    #[serde(default)]
    members: Vec<BadgeMember>,
}

#[derive(Debug, Deserialize, Default)]
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

// --- Custom Error Type (No changes) ---
#[derive(Debug)]
enum ScraperError {
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


/// NEW: Fetches a URL with a retry mechanism and exponential backoff.
/// This makes the scraper resilient to temporary network errors or server issues.
async fn fetch_with_retries(client: &Client, url: &str) -> Result<Response, ScraperError> {
    for attempt in 0..=MAX_RETRIES {
        match client.get(url).send().await {
            Ok(response) => {
                // Check if the response status is a server error (5xx) or client error (4xx) and retry if so.
                // You might want to be more specific (e.g., only retry on 5xx).
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
        
        // Don't sleep on the last attempt
        if attempt < MAX_RETRIES {
            let delay = Duration::from_millis(RETRY_DELAY_MS * 2_u64.pow(attempt));
            sleep(delay).await;
        }
    }

    Err(ScraperError::MaxRetriesExceeded(url.to_string()))
}

/// Fetches a single page and parses it to extract products and pagination info.
async fn scrape_page(
    client: &Client,
    base_url: &str,
    page: usize,
) -> Result<(Vec<Product>, Option<usize>), ScraperError> {
    let page_url = format!("{}&page={}", base_url, page);
    println!("Fetching page {}...", page);

    // CHANGED: Use the new reliable fetch function
    let response = fetch_with_retries(client, &page_url).await?;
    let body = response.text().await?;

    let document = Html::parse_document(&body);
    let script_selector = Selector::parse("script#__NEXT_DATA__").unwrap();

    let script_content = document
        .select(&script_selector)
        .next()
        .map(|e| e.inner_html())
        .ok_or_else(|| ScraperError::DataNotFound(format!("__NEXT_DATA__ script tag on page {}", page)))?;

    // IMPORTANT: A JSON error here is now less likely to be from a network hiccup
    // (since we retried) and more likely to be a genuine data format issue.
    // The fix to `number_of_reviews` should prevent the original crash.
    let data: PageData = serde_json::from_str(&script_content)?;

    for module in data.props.page_props.initial_data.content_layout.modules {
        if module.r#type == "ItemStack" {
            if let Some(configs) = module.configs {
                if let Some(item_stacks_top) = configs.item_stacks {
                    let products = item_stacks_top.stacks.into_iter().flat_map(|s| s.items).collect();
                    let max_pages = if page == 1 { Some(item_stacks_top.pagination.max_page) } else { None };
                    return Ok((products, max_pages));
                }
            }
        }
    }

    Err(ScraperError::DataNotFound(format!("ItemStack module on page {}", page)))
}


#[tokio::main]
async fn main() -> Result<(), ScraperError> {
    let base_url = "https://www.walmart.com/global/seller/102616245/cp/shopall?povid=LFNav_Landing%20Page_sellerpage_cat_pill_shopall";

    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36"
            .parse()
            .unwrap(),
    );

    let client = Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(30)) // Good practice to have a timeout
        .build()?;

    // --- CSV Writer Setup with a Channel (No changes here) ---
    let (tx, mut rx) = mpsc::channel::<Vec<Product>>(100);
    
    let writer_handle = tokio::spawn(async move {
        let mut wtr = csv::Writer::from_path("products_optimized.csv")?;
        wtr.write_record(&["Product Name", "Price", "Image URL", "Reviews Count", "Stock Status"])?;
        
        while let Some(products) = rx.recv().await {
            for product in products {
                wtr.write_record(&product.to_csv_record())?;
            }
        }
        wtr.flush()?;
        Ok::<(), ScraperError>(())
    });

    // --- Scrape First Page to Discover Total Pages ---
    // This initial scrape also benefits from the retry logic
    let (first_page_products, max_pages_option) = scrape_page(&client, base_url, 1).await?;
    let max_pages = max_pages_option.ok_or_else(|| ScraperError::DataNotFound("max_pages info on page 1".to_string()))?;
    
    println!("Discovered {} total pages to scrape. Starting concurrent scraping...", max_pages);
    tx.send(first_page_products).await.expect("CSV writer channel closed prematurely");

    // --- Scrape Remaining Pages Concurrently (No changes in this block) ---
    if max_pages > 1 {
        let page_numbers_to_scrape = 2..=max_pages;

        stream::iter(page_numbers_to_scrape)
            .for_each_concurrent(CONCURRENT_REQUESTS, |page| {
                let client = client.clone();
                let tx = tx.clone();
                async move {
                    // This small delay is still good practice to be polite between batches
                    sleep(Duration::from_millis(REQUEST_DELAY_MS)).await;
                    
                    match scrape_page(&client, base_url, page).await {
                        Ok((products, _)) => {
                            if tx.send(products).await.is_err() {
                                eprintln!("Error: CSV writer channel closed.");
                            }
                        }
                        Err(e) => {
                            // This error message now appears only after all retries have failed.
                            eprintln!("Failed to scrape page {}: {}", page, e);
                        }
                    }
                }
            })
            .await;
    }

    // --- Finalization (No changes here) ---
    drop(tx);
    writer_handle.await.unwrap()?;

    println!("\n--- Scraping finished ---");
    println!("Data for all {} pages saved to products_optimized.csv", max_pages);

    Ok(())
}