import assert from 'node:assert/strict';
import test from 'node:test';

import { createEventStream } from './serverStream.js';

function makeSourceClass() {
  return class FakeEventSource {
    static instances = [];

    constructor(url) {
      this.url = url;
      this.closed = false;
      this.listeners = new Map();
      FakeEventSource.instances.push(this);
    }

    addEventListener(name, handler) {
      const handlers = this.listeners.get(name) || [];
      handlers.push(handler);
      this.listeners.set(name, handlers);
    }

    emit(name, payload) {
      for (const handler of this.listeners.get(name) || []) {
        handler({ data: JSON.stringify(payload) });
      }
    }

    close() {
      this.closed = true;
    }
  };
}

test('shares one connection between log and instance subscribers', () => {
  const FakeEventSource = makeSourceClass();
  const stream = createEventStream('/api/stream', FakeEventSource);

  const logs = [];
  const instances = [];
  const offLogs = stream.subscribeLogs(event => logs.push(event));
  const offFirst = stream.subscribeInstances(event => instances.push(`first:${event.type}`));
  const offSecond = stream.subscribeInstances(event => instances.push(`second:${event.type}`));

  assert.equal(FakeEventSource.instances.length, 1);
  assert.equal(FakeEventSource.instances[0].url, '/api/stream');

  const source = FakeEventSource.instances[0];
  source.emit('log', { level: 'info', message: 'hello' });
  source.emit('instance', { type: 'deleted', id: '1' });
  source.emit('summaries', { type: 'summaries', instances: [] });

  assert.deepEqual(logs, [{ level: 'info', message: 'hello' }]);
  assert.deepEqual(instances, [
    'first:deleted',
    'second:deleted',
    'first:summaries',
    'second:summaries',
  ]);

  offLogs();
  offFirst();
  offSecond();

  assert.equal(source.closed, true);
});

test('reopens the connection only when a new subscriber arrives', () => {
  const FakeEventSource = makeSourceClass();
  const stream = createEventStream('/api/stream', FakeEventSource);

  const offLogs = stream.subscribeLogs(() => {});
  const offInstance = stream.subscribeInstances(() => {});
  const source = FakeEventSource.instances[0];

  offInstance();
  assert.equal(source.closed, false);

  offLogs();
  assert.equal(source.closed, true);

  const offAgain = stream.subscribeInstances(() => {});
  assert.equal(FakeEventSource.instances.length, 2);

  const next = FakeEventSource.instances[1];
  assert.equal(next.url, '/api/stream');

  offAgain();
  offAgain();
  assert.equal(next.closed, true);
});
