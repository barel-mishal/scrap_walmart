use scarper_walmart::run_scraper;
use scarper_walmart::ScraperError;
#[tokio::main]
async fn main() -> Result<(), ScraperError> {
    // קוראים לפונקציה הראשית מהספרייה שלנו
    if let Err(e) = run_scraper().await {
        // אם הפונקציה מחזירה שגיאה, נדפיס אותה
        eprintln!("An error occurred during scraping: {}", e);
        // נחזיר את השגיאה כדי שהתהליך יסתיים עם קוד שגיאה
        return Err(e);
    }

    Ok(())
}