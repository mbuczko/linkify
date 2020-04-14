(function () {
    function suggestTags(name) {
        chrome.extension.sendMessage(
            {
                action: 'suggestTags',
                name: name || ''
            },
            result => {
                if (result.status === 200) {
                    let response = JSON.parse(result.response),
                        taglist = document.getElementById('ly--taglist');

                    if (response && response.tags) {
                        taglist.innerHTML = '';
                        response.tags.forEach((tag, _) => {
                            let a = document.createElement('a'),
                                text = document.createTextNode(tag);

                            a.href = '#';
                            a.dataset.tag = tag;
                            a.addEventListener('click', selectTag);
                            a.appendChild(text);
                            taglist.append(a);
                        })
                    }
                }
            })
    }

    function isTagUsed(tags, tag) {
        for (let i in tags) {
            if (tags[i] === tag) return true
        }
    }

    function selectTag(e) {

        let tag = e.target.dataset.tag,
            input = document.getElementById('ly--tags'),
            tags = input.value.split(' ').filter(t => t.length);

        if (!isTagUsed(tags, tag)) {
            tags.push(tag);
            input.focus();
            input.value = tags.join(' ');
        }
    }

    document.addEventListener('DOMContentLoaded', function () {
        chrome.tabs.query({active: true, lastFocusedWindow: true}, tabs => {
            let activeTab = tabs[0];
            document.getElementById('ly--url').value = activeTab.url;
            document.getElementById('ly--title').value = activeTab.title;

            chrome.tabs.executeScript(activeTab.id, {
                    code: 'Array.from(document.getElementsByTagName("meta"))' +
                        '.map(m => (m.getAttribute("name") || "").endsWith("description") ? m.getAttribute("content") : null)' +
                        '.filter(m => m !== null)'
                },
                results => {
                    let descriptions = results[0];
                    if (descriptions) {
                        document.getElementById('ly--notes').value = descriptions[0] || '';
                    }
                    suggestTags();
                });
        });

        document.getElementById('ly--tags').addEventListener('input', e => {
            let val = e.target.value,
                start = e.target.selectionStart,
                end = val.indexOf(' ', start),
                tags = val
                    .substring(0, end === -1 ? val.length : end)
                    .split(' ')
                    .filter(t => t.length),
                current = tags[tags.length-1];

            suggestTags(current);
        })
    });
})();

