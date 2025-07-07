function injectButton() {
    const targetElement = document.querySelector('#results-container');
    if (targetElement && !document.getElementById('export-csv-button')) {
        const button = document.createElement('button');
        button.id = 'export-csv-button';
        button.textContent = 'Export to CSV';
        
        targetElement.parentNode.insertBefore(button, targetElement);
  
        button.addEventListener('click', () => {
            chrome.runtime.sendMessage({ action: "startScrape", url: window.location.href });
        });
    }
}

chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
    if (request.action === "updateUI") {
        const button = document.getElementById('export-csv-button');
        if (button) {
            button.disabled = request.task.status === 'processing';
            
            let message = 'Export to CSV';
            switch (request.task.status) {
                case 'processing':
                    message = request.task.progress || 'Scraping...';
                    break;
                case 'completed':
                    message = 'Download Complete!';
                    setTimeout(resetButton, 3000);
                    break;
                case 'failed':
                    message = 'Error - Try Again';
                    setTimeout(resetButton, 3000);
                    break;
            }
            button.textContent = message;
        }
    }
    return true; 
});

function resetButton() {
    const button = document.getElementById('export-csv-button');
    if (button) {
        button.disabled = false;
        button.textContent = 'Export to CSV';
    }
}

const injectionInterval = setInterval(() => {
    if (document.getElementById('export-csv-button')) {
        clearInterval(injectionInterval);
    } else {
        injectButton();
    }
}, 1000);