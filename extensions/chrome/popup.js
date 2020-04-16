(function () {

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
        chrome.tabs.query({active: true, lastFocusedWindow: true}, tabs => {
            let activeTab = tabs[0];
            document.getElementById('ly--url').value = activeTab.url;
            document.getElementById('ly--title').value = activeTab.title;
            Promise
                .all([fetchLink(activeTab.url), suggestNotes(activeTab.id), suggestTags()])
                .then(([link, notes, tags]) => {
                    document.getElementById('ly--notes').value = notes;
                    if (link) {
                        document.getElementById('ly--tags').value = link.tags.join(' ') + ' ';
                    }

            })
        });

        document.getElementById('ly--tags').addEventListener('input', e => {
            suggestTags(currentTag(e.target));
        })
    });
})();

