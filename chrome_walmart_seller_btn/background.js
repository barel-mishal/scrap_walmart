// קובץ: background.js - גרסה מתוקנת

let currentTask = {};

// האזנה להודעות מה-content script
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
    if (request.action === "startScrape") {
        startScrapeProcess(request.url);
        sendResponse({ status: "started" });
    }
    // החזרת true חיונית לתקשורת אסינכרונית
    return true;
});

async function startScrapeProcess(url) {
    // מניעת הרצה כפולה
    if (currentTask.status === 'processing') return;

    // אתחול המשימה והודעה ראשונית לממשק המשתמש
    currentTask = { status: 'processing', progress: 'Connecting to server...' };
    updateInjectedUI();

    try {
        const response = await fetch('http://127.0.0.1:5003/scrape', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ url })
        });
        
        if (response.status !== 202) {
            throw new Error(`Server returned status ${response.status}`);
        }
        
        const data = await response.json();
        currentTask.id = data.task_id;
        
        // התחלת בדיקת סטטוס תקופתית מהשרת
        const intervalId = setInterval(async () => {
            try {
                const statusResponse = await fetch(`http://127.0.0.1:5003/status/${currentTask.id}`);
                const statusData = await statusResponse.json();
                
                currentTask.status = statusData.status;

                // --- החלק שהשתנה: תרגום ההתקדמות לאנגלית ---
                if (statusData.status === 'processing' && statusData.progress) {
                    const match = statusData.progress.match(/(\d+)\/(\d+)/);
                    if (match) {
                        // יצירת הודעת התקדמות באנגלית
                        currentTask.progress = `Processing: ${match[1]} / ${match[2]} pages`;
                    } else {
                        currentTask.progress = 'Processing...';
                    }
                } else {
                    currentTask.progress = statusData.progress || `Status: ${statusData.status}`;
                }
                // ---------------------------------------------
                
                // בדיקה אם המשימה הסתיימה
                if (statusData.status === 'completed' || statusData.status === 'failed') {
                    clearInterval(intervalId);
                    if(statusData.status === 'completed') {
                        // התחלת ההורדה באמצעות chrome.downloads API
                        chrome.downloads.download({ url: `http://127.0.0.1:5003/download/${statusData.file}` });
                    }
                }
                updateInjectedUI(); // עדכון הכפתור בדף עם המידע החדש

            } catch (pollError) {
                // טיפול בשגיאה בזמן בדיקת הסטטוס
                console.error("Polling error:", pollError);
                currentTask = { status: 'failed', progress: 'Connection lost' };
                clearInterval(intervalId);
                updateInjectedUI();
            }
        }, 2000); // בדיקה כל 2 שניות

    } catch (error) {
        console.error("Initial request error:", error);
        currentTask = { status: 'failed', progress: 'Server communication error' };
        updateInjectedUI();
    }
}

function updateInjectedUI() {
    // שליחת הודעה לטאב הפעיל לעדכון ממשק המשתמש (הכפתור)
    chrome.tabs.query({active: true, currentWindow: true}, function(tabs) {
        if (tabs[0] && tabs[0].id) {
            chrome.tabs.sendMessage(tabs[0].id, {
                action: "updateUI",
                task: currentTask
            });
        }
    });
}