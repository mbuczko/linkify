(function() {
  function $(id) {
    return document.getElementById(id);
  }

  function stop(e) {
    e.stopPropagation();
    e.preventDefault();
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

  function selectNext(nodes) {
    let selected = nodes.getElementsByClassName('selected')[0], target;
    if (selected) {
      target = selected.nextSibling;
    }
    selectNode(selected, target || nodes.firstChild);
  }

  function selectPrev(nodes) {
    let selected = nodes.getElementsByClassName('selected')[0], target;
    if (selected) {
      target = selected.previousSibling;
    }
    selectNode(selected, target || nodes.lastChild);
  }

  function searchKeyDownHandler(e) {
    if (e.key === 'ArrowUp') {
      selectPrev($('ly-content-links'));
      stop(e);
    } else
    if (e.key === 'ArrowDown') {
      selectNext($('ly-content-links'));
      stop(e);
    } else
    if (e.key === 'Escape') {
      switchViews('ly-modal-selector');
      stop(e);
    }
    else
    if (e.ctrlKey && e.key === 'Enter') {
      let saveInput = $('ly-search-saver-input');
      switchViews('ly-content-inner', ['ly-search-saver']);
      saveInput.value = '';
      saveInput.focus();
    }
  }

  function saveKeyDownHandler(e) {
    let searchName = e.target.value;
    let warning = $('ly-search-saver-warning'),
        searchInput = $('ly-content-inner-input');

    if (e.key === 'Enter') {
      storeSearch(searchInput.value, searchName, function(response) {
        if (response.status === 200) {
          switchViews('ly-search-saver', ['ly-content-inner']);
          searchInput.focus();
        } else {
          console.error(response);
        }
      });
    } else
    if (e.key === 'Escape') {
      switchViews('ly-search-saver', ['ly-content-inner']);
      searchInput.focus();
      stop(e);
    } else if (searchName.length > 0) {
      fetchSearches(searchName, true, function(result) {
        if (result && result.status === 200 && JSON.parse(result.response).length) {
          warning.classList.add('ly-show');
        } else {
          warning.classList.remove('ly-show');
        }
      });
    }
  }

  function switchViews(from, to) {
    if (from) {
      $(from).classList.remove('ly-show');
    }
    for (let id in to) {
      let view = $(to[id]);
      if (view) {
        view.classList.add('ly-show');
      }
    }
  }

  function storeSearch(omnisearch, name, callback) {
    if (omnisearch.length > 0 && name.length > 0)
    chrome.extension.sendMessage(
        {
          action: 'storeSearch',
          omnisearch: omnisearch,
          searchname: name
        },
        callback
    )
  }

  function fetchSearches(name, exact, callback) {
    chrome.extension.sendMessage(
        {
          action: 'getSearches',
          searchname: name,
          exact: exact
        },
        callback
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
            let json = JSON.parse(result.response),
                ul = $('ly-content-links'),
                input = $('ly-content-inner-input'),
                link;
            let selected = ul.getElementsByClassName('selected')[0];
            if (selected) {
              selected.classList.remove('selected');
            }

            ul.innerHTML = '';
            for (link in json.slice(0, 10)) {
              let {href, description, tags} = json[link],
                  node = document.createElement('li'),
                  a = document.createElement('a'),
                  span = document.createElement('span'),
                  div = document.createElement('div'),
                  hreftext = document.createTextNode(href),
                  desctext = document.createTextNode(description);

              a.href = href;
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
    })
  }

  // inject dialog into DOM
  fetch(chrome.extension.getURL('/selector.html'))
    .then(response => response.text())
    .then(data => {
      document.body.insertAdjacentHTML('beforeend', data);

      let searchInput = $('ly-content-inner-input'),
          saveInput = $('ly-search-saver-input');

      saveInput.addEventListener('keydown', debounce(saveKeyDownHandler, 250));
      searchInput.addEventListener('keydown', searchKeyDownHandler);
      searchInput.addEventListener('input', debounce(function(e) {
        fetchLinks(e.target.value);
      }, 250));
  });


  // register listener for dialog shortcut
  window.addEventListener('keydown', function(e) {
    let modal = $('ly-modal-selector'),
        input = $('ly-content-inner-input');

    if (e.ctrlKey && e.key === '\\') {
      if (modal.classList.contains('ly-show')) {
        switchViews('ly-modal-selector');
      } else {
        switchViews('ly-search-saver');
        switchViews('ly-content-inner', ['ly-modal-selector', 'ly-content-spinner']);
        input.value = '';

        // last 10 links by default
        fetchLinks("", function() {
          switchViews('ly-content-spinner', ['ly-content-inner']);
        });
      }
    }
  }, false);
})();


