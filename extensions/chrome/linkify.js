/**
 *  Linkify
 */
var Linkify = (function() {
  function request(config) {
    var xhr = new XMLHttpRequest(), postData = '';
    xhr.open(config.method, config.url, config.async);
    if (config.apikey) {
      xhr.setRequestHeader('Authorization', 'Bearer ' + config.apikey);
    }
    if (config.method === 'POST') {
      xhr.setRequestHeader('Content-Type', 'application/x-www-form-urlencoded');
    }
    if (config.async && config.callback) {
      xhr.onload  = config.callback;
      xhr.onerror = config.callback;
    }
    try {
      if (config.data) {
        for (var key in config.data) {
          if (config.data.hasOwnProperty(key)) {
            postData += encodeURIComponent(key) + '=' + encodeURIComponent(
                config.data[key]) + '&';
          }
        }
      }
      xhr.send(postData);
      return xhr;
    } catch (e) {
      return {
        'status': 0,
        'exception': e
      }
    }
  }

  function asyncRequest(config, callback) {
    config.method = config.method || 'GET';
    config.async  = true;
    config.callback =  function (e) { callback(e.target); };
    return request(config);
  }

  function backgroundInit() {
    chrome.extension.onMessage.addListener(
        function(message, sender, reply) {
          if (message.action === "matchLinks") {
            asyncRequest({
              apikey: '5RXVlMCrpQVdFZqZLZSWK708R5kdWoLK',
              url: 'http://localhost:8001/?tags=api,rust'
            }, function(xhr) {
              reply({
                status: xhr.status,
                response: xhr.response
              });
            });
            return true;
          }
        }
    );
  }
  return {
    'backgroundInit': backgroundInit
  };
})();