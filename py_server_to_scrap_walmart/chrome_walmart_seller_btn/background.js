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
    // Assumes 'currentTask' is a global or accessible variable.
    // מניעת הרצה כפולה (Prevents running twice)
    if (currentTask.status === 'processing') {
        return;
    }

    // אתחול המשימה (Initialize the task)
    currentTask = { status: 'processing', progress: 'Connecting to server...' };
    updateInjectedUI();

    try {
        // Initial request to the server to start the task
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
        
        // Start the safe polling process
        pollForStatus(currentTask.id);

    } catch (error) {
        console.error("Initial request error:", error);
        currentTask.status = 'failed';
        currentTask.progress = 'Server communication error';
        updateInjectedUI();
    }
}
async function pollForStatus(taskId) {
    // Stop polling if the task is no longer the active one
    if (currentTask.id !== taskId || currentTask.status !== 'processing') {
        return; 
    }

    try {
        const statusResponse = await fetch(`http://127.0.0.1:5003/status/${taskId}`);
        if (!statusResponse.ok) {
            throw new Error(`Server returned status ${statusResponse.status}`);
        }

        const statusData = await statusResponse.json();
        
        currentTask.status = statusData.status;

        // --- Your progress formatting logic ---
        if (statusData.status === 'processing' && statusData.progress) {
            const match = statusData.progress.match(/(\d+)\/(\d+)/);
            if (match) {
                currentTask.progress = `Processing: ${match[1]} / ${match[2]} pages`;
            } else {
                currentTask.progress = statusData.progress;
            }
        } else {
            currentTask.progress = statusData.progress || `Status: ${statusData.status}`;
        }
        // ------------------------------------
        
        updateInjectedUI();

        // Check if the task is finished
        if (statusData.status === 'completed') {
            chrome.downloads.download({ url: `http://127.0.0.1:5003/download/${statusData.file}` });
            // Stop polling by not calling setTimeout again
        } else if (statusData.status === 'failed') {
            // Task failed on the server, stop polling
            console.error(`Task failed on server: ${statusData.progress}`);
        } else {
            // If still processing, schedule the next poll
            setTimeout(() => pollForStatus(taskId), 2000); // 2 second delay
        }

    } catch (pollError) {
        console.error("Polling error:", pollError);
        currentTask.status = 'failed';
        currentTask.progress = 'Connection lost';
        updateInjectedUI();
        // Stop polling due to connection error
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