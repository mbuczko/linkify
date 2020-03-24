var LinkifyInject = (function() {
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

  // input field onkeydown handler
  function keyDownHandler(event) {
    if (event.keyCode === 38) {
      selectPrev(document.getElementById('ly-content-links'));
      event.stopPropagation();
      event.preventDefault();
    } else
    if (event.keyCode === 40) {
      selectNext(document.getElementById('ly-content-links'));
      event.stopPropagation();
      event.preventDefault();
    }
  }

  function vswitch(el, on) {
    if (on) {
      el.classList.add('ly-show');
    } else {
      el.classList.remove('ly-show');
    }
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
                ul = document.getElementById('ly-content-links'),
                input = document.getElementById('ly-content-inner-input'),
                link;
            let selected = ul.getElementsByClassName('selected')[0];
            if (selected) {
              selected.classList.remove('selected');
            }

            ul.innerHTML = '';
            for (link in json.slice(0, 10)) {
              let {href, description} = json[link],
                  node = document.createElement('li'),
                  a = document.createElement('a'),
                  span = document.createElement('span'),
                  textnode = document.createTextNode(href),
                  descnode = document.createTextNode(description);

              a.appendChild(textnode);
              a.href = href;
              span.appendChild(descnode);
              node.appendChild(a);
              node.appendChild(span);
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

      let input = document.getElementById('ly-content-inner-input');
      input.addEventListener('keydown', keyDownHandler);
      input.addEventListener('input', debounce(function(e) {
        fetchLinks(e.target.value);
      }, 250))
  });


  // register listener for dialog shortcut
  window.addEventListener('keydown', function(e) {
    let modal = document.getElementById('ly-modal-selector'),
        spinner = document.getElementById('ly-content-spinner'),
        content = document.getElementById('ly-content-inner'),
        input = document.getElementById('ly-content-inner-input');

    // escape? close the dialog.
    if (e.keyCode === 27) {
      vswitch(modal);
    }
    else
    // otherwise check if dialog was not already opened
    if (e.ctrlKey && e.keyCode === 220) {
      if (modal.classList.contains('ly-show')) {
        vswitch(modal, false);
      } else {
        vswitch(content);
        vswitch(spinner, true);
        vswitch(modal, true);

        input.value = '';

        // last 10 links by default
        fetchLinks("", function() {
          vswitch(spinner);
          vswitch(content, true);
        });
      }
    }
  }, false);
})();


