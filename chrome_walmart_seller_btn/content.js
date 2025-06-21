// קובץ: content.js - גרסה מתוקנת

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
  
  // האזנה להודעות עדכון מה-background script
  chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
    if (request.action === "updateUI") {
        const button = document.getElementById('export-csv-button');
        if (button) {
            button.disabled = request.task.status === 'processing';
            
            // --- החלק שהשתנה: ניהול טקסט הכפתור וחווית המשתמש ---
            let message = 'Export to CSV';
            switch (request.task.status) {
                case 'processing':
                    message = request.task.progress || 'Scraping...';
                    break;
                case 'completed':
                    message = 'Done! Download started...';
                    // איפוס הכפתור אחרי 3 שניות
                    setTimeout(resetButton, 3000);
                    break;
                case 'failed':
                    message = request.task.progress || 'Error! Try Again';
                    // איפוס הכפתור אחרי 3 שניות
                    setTimeout(resetButton, 3000);
                    break;
            }
            button.textContent = message;
            // --------------------------------------------------------
        }
    }
    return true; 
  });
  
  // פונקציה לאיפוס הכפתור למצבו המקורי
  function resetButton() {
      const button = document.getElementById('export-csv-button');
      if (button) {
          button.disabled = false;
          button.textContent = 'Export to CSV';
      }
  }
  
  // נסיון חוזר להזריק את הכפתור בגלל טעינה דינמית של האתר
  const injectionInterval = setInterval(() => {
    if (document.getElementById('export-csv-button')) {
        clearInterval(injectionInterval);
    } else {
        injectButton();
    }
  }, 1000);