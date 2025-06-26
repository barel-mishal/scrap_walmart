use std::fs::File;
use std::io::Write;
use reqwest;
use scraper::{Html, Selector};
use csv::Writer;
use serde_json::Value;
use tokio::time::{sleep, Duration}; // Import tokio's sleep functionality

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Base URL without the page parameter
    let base_url = "https://www.walmart.com/global/seller/102616245/cp/shopall?povid=LFNav_Landing%20Page_sellerpage_cat_pill_shopall";

    // Set up custom headers to mimic a browser
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36"
            .parse()
            .unwrap(),
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    // --- CSV Writer Setup ---
    // Initialize the CSV writer before the loop
    let mut wtr = Writer::from_path("products.csv")?;
    wtr.write_record(&["Product Name", "Price", "Image URL", "Reviews Count", "Stock Status"])?;
    // --- End CSV Writer Setup ---

    let mut current_page = 1;
    let mut max_pages = 1; // Start with 1, will be updated after the first page is parsed

    // Loop until we have scraped all pages
    while current_page <= max_pages {
        // Construct the URL for the current page
        let page_url = format!("{}&page={}", base_url, current_page);
        println!("--- Scraping page {} of {} ---", current_page, max_pages);

        // Make an HTTP request to get the HTML content
        let response = client.get(&page_url).send().await?.error_for_status()?;
        let body = response.text().await?;

        // Optional: Save HTML content locally for debugging
        // let mut file = File::create(format!("output_page_{}.html", current_page))?;
        // file.write_all(body.as_bytes())?;

        // Parse the HTML
        let document = Html::parse_document(&body);

        // Define the selector for the script tag containing the data
        let script_selector = Selector::parse("script#__NEXT_DATA__").unwrap();

        // Find the script element and extract the JSON
        if let Some(script_element) = document.select(&script_selector).next() {
            let json_string = script_element.inner_html();
            let json_data: Value = serde_json::from_str(&json_string)?;

            let modules = json_data
                .get("props")
                .and_then(|p| p.get("pageProps"))
                .and_then(|pp| pp.get("initialData"))
                .and_then(|id| id.get("contentLayout"))
                .and_then(|cl| cl.get("modules"))
                .and_then(|m| m.as_array());

            if let Some(modules_vec) = modules {
                // Find the ItemStack module that contains the product list
                for module in modules_vec {
                    if module.get("type").and_then(|t| t.as_str()) == Some("ItemStack") {
                        if let Some(configs) = module.get("configs") {
                            
                            // On the first page, find the total number of pages
                            if current_page == 1 {
                                if let Some(pagination_info) = configs.get("itemStacks").and_then(|is| is.get("paginationV2")) {
                                    max_pages = pagination_info.get("maxPage").and_then(|mp| mp.as_u64()).unwrap_or(1) as usize;
                                    println!("Discovered {} total pages to scrape.", max_pages);
                                }
                            }

                            // Navigate within the JSON to find the array of products
                            if let Some(items) = configs
                                .get("itemStacks")
                                .and_then(|is| is.get("itemStacks"))
                                .and_then(|isv| isv.get(0))
                                .and_then(|is0| is0.get("items"))
                                .and_then(|i| i.as_array())
                            {
                                println!("Found {} items on this page.", items.len());
                                // Loop through all products found in the JSON
                                for product in items {
                                    let product_name = product["name"].as_str().unwrap_or("Name not found").to_string();
                                    
                                    let price = product["priceInfo"]["linePriceDisplay"].as_str().unwrap_or("Price not found").to_string();
                                    
                                    let image_url = product["imageInfo"]["thumbnailUrl"].as_str().unwrap_or("Image URL not found").to_string();
                                    
                                    let reviews_count = product["numberOfReviews"].as_u64().unwrap_or(0).to_string();

                                    let stock_status = if product["availabilityStatusV2"]["value"].as_str() == Some("OUT_OF_STOCK") {
                                        "0".to_string()
                                    } else {
                                        let urgency_text = product["badges"]["groups"]
                                            .as_array()
                                            .and_then(|groups| {
                                                groups.iter().find_map(|group| {
                                                    if group["name"].as_str() == Some("urgency") {
                                                        group["members"][0]["text"].as_str().map(String::from)
                                                    } else {
                                                        None
                                                    }
                                                })
                                            });

                                        if let Some(text) = urgency_text {
                                            let parts: Vec<&str> = text.split_whitespace().collect();
                                            parts.iter()
                                                 .find_map(|&part| part.parse::<i32>().ok())
                                                 .map(|num| num.to_string())
                                                 .unwrap_or_else(|| text)
                                        } else {
                                            "Available (quantity not specified)".to_string()
                                        }
                                    };

                                    // Write the extracted data to the CSV
                                    wtr.write_record(&[
                                        &product_name,
                                        &price,
                                        &image_url,
                                        &reviews_count,
                                        &stock_status,
                                    ])?;
                                }
                            }
                            // After processing the item stack for this page, we can stop searching through modules
                            break; 
                        }
                    }
                }
            }
        } else {
            println!("Could not find the __NEXT_DATA__ script tag on page {}.", current_page);
        }

        // Increment the page counter
        current_page += 1;

        // Add a polite delay to avoid overwhelming the server, only if it's not the last page
        if current_page <= max_pages {
            println!("Waiting for 2 seconds before next request...");
            sleep(Duration::from_secs(2)).await;
        }
    }

    wtr.flush()?; // Make sure all data is written to the file
    println!("\n--- Scraping finished ---");
    println!("Data for all pages saved to products.csv");

    Ok(())
}