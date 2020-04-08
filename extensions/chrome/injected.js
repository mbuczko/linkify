(function() {
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
  function stop(event) {
    event.stopPropagation();
    event.preventDefault();
  }

  function debounce(func, wait, immediate) {
    let timeout;
    return function() {
      let context = this, args = arguments;
      let later = function() {
        timeout = null;
        if (!immediate) func.apply(context, args);
      };
      let callNow = immediate && !timeout;
      clearTimeout(timeout);
      timeout = setTimeout(later, wait);
      if (callNow) {
        func.apply(context, args);
      }
    }
  }

  function selectNode(current, target) {
    if (current) {
      current.classList.remove('selected');
    }
    if (target) {
      target.classList.add('selected');
    }
  }

  function selectNext() {
    let nodes = $('ly--content-links'),
        selected = nodes.getElementsByClassName('selected')[0], target;
    if (selected) {
      target = selected.nextSibling;
    }
    selectNode(selected, target || nodes.firstChild);
  }

  function selectPrev() {
    let nodes = $('ly--content-links'),
        selected = nodes.getElementsByClassName('selected')[0], target;
    if (selected) {
      target = selected.previousSibling;
    }
    selectNode(selected, target || nodes.lastChild);
  }

  function blockKeyUpHandler(e) {
    stop(e);
  }

  function searchKeyDownHandler(e) {
    if (e.key === 'ArrowUp') {
      selectPrev();
      stop(e);
    } else
    if (e.key === 'ArrowDown') {
      selectNext();
      stop(e);
    } else
    if (e.key === 'Escape') {
      switchViews('ly--modal-selector');
    }
    else
    if (e.ctrlKey && e.key === 'Enter') {
      let saveInput = $('ly--content-saver-input'),
          warning = $('ly--content-search-saver-warning');
      switchViews('ly--content-inner', ['ly--content-search-saver']);
      hide(warning);
      saveInput.value = '';
      saveInput.focus();
    }
  }

  function saveKeyDownHandler(e) {
    let searchName = e.target.value;
    let warning = $('ly--content-search-saver-warning'),
        searchInput = $('ly--content-searcher-input');

    if (e.key === 'Enter') {
      storeSearch(searchInput.value, searchName, function(response) {
        if (response.status === 200) {
          switchViews('ly--content-search-saver', ['ly--content-inner']);
          searchInput.focus();
        } else {
          console.error(response);
        }
      });
    } else
    if (e.key === 'Escape') {
      switchViews('ly--content-search-saver', ['ly--content-inner']);
      searchInput.focus();
    } else if (searchName.length > 0) {
      fetchSearches(searchName, true, function(result) {
        toggle(warning, result && result.status === 200 && JSON.parse(result.response).length);
      });
    }
  }

  function switchViews(from, to) {
    if (from) {
      hide($(from));
    }
    for (let id in to) {
      show($(to[id]));
    }
  }

  function renderItems(items, callback) {
    let ul = $('ly--content-links'),
        input = $('ly--content-searcher-input');

    ul.innerHTML = '';
    for (let i in items.slice(0, 10)) {
      let {link, desc, tags} = items[i],
          node = document.createElement('li'),
          a = document.createElement('a'),
          span = document.createElement('span'),
          div = document.createElement('div'),
          hreftext = document.createTextNode(link),
          desctext = document.createTextNode(desc);

      a.href = link;
      a.appendChild(hreftext);
      span.appendChild(desctext);
      node.appendChild(a);
      if (tags) {
        let span = document.createElement('span'),
            tagsnode = document.createTextNode(tags.join(' '));
        span.classList.add('tags');
        span.appendChild(tagsnode);
        div.appendChild(span);
      }
      div.appendChild(span);
      node.appendChild(div);
      ul.appendChild(node);
    }
    if (callback) callback();
    input.focus();
  }

  function storeSearch(omnisearch, name, callback) {
    if (omnisearch.length > 0 && name.length > 0) {
      chrome.extension.sendMessage(
          {
            action: 'storeSearch',
            omnisearch: omnisearch,
            searchname: name
          },
          callback
      )
    }
  }

  function fetchSearches(name, exact, callback) {
    chrome.extension.sendMessage(
        {
          action: 'matchSearches',
          searchname: name,
          exact: exact
        },
        function(result) {
          if (result.status === 200) {
            let items = JSON.parse(result.response).map(({name, query}) => ({
              link: name,
              desc: query,
              tags: null
            }));
            renderItems(items, () => callback(result));
          }
        }
    )
  }

  function fetchLinks(omnisearch, callback) {
    chrome.extension.sendMessage(
        {
          action: 'matchLinks',
          omnisearch: omnisearch
        },
        function(result) {
          if (result.status === 200) {
            let items = JSON.parse(result.response).map(({href, description, tags}) => ({
              link: href,
              desc: description,
              tags: tags
            }));
            renderItems(items, callback);
          }
        }
    )
  }

  // inject dialog into DOM
  fetch(chrome.extension.getURL('/selector.html'))
    .then(response => response.text())
    .then(data => {
      document.body.insertAdjacentHTML('beforeend', data);

      let searchInput = $('ly--content-searcher-input'),
          saveInput = $('ly--content-saver-input');

      saveInput.addEventListener('keydown', debounce(saveKeyDownHandler, 250));
      saveInput.addEventListener('keyup', blockKeyUpHandler);
      searchInput.addEventListener('keyup', blockKeyUpHandler);
      searchInput.addEventListener('keydown', searchKeyDownHandler);
      searchInput.addEventListener('input', debounce(function(e) {
        let query = e.target.value;
        if (query.startsWith('@')) {
          fetchSearches(query.substring(1), false, selectNext);
        } else {
          fetchLinks(query, selectNext);
        }
      }, 250), true);
  });


  // register listener for dialog shortcut
  window.addEventListener('keydown', function(e) {
    let modal = $('ly--modal-selector'),
        input = $('ly--content-searcher-input');

    if (e.ctrlKey && e.key === '\\') {
      if (modal.classList.contains('ly--show')) {
        switchViews('ly--modal-selector');
        // wait for animation and remove contaner from page layout
        setTimeout(() => modal.style.display = 'none', 500);
      } else {
        // bring back container into page layout
        modal.style.display = '';
        input.value = '';
        switchViews('ly--content-search-saver');
        switchViews('ly--content-inner', ['ly--modal-selector', 'ly--content-spinner']);

        // last 10 links by default
        fetchLinks('', function() {
          switchViews('ly--content-spinner', ['ly--content-inner']);
        });
      }
    }
  }, false);
})();


