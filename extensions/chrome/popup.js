(function () {

    function $(id) {
        return document.getElementById(id);
    }

    function fetchLink(url) {
        return new Promise(
            (resolve, reject) => {
                chrome.extension.sendMessage(
                    {
                        action: 'getLink',
                        url: url
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

    function suggestTags(name) {
        return new Promise(
            (resolve, reject) => {
                chrome.extension.sendMessage(
                    {
                        action: 'suggestTags',
                        name: name || ''
                    },
                    result => {
                        if (result.status === 200) {
                            let response = JSON.parse(result.response),
                                taglist = document.getElementById('ly--taglist');

                            taglist.innerHTML = '';
                            if (response.tags.length) {
                                response.tags.forEach((tag, _) => {
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
                            resolve(response.tags);
                        } else reject(result.status);
                    })
            }
        )
    }

    function suggestNotes(tabId) {
        return new Promise(
            (resolve, reject) => {
                chrome.tabs.executeScript(tabId,
                    {
                        code: 'Array.from(document.getElementsByTagName("meta"))' +
                            '.map(m => (m.getAttribute("name") || "").endsWith("description") ? m.getAttribute("content") : null)' +
                            '.filter(m => m !== null)'
                    },
                    results => {
                        let descriptions = results[0];
                        resolve(descriptions[0] || '');
                    });
            }
        )
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

    function toggleElem(elem, show) {
        elem.style.display = show ? 'inline-block' : 'none';
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
            storeBtn  = document.getElementsByTagName("button")[0],
            deleteBtn = document.getElementsByTagName("button")[1];

        chrome.tabs.query({active: true, lastFocusedWindow: true}, tabs => {
            let activeTab = tabs[0];

            href.value = activeTab.url;

            Promise
                .all([fetchLink(activeTab.url), suggestNotes(activeTab.id), suggestTags()])
                .then(([link, notes, _]) => {
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
                        toggleElem(deleteBtn, true);
                    } else {
                        title.value = activeTab.title;
                        note.value = notes;
                    }
                    tags.focus();
                })
        });

        hint.addEventListener('click', e => {
            updateProto(href);
            toggleElem(hint, false);
        });
        tags.addEventListener('input', e => {
            suggestTags(currentTag(e.target));
        });
        deleteBtn.addEventListener('click', e => {
            chrome.extension.sendMessage(
                {
                    action: 'delLink',
                    url: href.value
                },
                result => {
                    if (result.status === 204) {
                        window.close();
                    } else {

                    }
                })
        })
        storeBtn.addEventListener('click', e => {
            chrome.extension.sendMessage(
                {
                    action: 'storeLink',
                    url:   href.value,
                    tags:  tags.value.split(' '),
                    title: title.value,
                    notes: note.value,
                    flags: Array.from(document.getElementsByTagName('input'))
                        .filter(e=>e.type === 'checkbox' && e.checked)
                        .map(e=>e.value)
                },
                result => {
                    if (result.status === 204) {
                        window.close();
                    } else {

                    }
                })
        })
    });

})();

