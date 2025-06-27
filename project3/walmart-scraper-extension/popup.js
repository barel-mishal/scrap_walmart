document.addEventListener('DOMContentLoaded', function () {
    const scrapeBtn = document.getElementById('scrapeBtn');
    const statusDiv = document.getElementById('status');
    const downloadLink = document.getElementById('download-link');
  
    // הסתרת אלמנט ההורדה, כי אנחנו לא צריכים אותו יותר
    downloadLink.style.display = 'none';
  
    scrapeBtn.addEventListener('click', () => {
      chrome.tabs.query({ active: true, currentWindow: true }, function (tabs) {
        const currentTab = tabs[0];
        if (currentTab.url && currentTab.url.includes('walmart.com/global/seller')) {
          scrapeBtn.disabled = true;
          statusDiv.textContent = 'Scraping in progress... This may take several minutes.';
  
          fetch('http://127.0.0.1:5000/scrape', {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
            },
            body: JSON.stringify({ url: currentTab.url }),
          })
            .then(response => {
              if (!response.ok) {
                // טיפול בשגיאות מהשרת (כמו 400 או 500)
                throw new Error(`Server responded with status: ${response.status}`);
              }
              return response.json();
            })
            .then(data => {
              // הצגת ההודעה שחזרה מהשרת
              statusDiv.textContent = data.message || 'Process finished.';
            })
            .catch(error => {
              console.error('Error:', error);
              statusDiv.textContent = 'Error: Failed to connect to the local server or an error occurred. Check the server console.';
            })
            .finally(() => {
              scrapeBtn.disabled = false;
            });
        } else {
          statusDiv.textContent = 'Please navigate to a Walmart seller page first.';
        }
      });
    });
  });