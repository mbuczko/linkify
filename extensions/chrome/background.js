(function() {

  // chrome.storage.sync.set({key: value}, function() {
  //   console.log('Value is set to ' + value);
  // });


  function getOption(opt, callback) {
    chrome.storage.sync.get([opt], callback);
  }

  function request(config) {
    let xhr = new XMLHttpRequest(), postData = '';
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
        for (let key in config.data) {
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
    chrome.declarativeContent.onPageChanged.removeRules(undefined, function() {
      chrome.declarativeContent.onPageChanged.addRules([{
        conditions: [new chrome.declarativeContent.PageStateMatcher({
          pageUrl: { schemes: ['http', 'https'] },
        })
        ],
        actions: [new chrome.declarativeContent.ShowPageAction()]
      }]);
    });

    chrome.extension.onMessage.addListener(
        function(message, sender, reply) {
          let responder = (xhr) => {
            reply({
              status: xhr.status,
              response: xhr.response
            });
          };
          switch (message.action) {
            case 'matchLinks':
              asyncRequest({
                apikey: 'lKnrPZUM8Lh2kBfnraLMOgttjrMwmqC4',
                url: 'http://localhost:8001/links?limit=10&omni=' + message.omnisearch
              }, responder);
              return true;

            case 'matchSearches':
              asyncRequest({
                apikey: 'lKnrPZUM8Lh2kBfnraLMOgttjrMwmqC4',
                url: 'http://localhost:8001/searches?name=' + message.searchname + '&exact=' + message.exact,
                method: 'GET',
              }, responder);
              return true;

            case 'storeSearch':
              asyncRequest({
                apikey: 'lKnrPZUM8Lh2kBfnraLMOgttjrMwmqC4',
                url: 'http://localhost:8001/searches',
                method: 'POST',
                data: {
                  name: message.searchname,
                  query: message.omnisearch
                }
              }, responder);
              return true;
            case 'openTab':
              chrome.tabs.create({
                active: true,
                url: message.url
              })
          }
        }
    );
  }
  window.addEventListener('load', backgroundInit)
})();

