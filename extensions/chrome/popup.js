
function $(id) {
    return document.getElementById(id);
}

function toggleElem(elem, show) {
    elem.style.display = show ? 'block' : 'none';
}

function showPanel(clazz, buttonz) {
    document
        .querySelectorAll('form .ly--panel')
        .forEach(e => toggleElem(e))
    document
        .querySelectorAll('button')
        .forEach(e => toggleElem(e))

    toggleElem(document.querySelector('form .ly--panel' + clazz), true)
    buttonz && buttonz.forEach(b => toggleElem(document.querySelector('button' + b), true));
}

function storeSettings(token, server) {
    return new Promise(
        (resolve, _) => {
            chrome.storage.sync.set({
                token: token,
                server: server
            }, resolve)
        })
}

function fetchSettings() {
    return new Promise(
        (resolve, reject) => {
            chrome.storage.sync.get(['token', 'server'], settings => {
                if (settings.token && settings.server) {
                    resolve(settings)
                } else {
                    reject()
                }
            })
        })
}

function fetchTabs(settings) {
    return new Promise(
        (resolve, _) => {
            chrome.tabs.query({active: true, lastFocusedWindow: true}, tabs => {
                resolve({
                    settings: settings,
                    tabs: tabs
                });
            })
        }
    )
}

function fetchLink(settings, url) {
    return new Promise(
        (resolve, reject) => {
            chrome.extension.sendMessage({
                    action: 'getLink',
                    settings: settings,
                    url: url,
                },
                result => {
                    if (result.status === 200) {
                        resolve(JSON.parse(result.response)[0]);
                    } else {
                        reject(result.status);
                    }
                })
        }
    )
}

function storeLink(settings, url, title, notes, tags, flags) {
    return new Promise(
        (resolve, reject) => {
            chrome.extension.sendMessage({
                    action: 'storeLink',
                    settings: settings,
                    url:   url,
                    tags:  tags,
                    title: title,
                    notes: notes,
                    flags: flags
                },
                result => {
                    if (result.status === 204) {
                        resolve(settings)
                    } else {
                        reject(result.status)
                    }
            })
        })
}

function removeLink(settings, url) {
    return new Promise(
        (resolve, reject) => {
            chrome.extension.sendMessage({
                    action: 'removeLink',
                    settings: settings,
                    url: url
                },
                result => {
                    if (result.status === 204) {
                        resolve(settings)
                    } else {
                        reject(result.status)
                    }
                })
        }
    )
}

function suggestTags(settings, name) {
    return new Promise(
        (resolve, reject) => {
            chrome.extension.sendMessage({
                    action: 'suggestTags',
                    settings: settings,
                    name: name || ''
                },
                result => {
                    if (result.status === 200) {
                        resolve(JSON.parse(result.response).tags);
                    } else {
                        reject(result.status);
                    }
                })
        }
    )
}

function suggestNotes(settings, tabId) {
    return new Promise(
        (resolve, reject) => {
            chrome.tabs.executeScript(tabId,
                {
                    code: 'Array.from(document.getElementsByTagName("meta"))' +
                        '.map(m => (m.getAttribute("name") || "").endsWith("description") ? m.getAttribute("content") : null)' +
                        '.filter(m => m !== null)'
                },
                results => {
                    resolve(results[0][0] || '');
                });
        }
    )
}

function updateIcon(settings) {
    chrome.extension.sendMessage({
        action: 'updateIcon',
        settings: settings
    })
}

function isTagUsed(tags, tag) {
    for (let i in tags) {
        if (tags[i] === tag) return true;
    }
}

function currentTag(input) {
    let val = input.value,
        sel = input.selectionStart,
        end = val.indexOf(' ', sel),
        tags = val
            .substring(0, end === -1 ? val.length : end)
            .split(' ')
            .filter(t => t.length);

    return tags[tags.length-1];
}

function renderTags(tags) {
    let taglist = document.getElementById('ly--taglist');

    taglist.innerHTML = '';
    if (tags.length) {
        tags.forEach((tag, _) => {
            let a = document.createElement('a'),
                text = document.createTextNode(tag);

            a.href = '#';
            a.dataset.tag = tag;
            a.addEventListener('click', selectTag);
            a.appendChild(text);
            taglist.append(a);
        })
    } else {
        let span = document.createElement('span'),
            text = document.createTextNode('nothing to suggest');
        span.appendChild(text);
        span.classList.add('no-suggests');
        taglist.append(span);
    }
}

function updateProto(input) {
    let value = input.value.split('://', 2);
    if (value.length === 2) {
        input.value = 'https://' + value[1];
    }
}

function selectTag(e) {
    let input = document.getElementById('ly--tags'),
        value = input.value,
        tags = value.split(' ').filter(t => t.length),
        tag = e.target.dataset.tag,
        sel = input.selectionStart;

    if (!isTagUsed(tags, tag)) {
        // cursor at the end of text?
        if (sel === value.length && (!sel || value[sel-1] === ' ')) {
            tags.push(tag);
        } else {
            // replace tag under the cursor with selected one
            for (let c=currentTag(input), i=0; i<tags.length; i++) {
                if (tags[i] === c) {
                    tags[i] = tag;
                    break;
                }
            }
        }
        input.value = tags.join(' ') + ' ';
        input.focus();
    }
}

document.addEventListener('DOMContentLoaded', function () {
    let href  = $('ly--url'),
        tags  = $('ly--tags'),
        hint  = $('ly--update-proto'),
        note  = $('ly--notes'),
        title = $('ly--title'),
        buttons = document.getElementsByTagName("button"),
        storeBtn = buttons[0], removeBtn = buttons[1], initBtn = buttons[2];

    fetchSettings()
        .then(fetchTabs)
        .then(({settings, tabs}) => {
            let activeTab = tabs[0];
            Promise
                .all([
                    fetchLink(settings, activeTab.url),
                    suggestNotes(settings, activeTab.id),
                    suggestTags(settings)
                ])
                .then(([link, notes, tagz]) => {
                    if (link) {
                        let currentProto = activeTab.url.split('://')[0];
                        let storedProto = link.href.split('://')[0];

                        href.value = link.href;
                        tags.value = link.tags.join(' ') + ' ';
                        note.value = link.notes;
                        title.value = link.title;

                        storeBtn.innerHTML = "Update link";

                        // protocol update possible?
                        toggleElem(hint, currentProto === 'https' && storedProto === 'http');
                        toggleElem(removeBtn, true);
                    } else {
                        href.value  = activeTab.url;
                        title.value = activeTab.title;
                        note.value  = notes;
                    }
                    renderTags(tagz);
                    tags.focus();
                })
                .catch(() => showPanel('.ly--connection-error'))
        })
        .catch(() => showPanel('.ly--uninitialized', ['.ly--init']))

    // event handlers

    hint.addEventListener('click', e => {
        updateProto(href);
        toggleElem(hint, false);
    });

    tags.addEventListener('input', e => {
        suggestTags(currentTag(e.target));
    });

    storeBtn.addEventListener('click', e => {
        fetchSettings()
            .then(settings => storeLink(
                settings,
                href.value,
                title.value,
                note.value,
                tags.value.split(' '),
                Array.from(document.getElementsByTagName('input'))
                    .filter(e=>e.type === 'checkbox' && e.checked)
                    .map(e=>e.value)
            ))
            .then(settings => {
                updateIcon(settings);
                window.close();
            })
    })

    removeBtn.addEventListener('click', e => {
        fetchSettings()
            .then(settings => removeLink(settings, href.value))
            .then(settings => {
                updateIcon(settings);
                window.close();
            })
    })

    initBtn.addEventListener('click', e => {
        storeSettings($('ly--token').value, $('ly--server').value || 'http://127.0.0.1:8001')
            .then(() => window.close())
    })
});


