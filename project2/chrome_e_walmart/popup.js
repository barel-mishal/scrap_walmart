const scrapeButton = document.getElementById('scrape-button');
const sellerNameSpan = document.getElementById('seller-name');
const statusDiv = document.getElementById('status');

// Function to update the UI state
function updateUI(isScraping, statusText) {
  scrapeButton.disabled = isScraping;
  statusDiv.textContent = statusText;
  if (isScraping) {
    scrapeButton.textContent = 'Scraping...';
  }
}

// Immediately try to get seller info when popup opens
document.addEventListener('DOMContentLoaded', () => {
  chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
    const tab = tabs[0];
    if (tab.url && tab.url.startsWith("https://www.walmart.com/global/seller/")) {
      chrome.scripting.executeScript({
        target: { tabId: tab.id },
        function: getSellerInfo,
      }, (injectionResults) => {
        if (chrome.runtime.lastError) {
          sellerNameSpan.textContent = "Error";
          statusDiv.textContent = "Could not access page data.";
          scrapeButton.disabled = true;
          return;
        }
        const [result] = injectionResults;
        if (result && result.result) {
          sellerNameSpan.textContent = result.result.sellerName;
        } else {
          sellerNameSpan.textContent = 'N/A';
          scrapeButton.disabled = true;
          statusDiv.textContent = "Could not find seller info on this page.";
        }
      });
    } else {
        sellerNameSpan.textContent = 'Not a seller page';
        scrapeButton.disabled = true;
    }
  });
});

// Listener for the scrape button
scrapeButton.addEventListener('click', () => {
  updateUI(true, 'Starting scan...');
  // Send a message to the background script to start the process
  chrome.runtime.sendMessage({ command: "startScraping" });
});

// Listen for status updates from the background script
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.command === "updateStatus") {
    // If there is an error in the status, display it and stop
    if (message.status.startsWith("Error:")) {
        updateUI(false, message.status);
        scrapeButton.textContent = 'Failed. Try Again?';
        scrapeButton.onclick = () => window.location.reload();
    } else {
        updateUI(true, message.status);
    }
  }
  if (message.command === "scrapingComplete") {
    scrapeButton.disabled = false;
    statusDiv.textContent = `Scraping finished. ${message.count} products found.`;
    scrapeButton.textContent = 'Download CSV';
    
    // Remove previous listeners to avoid multiple downloads
    const newScrapeButton = scrapeButton.cloneNode(true);
    scrapeButton.parentNode.replaceChild(newScrapeButton, scrapeButton);

    newScrapeButton.addEventListener('click', () => {
         chrome.runtime.sendMessage({ command: "downloadCSV" });
    });
  }
});


// This function will be injected into the page to get initial info
function getSellerInfo() {
  try {
    const nextData = JSON.parse(document.getElementById('__NEXT_DATA__').textContent);
    const sellerName = nextData.props.pageProps.initialData.seller.sellerDisplayName;
    return { sellerName };
  } catch (e) {
    return null;
  }
}