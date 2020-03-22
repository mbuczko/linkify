window.addEventListener('keydown', function(e) {
  let modal = document.getElementById('ly-modal-selector');
  if (e.keyCode === 27) {
    modal.classList.remove('ly-show');
  }
  else
  if (e.ctrlKey && e.keyCode === 220) {
    if (modal.classList.contains('ly-show')) {
      modal.classList.remove('ly-show');
    } else {
      document.getElementById('ly-modal-selector').classList.add('ly-show');
      chrome.extension.sendMessage({action: "matchLinks" }, function(result) {

      });
    }
  }
}, false);

fetch(chrome.extension.getURL('/selector.html'))
  .then(response => response.text())
  .then(data => { document.body.innerHTML += data; })
