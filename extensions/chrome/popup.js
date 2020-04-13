document.addEventListener('DOMContentLoaded', function () {
    chrome.tabs.query({active: true, lastFocusedWindow: true}, tabs => {
        let activeTab = tabs[0];
        document.getElementById('ly--url').value = activeTab.url;
        document.getElementById('ly--title').value = activeTab.title;

        chrome.tabs.executeScript(activeTab.id, { code: 'Array.from(document.getElementsByTagName("meta"))' +
                '.map(m => (m.getAttribute("name") || "").endsWith("description") ? m.getAttribute("content") : null)' +
                '.filter(m => m !== null)' },
                results => {
            let descriptions = results[0];
            if (descriptions) {
                document.getElementById('ly--notes').value = descriptions[0] || '';
            }
        });
    });

    document.getElementById('ly--tags').addEventListener('keypress', e => {

    })
});