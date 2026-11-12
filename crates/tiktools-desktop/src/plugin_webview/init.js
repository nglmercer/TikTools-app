(function () {
  'use strict';
  if (window.tiktools !== undefined) return;
  // Page-focused shortcuts, installed even when the bridge is unavailable:
  // F5 / Ctrl+R reload the page in place (the navigation Started event
  // resets subscriptions for the previous document); F12 / Ctrl+Shift+I
  // ask the host to open the inspector. The devtools request reads the
  // transport directly and never reaches the broker allowlist.
  window.addEventListener('keydown', function (e) {
    try {
      var reloadKey = e.key === 'F5' || ((e.ctrlKey || e.metaKey) && !e.shiftKey && (e.key === 'r' || e.key === 'R'));
      var devtoolsKey = e.key === 'F12' || ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === 'I' || e.key === 'i'));
      if (reloadKey) {
        e.preventDefault();
        window.location.reload();
      } else if (devtoolsKey) {
        e.preventDefault();
        if (window.ipc && typeof window.ipc.postMessage === 'function') {
          window.ipc.postMessage('{"type":"open-devtools"}');
        }
      }
    } catch (shortcutError) { /* never break page input */ }
  }, true);
  var ipc = (window.ipc && typeof window.ipc.postMessage === 'function') ? window.ipc : null;
  function unavailable() { return Promise.reject(new Error('plugin host is unavailable')); }
  if (!ipc) {
    window.__tiktools_transport__ = 'unavailable';
    window.tiktools = {
      settings: { get: unavailable, set: unavailable },
      actions: { execute: unavailable },
      options: { get: unavailable },
      events: { subscribe: function () { return function () {}; } },
      host: { locale: unavailable, theme: unavailable }
    };
    return;
  }
  window.__tiktools_transport__ = 'native';
  if (window.__tiktools_broker_dispose__) window.__tiktools_broker_dispose__();
  var sequence = 0;
  var instanceId = Date.now().toString(36) + '-' + Math.random().toString(36).slice(2);
  var pending = new Map();
  var listeners = new Map();
  var listenerCount = 0;
  var disposed = false;
  function dispose() {
    if (disposed) return;
    disposed = true;
    pending.forEach(function (entry) {
      clearTimeout(entry.timer);
      entry.reject(new Error('plugin host was closed'));
    });
    pending.clear();
    listeners.clear();
    listenerCount = 0;
    window.removeEventListener('pagehide', dispose);
    window.removeEventListener('beforeunload', dispose);
  }
  window.__tiktools_broker_dispose__ = dispose;
  window.__tiktools_broker_diagnostics__ = function () { return {pending:pending.size,topics:listeners.size,listeners:listenerCount}; };
  window.addEventListener('pagehide', dispose);
  window.addEventListener('beforeunload', dispose);
  function fire(topic, data) {
    var watchers = listeners.get(topic);
    if (!watchers) return;
    watchers.slice().forEach(function (listener) {
      try { listener(topic, data); } catch (listenerError) { /* listener bugs must not break the bridge */ }
    });
  }
  window.__tiktools_broker_push__ = function (envelope) {
    if (disposed || !envelope || envelope.apiVersion !== 1) return;
    if (envelope.method === 'event' && envelope.params && typeof envelope.params.topic === 'string') {
      fire(envelope.params.topic, envelope.params.data);
      return;
    }
    if (typeof envelope.event === 'string') {
      fire(envelope.event, envelope.data);
      return;
    }
    if (typeof envelope.id !== 'string') return;
    var entry = pending.get(envelope.id);
    if (!entry) return;
    pending.delete(envelope.id);
    clearTimeout(entry.timer);
    if (envelope.ok === true) entry.resolve(envelope.result);
    else entry.reject(new Error(typeof envelope.error === 'string' ? envelope.error : 'broker error'));
  };
  function call(method, params) {
    if (disposed) return Promise.reject(new Error('plugin host was closed'));
    if (pending.size >= 128) return Promise.reject(new Error('plugin broker request limit reached'));
    sequence += 1;
    var id = 'plg-' + instanceId + '-' + sequence;
    return new Promise(function (resolve, reject) {
      var timer = setTimeout(function () {
        pending.delete(id);
        reject(new Error('plugin broker request timed out'));
      }, 30000);
      pending.set(id, { resolve: resolve, reject: reject, timer: timer });
      try {
        ipc.postMessage(JSON.stringify({ apiVersion: 1, id: id, method: method, params: params || {} }));
      } catch (error) {
        pending.delete(id);
        clearTimeout(timer);
        reject(error);
      }
    });
  }
  window.tiktools = {
    settings: {
      get: function () { return call('settings.get', {}); },
      set: function (values) { return call('settings.set', { values: values || {} }); }
    },
    actions: {
      execute: function (action, config) { return call('actions.execute', { action: action, config: config || {} }); }
    },
    options: {
      get: function (source, refresh) { return call('options.get', { source: source, refresh: refresh === true }); }
    },
    events: {
      subscribe: function (topics, listener) {
        if (disposed) throw new Error('plugin host was closed');
        var names = Array.isArray(topics) ? topics : [topics];
        if (typeof listener !== 'function' || names.length < 1 || names.length > 32 || names.some(function (topic) { return typeof topic !== 'string' || topic.length > 128 || topic.indexOf('plugin.') !== 0; })) throw new Error('invalid plugin event subscription');
        var allTopics = new Set(Array.from(listeners.keys()).concat(names));
        if (allTopics.size > 32 || listenerCount + names.length > 128) throw new Error('plugin event listener limit reached');
        names.forEach(function (topic) {
          if (!listeners.has(topic)) listeners.set(topic, []);
          listeners.get(topic).push(listener);
          listenerCount += 1;
        });
        var removed = false;
        function unsubscribe() {
          if (removed) return;
          removed = true;
          names.forEach(function (topic) {
            var watchers = listeners.get(topic);
            if (!watchers) return;
            var index = watchers.indexOf(listener);
            if (index >= 0) { watchers.splice(index, 1); listenerCount -= 1; }
            if (watchers.length === 0) {
              listeners.delete(topic);
              if (!disposed) call('events.unsubscribe', { topics: [topic] }).catch(function () {});
            }
          });
        }
        call('events.subscribe', { topics: names }).catch(unsubscribe);
        return unsubscribe;
      }
    },
    host: {
      locale: function () { return call('host.locale', {}); },
      theme: function () { return call('host.theme', {}); }
    }
  };
})();