(function () {

    let modal, searcher, saver;

    function $(id) {
        return document.getElementById(id);
    }

    function show(elem) {
        elem.classList.add('ly--show')
    }

    function hide(elem) {
        elem.classList.remove('ly--show')
    }

    function toggle(elem, showing) {
        if (showing) show(elem); else hide(elem);
    }

    function toggleWarning(showing) {
        toggle($('ly--content-search-saver-warning'), showing);
    }

    function switchViews(from, to, showSpinner) {
        if (from) {
            hide($(from));
            hide($('ly--content-spinner'));
        }
        if (to) {
            show($(to));
            if (showSpinner) {
                show($('ly--content-spinner'));
            }
        }
    }

    function isShortcut(e) {
        return e.key === '\\' && e.ctrlKey;
    }

    function muteEvent(event) {
        event.stopPropagation();
        event.preventDefault();
    }

    function debounce(func, wait, immediate) {
        let timeout;
        return function () {
            let context = this, args = arguments;
            let later = function () {
                timeout = null;
                if (!immediate) func.apply(context, args);
            }
            let callNow = immediate && !timeout;
            clearTimeout(timeout);
            timeout = setTimeout(later, wait);
            if (callNow) {
                func.apply(context, args);
            }
        }
    }

    function initInput(inputElem) {
        return {
            getValue: () => {
                return inputElem.value;
            },
            setValue: (query) => {
                if (typeof query !== 'undefined') {
                    inputElem.value = query;
                    inputElem.dispatchEvent(new Event('input'));
                }
                inputElem.focus();
            }
        }
    }

    function initModal(inputElem) {
        return {
            open: () => {
                inputElem.style.display = '';

                // pop up spinner by default
                switchViews('ly--content-search-saver');
                switchViews('ly--content-inner', 'ly--modal-selector', true)
            },
            close: () => {
                hide(inputElem);
                setTimeout(() => inputElem.style.display = 'none', 1000);
            },
            isOpened: () => {
                return inputElem.classList.contains('ly--show');
            }
        }
    }

    function selectNode(current, target) {
        if (current) {
            current.classList.remove('ly--selected');
        }
        if (target) {
            target.classList.add('ly--selected');
        }
    }

    function selectedNode() {
        let nodes = $('ly--content-links');
        return {
            nodes: nodes,
            selected: nodes.getElementsByClassName('ly--selected')[0]
        }
    }

    function selectNext() {
        let {nodes, selected} = selectedNode(), target = selected && selected.nextSibling;
        selectNode(selected, target || nodes.firstChild);
    }

    function selectPrev() {
        let {nodes, selected} = selectedNode(), target = selected && selected.previousSibling;
        selectNode(selected, target || nodes.lastChild);
    }

    function searchKeyDownHandler(e) {
        switch (e.key) {
            case 'ArrowUp':
                selectPrev();
                e.preventDefault();
                break;
            case 'ArrowDown':
                selectNext();
                e.preventDefault();
                break;
            case 'Escape':
                modal.close();
                break;
            case 'Enter':
                if (e.ctrlKey) {
                    switchViews('ly--content-inner', 'ly--content-search-saver');
                    hide($('ly--content-search-saver-warning'));
                    saver.setValue('');
                } else {
                    let link = selectedNode().selected.firstChild,
                        type = link.dataset.type;

                    if (type === 'search') {
                        searcher.setValue(link.dataset.query);
                    } else {
                        modal.close();
                        if (e.shiftKey) {
                            chrome.extension.sendMessage({
                                action: 'openTab',
                                url: link.href
                            });
                        } else {
                            window.location = link.href;
                        }
                    }
                }
        }
        if (!isShortcut(e)) e.stopPropagation();
    }

    function saveKeyDownHandler(e) {
        switch (e.key) {
            case 'Enter':
                storeSearch(searcher.getValue(), e.target.value, response => {
                    if (response.status === 200) {
                        switchViews('ly--content-search-saver', 'ly--content-inner');
                        searcher.setValue();
                    } else console.error(response);
                });
                break;
            case 'Escape':
                switchViews('ly--content-search-saver', 'ly--content-inner');
                searcher.setValue();
                break;
        }
        if (!isShortcut(e)) e.stopPropagation();
    }

    function onSavedSearchClickHandler(e) {
        searcher.setValue(e.target.dataset.query);
        muteEvent(e);
    }

    function renderItems(items, callback) {
        let ul = $('ly--content-links');
        ul.innerHTML = '';
        items.slice(0, 10).forEach(({link, title, notes, tags, type, handler}) => {
            let node = document.createElement('li'),
                a = document.createElement('a'),
                span = document.createElement('span'),
                div = document.createElement('div'),
                txt = document.createTextNode(title);

            a.href = link;
            a.dataset.type = type;
            a.rel = 'noopener noreferrer';

            if (type === 'search') {
                a.dataset.query = notes;
            }
            if (handler) {
                a.addEventListener('click', handler);
            }
            a.appendChild(txt);
            node.appendChild(a);
            if (tags && tags.length) {
                let span = document.createElement('span'),
                    tagsnode = document.createTextNode(tags.join(' '));

                span.classList.add('ly--tags');
                span.appendChild(tagsnode);
                div.appendChild(span);
            }
            if (notes && notes.length) {
                span.appendChild(document.createTextNode(notes));
                div.appendChild(span);
            }
            node.appendChild(div);
            ul.appendChild(node);
        });
        toggle($('ly--no-results'), items.length === 0);

        if (callback) callback();
        searcher.setValue();
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

    function storeSearch(query, name, callback) {
        if (query.length > 0 && name.length > 0) {
            fetchSettings().then(settings => {
                chrome.extension.sendMessage({
                        action: 'storeSearch',
                        settings: settings,
                        query: query,
                        name: name
                    },
                    callback)
            })

        }
    }

    function fetchSearches(name, exact, callback) {
        fetchSettings().then(settings => {
            chrome.extension.sendMessage({
                    action: 'matchSearches',
                    settings: settings,
                    searchname: name,
                    exact: exact
                },
                result => {
                    if (result.status === 200) {
                        if (exact) {
                            callback(result)
                        } else {
                            let items = JSON.parse(result.response).map(({name, query}) => ({
                                link: name,
                                title: name,
                                notes: query,
                                type: 'search',
                                handler: onSavedSearchClickHandler
                            }));
                            renderItems(items, () => callback(result));
                        }
                    }
                })
        })
    }

    function fetchLinks(query, callback) {
        fetchSettings().then(settings => {
            chrome.extension.sendMessage({
                    action: 'matchLinks',
                    settings: settings,
                    query: query
                },
                result => {
                    if (result.status === 200) {
                        let items = JSON.parse(result.response).map(({href, title, notes, tags}) => ({
                            link: href,
                            title: title,
                            notes: notes,
                            tags: tags,
                            type: 'link'
                        }));
                        renderItems(items, callback);
                    }
                })
        })
    }

    // inject dialog into DOM
    fetch(chrome.extension.getURL('/modal.html'))
        .then(response => response.text())
        .then(data => {
            document.body.insertAdjacentHTML('beforeend', data);

            let searchInput = $('ly--content-searcher-input'),
                saveInput = $('ly--content-saver-input'),
                popup = $('ly--modal-selector');

            searcher = initInput(searchInput);
            saver = initInput(saveInput);
            modal = initModal(popup);

            saveInput.addEventListener('keydown', saveKeyDownHandler);
            saveInput.addEventListener('keyup', muteEvent);
            saveInput.addEventListener('input', debounce(e => {
                let searchName = e.target.value;
                if (searchName.length > 0) {
                    fetchSearches(searchName, true, result => {
                        toggleWarning(
                            result &&
                            result.status === 200 &&
                            JSON.parse(result.response).length);
                    });
                }
            }, 250));

            searchInput.addEventListener('keydown', searchKeyDownHandler);
            searchInput.addEventListener('keyup', muteEvent);
            searchInput.addEventListener('input', debounce(e => {
                let query = e.target.value, saved = query.startsWith('@');
                toggle($('ly--searcher-hint'), !saved);
                if (saved) {
                    fetchSearches(query.substring(1), false, selectNext);
                } else {
                    fetchLinks(query, () => {
                        switchViews('ly--content-spinner', 'ly--content-inner');
                        selectNext();
                    });
                }
            }, 250));

            document.querySelector('.ly--overlay').addEventListener('click', modal.close);
        });


    // register listener for dialog shortcut
    window.addEventListener('keydown', e => {
        if (isShortcut(e)) {
            if (modal.isOpened()) {
                modal.close();
            } else {
                modal.open();
                searcher.setValue('');
            }
        }
    }, false);

    // update the icon based on the response from /links endpoint
    chrome.storage.sync.get(['token', 'server'], settings => {
        if (settings && settings.token && settings.server) {
            chrome.extension.sendMessage({
                action: 'updateIcon',
                settings: settings
            })
        }
    });
})();


