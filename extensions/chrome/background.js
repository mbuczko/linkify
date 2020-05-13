(function () {

    function escapeXml(unsafe) {
        if (unsafe) {
            return unsafe.replace(/[<>&'"]/g,  c => {
                switch (c) {
                    case '<':
                        return '&lt;';
                    case '>':
                        return '&gt;';
                    case '&':
                        return '&amp;';
                    case '\'':
                        return '&apos;';
                    case '"':
                        return '&quot;';
                }
            });
        } else return '';
    }

    function request(config) {
        let xhr = new XMLHttpRequest(), postData = '';
        xhr.open(config.method, config.url, config.async);
        if (config.apikey) {
            xhr.setRequestHeader('Authorization', 'Bearer ' + config.apikey);
        }
        if (config.method === 'POST' || config.method === 'DELETE') {
            xhr.setRequestHeader('Content-Type', 'application/x-www-form-urlencoded');
        }
        if (config.async && config.callback) {
            xhr.onload = config.callback;
            xhr.onerror = config.callback;
        }
        try {
            if (config.data) {
                for (let key in config.data) {
                    if (config.data.hasOwnProperty(key)) {
                        postData += encodeURIComponent(key) + '=' + encodeURIComponent(config.data[key]) + '&';
                    }
                }
            }
            xhr.send(postData);
            return xhr;
        } catch (e) {
            return {
                'status': 0,
                'exception': e
            }
        }
    }

    function asyncRequest(config, callback) {
        config.method = config.method || 'GET';
        config.async = true;
        config.callback = function (e) {
            callback(e.target);
        };
        return request(config);
    }

    function backgroundInit() {
        chrome.declarativeContent.onPageChanged.removeRules(undefined, function () {
            chrome.declarativeContent.onPageChanged.addRules([{
                conditions: [new chrome.declarativeContent.PageStateMatcher({
                    pageUrl: {schemes: ['http', 'https']},
                })],
                actions: [new chrome.declarativeContent.ShowPageAction()]
            }])
        })

        chrome.extension.onMessage.addListener(
            function (message, sender, reply) {
                let responder = (xhr) => {
                    reply({
                        status: xhr.status,
                        response: xhr.response
                    });
                };
                switch (message.action) {
                    case 'getLink':
                        asyncRequest({
                            apikey: message.settings.token,
                            url: message.settings.server + '/links?href=' + encodeURIComponent(message.url)
                        }, responder);
                        return true;

                    case 'removeLink':
                        asyncRequest({
                            method: 'DELETE',
                            apikey: message.settings.token,
                            url: message.settings.server + '/links/' + message.linkId
                        }, responder);
                        return true;

                    case 'readLink':
                        asyncRequest({
                            method: 'POST',
                            apikey: message.settings.token,
                            url: message.settings.server + '/links/' + message.linkId + '/read'
                        }, responder);
                        return true;

                    case 'storeLink':
                        asyncRequest({
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
                        }, responder);
                        return true;

                    case 'matchLinks':
                        let q = message.query.trim();
                        asyncRequest({
                            apikey: message.settings.token,
                            url: message.settings.server + '/links?limit=10' + (q && q.length ? '&q=' + encodeURIComponent(q) : '')
                        }, responder);
                        return true;

                    case 'matchSearches':
                        asyncRequest({
                            apikey: message.settings.token,
                            url: message.settings.server + '/searches?name=' + message.searchname + '&exact=' + message.exact,
                            method: 'GET',
                        }, responder);
                        return true;

                    case 'storeSearch':
                        asyncRequest({
                            apikey: message.settings.token,
                            url: message.settings.server + '/searches',
                            method: 'POST',
                            data: {
                                name: message.name,
                                query: message.query
                            }
                        }, responder);
                        return true;

                    case 'removeSearch':
                        asyncRequest({
                            apikey: message.settings.token,
                            url: message.settings.server + '/searches/' + message.searchId,
                            method: 'DELETE'
                        }, responder);
                        return true;

                    case 'suggestTags':
                        asyncRequest({
                            apikey: message.settings.token,
                            url: message.settings.server + '/tags?name=' + encodeURIComponent(message.name),
                            method: 'GET'
                        }, responder);
                        return true;

                    case 'updateIcon':
                        chrome.tabs.query({active: true, lastFocusedWindow: true}, tabs => {
                            let activeTab = tabs[0];

                            asyncRequest({
                                apikey: message.settings.token,
                                url: message.settings.server + '/links?href=' + encodeURIComponent(activeTab.url)
                            }, result => {
                                chrome.pageAction.setIcon({
                                    tabId: activeTab.id,
                                    path: (result && result.status === 200 && JSON.parse(result.response).length) ?
                                        'icon128_full.png' :
                                        'icon128.png'
                                });
                            })
                        })
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
                        })
                }
            }
        );
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
                asyncRequest({
                    apikey: settings.token,
                    url: settings.server + '/links?limit=10' + (q && q.length ? '&q=' + encodeURIComponent(q) : '')
                }, result => {
                    if (result.status === 200) {
                        let items = JSON.parse(result.response).map(({href, name, description}) => ({
                            content: href,
                            description: `${escapeXml(name)} <dim>${escapeXml(description)}</dim>`
                        }));
                        suggest(items);
                    }
                })
            }
        })
    })

    window.addEventListener('load', backgroundInit)
})();
