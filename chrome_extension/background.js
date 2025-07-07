let currentTask = {};

chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
    if (request.action === "startScrape") {
        startScrapeProcess(request.url);
        sendResponse({ status: "started" });
    }
    return true;
});

async function startScrapeProcess(url) {
    if (currentTask.status === 'processing') {
        return;
    }

    currentTask = { status: 'processing', progress: 'Connecting to server...' };
    updateInjectedUI();

    try {
        const response = await fetch('http://127.0.0.1:5003/scrape', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ url })
        });
        
        if (response.status !== 202) {
            throw new Error(`Server responded with status ${response.status}`);
        }
        
        const data = await response.json();
        currentTask.id = data.task_id;
        
        pollForStatus(currentTask.id);

    } catch (error) {
        console.error("Initial request error:", error);
        currentTask.status = 'failed';
        currentTask.progress = 'Server communication error';
        updateInjectedUI();
    }
}

async function pollForStatus(taskId) {
    if (currentTask.id !== taskId || currentTask.status !== 'processing') {
        return; 
    }

    try {
        const response = await fetch(`http://127.0.0.1:5003/status/${taskId}`);
        const data = await response.json();
        
        currentTask.status = data.status;
        currentTask.progress = data.progress;
        
        if (data.status === 'completed' && data.file) {
            const downloadUrl = `http://127.0.0.1:5003/download/${data.file}`;
            chrome.downloads.download({ url: downloadUrl });
        }
        
        updateInjectedUI();
        
        if (data.status === 'processing') {
            setTimeout(() => pollForStatus(taskId), 1000);
        }
        
    } catch (pollError) {
        console.error("Polling error:", pollError);
        currentTask.status = 'failed';
        currentTask.progress = 'Connection lost';
        updateInjectedUI();
    }
}

function updateInjectedUI() {
    chrome.tabs.query({active: true, currentWindow: true}, function(tabs) {
        if (tabs[0]) {
            chrome.tabs.sendMessage(tabs[0].id, {
                action: "updateUI",
                task: currentTask
            });
        }
    });
}