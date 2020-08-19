(function () {

    let Modal, Finder, Saver;

    function $(id) {
        return document.getElementById(id);
    }

    function show(elem) {
        elem.classList.add('ly--show');
    }

    function hide(elem) {
        elem.classList.remove('ly--show');
    }

    function toggle(elem, showing) {
        if (showing) show(elem); else hide(elem);
    }

    function toggleWarning(showing) {
        toggle($('ly--content-query-saver-warning'), showing);
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

    function isSavedQuery(query) {
        return query.startsWith('@');
    }

    function muteEvent(event) {
        event.stopPropagation();
        if (event.type !== 'keypress') {
            event.preventDefault();
        }
    }

    function debounce(func, wait, immediate) {
        let timeout;
        return function () {
            let context = this, args = arguments;
            let later = function () {
                timeout = null;
                if (!immediate) func.apply(context, args);
            };
            let callNow = immediate && !timeout;
            clearTimeout(timeout);
            timeout = setTimeout(later, wait);
            if (callNow) {
                func.apply(context, args);
            }
        };
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
        };
    }

    function initModal(inputElem) {
        return {
            open: () => {
                inputElem.style.display = '';

                // pop up spinner by default
                switchViews('ly--content-query-saver');
                switchViews('ly--content-inner', 'ly--modal-selector', true);
            },
            close: () => {
                hide(inputElem);
                setTimeout(() => inputElem.style.display = 'none', 500);
            },
            isOpened: () => {
                return inputElem.classList.contains('ly--show');
            }
        };
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
        };
    }

    function selectNext() {
        let {nodes, selected} = selectedNode(), target = selected && selected.nextSibling;
        selectNode(selected, target || nodes.firstChild);
    }

    function selectPrev() {
        let {nodes, selected} = selectedNode(), target = selected && selected.previousSibling;
        selectNode(selected, target || nodes.lastChild);
    }

    function queryKeyDownHandler(e) {
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
                Modal.close();
                break;
            case 'Enter':
                let selected = selectedNode().selected,
                    query = Finder.getValue();

                if (e.ctrlKey) {
                    if (query.length > 0) {
                        switchViews('ly--content-inner', 'ly--content-query-saver');
                        hide($('ly--content-query-saver-warning'));
                        Saver.setValue('');
                    }
                } else if (selected) {
                    let link = Array
                            .from(selected.children)
                            .filter(e => e.tagName === 'A')[0],
                        type = link.dataset.type;

                    if (type === 'query') {
                        Finder.setValue(link.dataset.query);
                    } else {
                        Modal.close();
                        if (e.shiftKey) {
                            chrome.extension.sendMessage({
                                action: 'openTab',
                                url: link.href
                            }, _ => readLink(link.dataset.id));
                        } else {
                            window.location = link.href;
                            readLink(link.dataset.id);
                        }
                    }
                }
        }
        if (!isShortcut(e)) {
            e.stopPropagation();
        }
    }

    function saveKeyDownHandler(e) {
        switch (e.key) {
            case 'Enter':
                storeQuery(Finder.getValue(), e.target.value, ({data, error}) => {
                    if (data) {
                        switchViews('ly--content-query-saver', 'ly--content-inner');
                        Finder.setValue();
                    } else console.error(error);
                });
                break;
            case 'Escape':
                switchViews('ly--content-query-saver', 'ly--content-inner');
                Finder.setValue();
                break;
        }
        e.stopPropagation();
    }

    function onSavedQueryClickHandler(e) {
        Finder.setValue(e.target.dataset.query);
        muteEvent(e);
    }

    function onSavedQueryDeleteClickHandler(e) {
        removeQuery(e.target.dataset.id, () => Finder.setValue(Finder.getValue()));
        muteEvent(e);
    }

    function createTagNode(tags, clazz) {
        let span = document.createElement('span'),
            tagsnode = document.createTextNode(tags);
        span.appendChild(tagsnode);
        span.classList.add('ly--tags');
        if (clazz) {
            span.classList.add(clazz);
        }
        return span;
    }

    function renderItems(items, callback) {
        let ul = $('ly--content-links');
        ul.innerHTML = '';
        items.slice(0, 10).forEach(({url, name, description, tags, type, handler, toread, favourite, id}) => {
            let node = document.createElement('li'),
                a = document.createElement('a'),
                span = document.createElement('span'),
                div = document.createElement('div');

            a.href = url;
            a.dataset.type = type;
            a.dataset.id = id;
            a.rel = 'noopener noreferrer';
            a.appendChild(document.createTextNode(name));

            if (type === 'query') {
                let d = document.createElement('a'),
                    s = document.createElement('span');

                a.dataset.query = description;
                d.dataset.id = id;
                d.href = '#';
                s.classList.add('ly--delete-query');
                d.addEventListener('click', onSavedQueryDeleteClickHandler);
                d.appendChild(document.createTextNode("delete"));
                s.appendChild(document.createTextNode(" → "));
                s.appendChild(d);

                node.appendChild(a);
                node.appendChild(s);
            } else {
                node.appendChild(a);
            }
            if (handler) {
                a.addEventListener('click', handler);
            }
            if (toread) {
                div.appendChild(createTagNode('read later', 'ly--readlater'));
            }
            if (tags && tags.length) {
                div.appendChild(createTagNode(tags.join(' ')));
            }
            if (description && description.length) {
                span.appendChild(document.createTextNode(description));
                span.classList.add('ly--desc');
                div.appendChild(span);
            }
            if (favourite) {
                let star = document.createElement('span');
                star.classList.add('ly--favourite');
                star.appendChild(document.createTextNode("★"));
                node.append(star);
            }
            node.appendChild(div);
            ul.appendChild(node);
        });
        toggle($('ly--no-results'), items.length === 0);

        if (callback) callback();
        Finder.setValue();
    }

    // fetching token/server settings from the storage

    function fetchSettings() {
        return new Promise(
            (resolve, reject) => {
                chrome.storage.sync.get(['token', 'server'], settings => {
                    if (settings.token && settings.server) {
                        resolve(settings);
                    } else {
                        reject();
                    }
                });
            });
    }

    // communication with backend script

    function readLink(linkId) {
        fetchSettings().then(settings => {
            chrome.extension.sendMessage({
                action: 'readLink',
                settings: settings,
                linkId: linkId
            });
        });
    }

    function fetchLinks(query, callback) {
        fetchSettings().then(settings => {
            chrome.extension.sendMessage({
                    action: 'matchLinks',
                    settings: settings,
                    query: query
                },
                ({data, error}) => {
                    if (data) {
                        let items = data.map(({id, href, name, description, tags, toread, shared, favourite}) => ({
                            id: id,
                            url: href,
                            name: name,
                            tags: tags,
                            description: description,
                            toread: toread,
                            shared: shared,
                            favourite: favourite,
                            type: 'link'
                        }));
                        renderItems(items, callback);
                    } else console.error(error);
                });
        });
    }

    function storeQuery(query, name, callback) {
        if (query.length > 0 && name.length > 0) {
            fetchSettings().then(settings => {
                chrome.extension.sendMessage({
                        action: 'storeQuery',
                        settings: settings,
                        query: query,
                        name: name
                    },
                    callback);
            });
        }
    }

    function removeQuery(id, callback) {
        fetchSettings().then(settings => {
            chrome.extension.sendMessage({
                    action: 'removeQuery',
                    settings: settings,
                    queryId: id
                },
                callback);
        });
    }

    function fetchQueries(name, exact, callback) {
        fetchSettings().then(settings => {
            chrome.extension.sendMessage({
                    action: 'matchQueries',
                    settings: settings,
                    queryName: name,
                    exact: exact
                },
                ({data, error}) => {
                    if (data) {
                        if (exact) {
                            callback(data);
                        } else {
                            let items = data.map(({id, name, query}) => ({
                                id: id,
                                url: name,
                                name: name,
                                description: query,
                                type: 'query',
                                handler: onSavedQueryClickHandler
                            }));
                            renderItems(items, callback);
                        }
                    } else console.error(error);
                });
        });
    }

    // inject dialog into DOM
    fetch(chrome.extension.getURL('/modal.html'))
        .then(response => response.text())
        .then(data => {
            document.body.insertAdjacentHTML('beforeend', data);

            let queryInput = $('ly--content-query-input'),
                saveInput = $('ly--content-saver-input'),
                popup = $('ly--modal-selector');

            Finder = initInput(queryInput);
            Saver  = initInput(saveInput);
            Modal  = initModal(popup);

            saveInput.addEventListener('keydown', saveKeyDownHandler);
            saveInput.addEventListener('keyup', muteEvent);
            saveInput.addEventListener('keypress', muteEvent);
            saveInput.addEventListener('input', debounce(e => {
                let queryName = e.target.value;
                if (queryName.length > 0) {
                    fetchQueries(queryName, true, result => {
                        toggleWarning(result.length);
                    });
                }
            }, 250));

            queryInput.addEventListener('keydown', queryKeyDownHandler);
            queryInput.addEventListener('keyup', muteEvent);
            queryInput.addEventListener('keypress', muteEvent);
            queryInput.addEventListener('input', debounce(e => {
                let query = e.target.value, saved = isSavedQuery(query);
                toggle($('ly--query-hint'), !saved);
                if (saved) {
                    fetchQueries(query.substring(1), false, selectNext);
                } else {
                    fetchLinks(query, () => {
                        switchViews('ly--content-spinner', 'ly--content-inner');
                        selectNext();
                    });
                }
            }, 250));

            // allow for closing the dialog by clicking on overlay
            document.querySelector('.ly--overlay').addEventListener('click', Modal.close);
        });


    setTimeout(() => {

        // register listener for dialog shortcut
        window.addEventListener('keydown', e => {
            if (isShortcut(e)) {
                if (Modal.isOpened()) {
                    Modal.close();
                } else {
                    Modal.open();
                    Finder.setValue('');
                }
            }
        }, false);

        // update the icon based on the response from /links endpoint
        chrome.storage.sync.get(['token', 'server'], settings => {
            if (settings && settings.token && settings.server) {
                chrome.extension.sendMessage({
                    action: 'updateIcon',
                    settings: settings
                });
            }
        });
    }, 500);
})();


