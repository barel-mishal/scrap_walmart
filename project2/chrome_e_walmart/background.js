let allProducts = [];
let sellerInfo = {};

// Listen for the "startScraping" command from the popup
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
    if (message.command === "startScraping") {
        allProducts = []; // Reset data for a new scrape
        sellerInfo = {};
        startScrapingProcess();
    }
    if (message.command === "downloadCSV") {
        downloadCSV();
    }
});

async function startScrapingProcess() {
    const tab = await getCurrentTab();
    const initialUrl = new URL(tab.url);
    const sellerId = initialUrl.pathname.split('/')[3] || 'unknown_seller';
    sellerInfo.id = sellerId;


    // First, scrape the initial page to get total pages
    const firstPageData = await scrapePage(tab.id);
    if (!firstPageData || firstPageData.error) {
        chrome.runtime.sendMessage({ command: "updateStatus", status: "Error: Could not scrape page 1." });
        return;
    }
    
    allProducts.push(...firstPageData.products);
    sellerInfo.name = firstPageData.sellerName; // Store seller name
    const totalPages = firstPageData.totalPages;
    
    chrome.runtime.sendMessage({ command: "updateStatus", status: `Scraping page 1 of ${totalPages}... Collected ${allProducts.length} products.` });

    // Loop through the rest of the pages
    for (let i = 2; i <= totalPages; i++) {
        const nextPageUrl = `${initialUrl.pathname}?page=${i}&affinityOverride=default`;
        await navigateToPage(tab.id, `https://www.walmart.com${nextPageUrl}`);
        
        const pageData = await scrapePage(tab.id);
        if (pageData && pageData.products) {
            allProducts.push(...pageData.products);
            const status = `Scraping page ${i} of ${totalPages}... Collected ${allProducts.length} products.`;
            chrome.runtime.sendMessage({ command: "updateStatus", status: status });
        } else {
            const status = `Warning: Could not scrape page ${i}. Continuing...`;
             chrome.runtime.sendMessage({ command: "updateStatus", status: status });
        }
        // Add a delay to be polite to Walmart's servers
        await new Promise(resolve => setTimeout(resolve, 1000)); // 1-second delay
    }

    // Notify popup that scraping is complete
    chrome.runtime.sendMessage({ command: "scrapingComplete", count: allProducts.length });
}

function scrapePage(tabId) {
    return new Promise((resolve) => {
        chrome.scripting.executeScript({
            target: { tabId: tabId },
            files: ['content.js'],
        }, (injectionResults) => {
            if (chrome.runtime.lastError) {
                console.error(chrome.runtime.lastError);
                resolve({ error: chrome.runtime.lastError.message });
                return;
            }
            // The result is an array, we take the first element's result
            resolve(injectionResults[0].result);
        });
    });
}

function navigateToPage(tabId, url) {
    return new Promise(resolve => {
        chrome.tabs.update(tabId, { url }, (tab) => {
            // Wait for the tab to finish loading
            chrome.tabs.onUpdated.addListener(function listener(updatedTabId, info) {
                if (info.status === 'complete' && updatedTabId === tabId) {
                    chrome.tabs.onUpdated.removeListener(listener);
                    resolve();
                }
            });
        });
    });
}


function downloadCSV() {
    if (allProducts.length === 0) {
        console.warn("No products to download.");
        return;
    }

    const headers = ["product_name", "price", "reviews_count", "stock_status", "product_url", "image_url"];
    let csvContent = headers.join(",") + "\n";

    allProducts.forEach(product => {
        const row = [
            `"${product.product_name.replace(/"/g, '""')}"`, // Handle quotes in name
            product.price,
            product.reviews_count,
            product.stock_status,
            product.product_url,
            product.image_url
        ];
        csvContent += row.join(",") + "\n";
    });

    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    
    const date = new Date().toISOString().split('T')[0]; // YYYY-MM-DD
    const filename = `walmart_seller_${sellerInfo.id}_${date}.csv`;

    chrome.downloads.download({
        url: url,
        filename: filename,
        saveAs: true
    });
}

function getCurrentTab() {
    return new Promise(resolve => {
        chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
            resolve(tabs[0]);
        });
    });
}