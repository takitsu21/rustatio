// Shared Server-Sent Events connection for server mode.
//
// Browsers limit concurrent HTTP/1.1 connections per origin (typically six).
// Opening a separate stream for logs and every instance-event subscriber made
// two tabs enough to exhaust that pool, blocking refreshes and API requests.
// Subscribers now share one multiplexed connection, which is closed as soon as
// the last subscriber unsubscribes.

function parseEvent(event, label) {
  try {
    return JSON.parse(event.data);
  } catch (error) {
    console.error(`Failed to parse ${label}:`, error);
    return null;
  }
}

export function createEventStream(url, Source = globalThis.EventSource) {
  const handlers = {
    log: new Set(),
    instance: new Set(),
  };
  let source = null;

  function dispatch(kind, event, label) {
    const parsed = parseEvent(event, label);
    if (parsed === null) return;

    for (const handler of handlers[kind]) {
      handler(parsed);
    }
  }

  function connect() {
    if (source) return;

    if (typeof Source !== 'function') {
      console.error('EventSource is not available in this environment');
      return;
    }

    try {
      source = new Source(url);
    } catch (error) {
      console.error('Failed to open SSE connection:', error);
      return;
    }

    source.addEventListener('log', event => dispatch('log', event, 'log event'));

    const handleInstance = event => dispatch('instance', event, 'instance event');
    source.addEventListener('instance', handleInstance);
    source.addEventListener('summaries', handleInstance);

    source.onerror = error => {
      console.warn('SSE connection error, will retry:', error);
    };
  }

  function unsubscribe(kind, handler) {
    if (!handlers[kind].delete(handler)) return;

    if (handlers.log.size === 0 && handlers.instance.size === 0 && source) {
      source.close();
      source = null;
    }
  }

  function subscribe(kind, handler) {
    handlers[kind].add(handler);
    connect();
    return () => unsubscribe(kind, handler);
  }

  return {
    subscribeLogs: handler => subscribe('log', handler),
    subscribeInstances: handler => subscribe('instance', handler),
    close() {
      handlers.log.clear();
      handlers.instance.clear();
      if (source) {
        source.close();
        source = null;
      }
    },
  };
}
