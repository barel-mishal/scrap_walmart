// This function will be executed in the context of the Walmart page
function scrapePageData() {
    try {
        const nextDataElement = document.getElementById('__NEXT_DATA__');
        if (!nextDataElement) {
            return { error: "Could not find __NEXT_DATA__ script tag." };
        }

        const nextData = JSON.parse(nextDataElement.textContent);
        
        // --- Safely access nested properties using Optional Chaining (?.) ---
        const sellerInfo = nextData.props?.pageProps?.initialData?.seller;
        const contentLayout = nextData.props?.pageProps?.contentLayout;

        if (!sellerInfo || !contentLayout) {
            return { error: "Seller or content layout data is missing in __NEXT_DATA__." };
        }

        const sellerId = sellerInfo.sellerId || 'unknown';
        const sellerName = sellerInfo.sellerDisplayName || 'Unknown Seller';
        
        const itemStacksModule = contentLayout.modules?.find(m => m.type === 'ItemStack');
        if (!itemStacksModule) {
            // If no items are on the page, it might be the last page with no results. Return empty.
            return { products: [], totalPages: 0, sellerId: sellerId, sellerName: sellerName };
        }
        
        const pagination = itemStacksModule.configs?.itemStacks?.paginationV2;
        const totalPages = pagination ? pagination.maxPage : 1;
        const items = itemStacksModule.configs?.itemStacks?.itemStacks?.[0]?.items || [];

        // --- Extract Products ---
        const productsData = [];

        for (const item of items) {
            const productName = item.name || 'N/A';
            
            let price = 'N/A';
            if (item.priceInfo?.linePrice) {
                 price = item.priceInfo.linePrice.replace(/[^0-9.]/g, ''); // Keep only numbers and dot
            } else if (item.price) {
                 price = item.price;
            }

            const reviewsCount = item.numberOfReviews || 0;

            let stockStatus = ''; // Default to blank
            if (item.availabilityStatusV2?.value === 'OUT_OF_STOCK') {
                stockStatus = '0';
            }
            // Walmart's JSON does not seem to provide a numeric stock count, only a status.
            
            const productUrl = item.canonicalUrl ? `https://www.walmart.com${item.canonicalUrl}` : 'N/A';
            const imageUrl = item.imageInfo?.thumbnailUrl || 'N/A';

            productsData.push({
                product_name: productName,
                price: price,
                reviews_count: reviewsCount,
                stock_status: stockStatus,
                product_url: productUrl,
                image_url: imageUrl
            });
        }
        
        return {
            products: productsData,
            totalPages: totalPages,
            sellerId: sellerId,
            sellerName: sellerName
        };

    } catch (error) {
        console.error('Walmart Scraper Error:', error);
        return { error: error.message, products: [], totalPages: 0, sellerId: 'unknown' };
    }
}

// Return the result to the caller (the background script)
scrapePageData();