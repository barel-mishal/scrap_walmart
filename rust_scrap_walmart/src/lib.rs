// src/lib.rs

use futures::stream::{self, StreamExt};
use reqwest::header::{HeaderMap, USER_AGENT};
use reqwest::{Client, Response};
use scraper::{Html, Selector};
use serde::Deserialize;
use std::fmt;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

// --- Configuration ---
const CONCURRENT_REQUESTS: usize = 10;
const REQUEST_DELAY_MS: u64 = 500;
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 500;

// --- Structs for Deserializing JSON data ---
// (כל ה-structs נשארים זהים, אין צורך לשנות אותם)

#[derive(Debug, Deserialize)]
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

// --- Custom Error Type ---
// הוספנו pub כדי שנוכל להשתמש בטיפוס השגיאה הזה מחוץ למודול (ב-main.rs)
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


/// Fetches a URL with a retry mechanism.
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

/// Fetches a single page and parses it to extract products.
async fn scrape_page(
    client: &Client,
    base_url: &str,
    page: usize,
) -> Result<(Vec<Product>, Option<usize>), ScraperError> {
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


/// The main public function that runs the entire scraping process.
/// זו הפונקציה הראשית של הספרייה שלנו. הוספנו 'pub' כדי שנוכל לקרוא לה מבחוץ
pub async fn run_scraper() -> Result<(), ScraperError> {
    let base_url = "https://www.walmart.com/global/seller/102616245/cp/shopall?povid=LFNav_Landing%20Page_sellerpage_cat_pill_shopall";
    let output_file = "products.csv";

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

    // --- CSV Writer Setup with a Channel ---
    let (tx, mut rx) = mpsc::channel::<Vec<Product>>(100);
    
    let writer_handle = tokio::spawn(async move {
        let mut wtr = csv::Writer::from_path(output_file)?;
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
    let (first_page_products, max_pages_option) = scrape_page(&client, base_url, 1).await?;
    let max_pages = max_pages_option.ok_or_else(|| ScraperError::DataNotFound("max_pages info on page 1".to_string()))?;
    
    println!("Discovered {} total pages to scrape. Starting concurrent scraping...", max_pages);
    tx.send(first_page_products).await.expect("CSV writer channel closed prematurely");

    // --- Scrape Remaining Pages Concurrently ---
    if max_pages > 1 {
        let page_numbers_to_scrape = 2..=max_pages;

        stream::iter(page_numbers_to_scrape)
            .for_each_concurrent(CONCURRENT_REQUESTS, |page| {
                let client = client.clone();
                let tx = tx.clone();
                async move {
                    sleep(Duration::from_millis(REQUEST_DELAY_MS)).await;
                    
                    match scrape_page(&client, base_url, page).await {
                        Ok((products, _)) => {
                            if tx.send(products).await.is_err() {
                                eprintln!("Error: CSV writer channel closed.");
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to scrape page {}: {}", page, e);
                        }
                    }
                }
            })
            .await;
    }

    // --- Finalization ---
    drop(tx); // Close the channel to signal the writer to finish.
    // Wait for the writer task to complete and handle any potential errors.
    match writer_handle.await {
        Ok(Ok(_)) => {},
        Ok(Err(e)) => return Err(e),
        Err(e) => eprintln!("Writer task panicked: {}", e),
    }

    println!("\n--- Scraping finished ---");
    println!("Data for all {} pages saved to {}", max_pages, output_file);

    Ok(())
}