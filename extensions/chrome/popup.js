chrome.tabs.query({active: true, lastFocusedWindow: true}, tabs => {
    document.getElementById('ly--url').value = tabs[0].url;
});

