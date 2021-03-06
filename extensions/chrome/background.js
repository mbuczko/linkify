(function () {

    function escapeXml(unsafe) {
        if (unsafe) {
            return unsafe.replace(/[<>&'"]/g,  c => {
                switch (c) {
                    case '<':  return '&lt;';
                    case '>':  return '&gt;';
                    case '&':  return '&amp;';
                    case '\'': return '&apos;';
                    case '"':  return '&quot;';
                }
            })
        } else return '';
    }

    function request(config, callback) {
        let postData = null, method = config.method || 'GET';
        if (config.data) {
            postData = '';
            for (let key in config.data) {
                if (config.data.hasOwnProperty(key)) {
                    postData += encodeURIComponent(key) + '=' + encodeURIComponent(config.data[key]) + '&';
                }
            }
        }
        fetch(config.url, {
            method: method,
            headers: {
                'Authorization': 'Bearer ' + config.apikey,
                'Content-Type': 'application/x-www-form-urlencoded'
            },
            body: postData
        })
            .then(response => {
                if (!response.ok) {
                    throw new Error('Network response was not ok');
                }
                return response.status === 204 ? {} : response.json();
            })
            .then(data => callback({ data: data }))
            .catch(err => callback({ error: err.message || 'Error while reaching destination URL'}));

        return true;
    }

    function backgroundInit() {
        chrome.declarativeContent.onPageChanged.removeRules(undefined, function () {
            chrome.declarativeContent.onPageChanged.addRules([{
                conditions: [new chrome.declarativeContent.PageStateMatcher({
                    pageUrl: {schemes: ['http', 'https']},
                })],
                actions: [new chrome.declarativeContent.ShowPageAction()]
            }]);
        });

        chrome.extension.onMessage.addListener(
            function (message, sender, reply) {
                switch (message.action) {
                    case 'getLink':
                        return request({
                            apikey: message.settings.token,
                            url: message.settings.server + '/links?exact=true&q=' + encodeURIComponent(message.url)
                        }, reply);

                    case 'removeLink':
                        return request({
                            method: 'DELETE',
                            apikey: message.settings.token,
                            url: message.settings.server + '/links/' + message.linkId
                        }, reply);

                    case 'readLink':
                        return request({
                            method: 'POST',
                            apikey: message.settings.token,
                            url: message.settings.server + '/links/' + message.linkId + '/read'
                        }, reply);

                    case 'storeLink':
                        return request({
                            method: 'POST',
                            apikey: message.settings.token,
                            url: message.settings.server + '/links',
                            data: {
                                href: message.url,
                                name: message.name,
                                tags: message.tags,
                                flags: message.flags,
                                description: message.description
                            }
                        }, reply);

                    case 'search':
                        let q = message.query.trimLeft();
                        return request({
                            apikey: message.settings.token,
                            url: message.settings.server + '/search?limit=10' + (q && q.length ? '&q=' + encodeURIComponent(q) : '')
                        }, reply);

                    case 'storeQuery':
                        return request({
                            apikey: message.settings.token,
                            url: message.settings.server + '/queries',
                            method: 'POST',
                            data: {
                                name: message.name,
                                query: message.query
                            }
                        }, reply);

                    case 'removeQuery':
                        return request({
                            apikey: message.settings.token,
                            url: message.settings.server + '/queries/' + message.queryId,
                            method: 'DELETE'
                        }, reply);

                    case 'suggestTags':
                        return request({
                            apikey: message.settings.token,
                            url: message.settings.server + '/tags?name=' + encodeURIComponent(message.name) + '&exclude=' + message.exclude,
                            method: 'GET'
                        }, reply);

                    case 'updateIcon':
                        chrome.tabs.query({active: true, lastFocusedWindow: true}, tabs => {
                            let activeTab = tabs[0];

                            request({
                                apikey: message.settings.token,
                                url: message.settings.server + '/links?exact=true&q=' + encodeURIComponent(activeTab.url)
                            }, ({data}) => {
                                chrome.pageAction.setIcon({
                                    tabId: activeTab.id,
                                    path: (data && data.length) ?
                                        'icon128_full.png' :
                                        'icon128.png'
                                });
                            });
                        });
                        return true;

                    case 'setIcon':
                        chrome.pageAction.setIcon({
                            path: message.iconPath,
                            tabId: message.tabId
                        });
                        break;

                    case 'openTab':
                        chrome.tabs.create({
                            active: true,
                            url: message.url
                        });
                }
            }
        );
    }

    function generateItems(data) {
        return data.map(({id, href, name, description}) => ({
            content: href || '@'+name,
            description: `${escapeXml(name)} <dim>${escapeXml(description || '')}</dim>`
        }));
    }

    // omnibox
    chrome.omnibox.onInputEntered.addListener((text, disposition) => {
        chrome.tabs.query({active: true, currentWindow: true}, function(tabs) {
            chrome.tabs.update(tabs[0].id, {url: text});
        });
    });
    chrome.omnibox.onInputChanged.addListener((text, suggest) => {
        chrome.storage.sync.get(['token', 'server'], settings => {
            if (settings.token && settings.server) {
                let q = text.trim();
                request({
                    apikey: settings.token,
                    url: settings.server + '/search?limit=10' + (q && q.length ? '&q=' + encodeURIComponent(q) : '')
                }, ({data, error}) => {
                    if (data) {
                        suggest(generateItems(data));
                    }
                });
            }
        });
    });

    window.addEventListener('load', backgroundInit);
})();
