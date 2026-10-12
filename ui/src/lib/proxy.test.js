import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  defaultPortFor,
  emptyProxyForm,
  errorText,
  formFromSettings,
  formatProxyUrl,
  parseProxyUrl,
  settingsFromForm,
  validateProxyForm,
} from './proxy.js';

test('empty form defaults to a disabled SOCKS5 proxy with hostname lookup', () => {
  assert.deepEqual(emptyProxyForm(), {
    enabled: false,
    type: 'socks5',
    host: '',
    port: '1080',
    resolveHostnameViaProxy: true,
    authentication: false,
    username: '',
    password: '',
  });
});

test('defaultPortFor falls back to 1080 for unknown types', () => {
  assert.equal(defaultPortFor('socks5'), 1080);
  assert.equal(defaultPortFor('unknown'), 1080);
});

test('formatProxyUrl maps types and hostname lookup to schemes', () => {
  const base = { ...emptyProxyForm(), host: 'proxy.local', port: '1080' };
  assert.equal(
    formatProxyUrl({ ...base, type: 'socks5', resolveHostnameViaProxy: true }),
    'socks5h://proxy.local:1080'
  );
  assert.equal(
    formatProxyUrl({ ...base, type: 'socks5', resolveHostnameViaProxy: false }),
    'socks5://proxy.local:1080'
  );
  assert.equal(
    formatProxyUrl({ ...base, type: 'socks4', resolveHostnameViaProxy: true }),
    'socks4a://proxy.local:1080'
  );
  assert.equal(
    formatProxyUrl({ ...base, type: 'socks4', resolveHostnameViaProxy: false }),
    'socks4://proxy.local:1080'
  );
});

test('parseProxyUrl maps schemes back to form fields', () => {
  assert.deepEqual(parseProxyUrl('socks5h://proxy.local:1080'), {
    type: 'socks5',
    host: 'proxy.local',
    port: '1080',
    resolveHostnameViaProxy: true,
  });
  assert.deepEqual(parseProxyUrl('socks5://proxy.local:1080'), {
    type: 'socks5',
    host: 'proxy.local',
    port: '1080',
    resolveHostnameViaProxy: false,
  });
  assert.deepEqual(parseProxyUrl('socks4a://proxy.local:1080'), {
    type: 'socks4',
    host: 'proxy.local',
    port: '1080',
    resolveHostnameViaProxy: true,
  });
  assert.deepEqual(parseProxyUrl('socks4://proxy.local:1080'), {
    type: 'socks4',
    host: 'proxy.local',
    port: '1080',
    resolveHostnameViaProxy: false,
  });
});

test('parseProxyUrl handles missing URL and missing port', () => {
  assert.deepEqual(parseProxyUrl(''), {});
  assert.deepEqual(parseProxyUrl(null), {});
  assert.deepEqual(parseProxyUrl('socks5://proxy.local'), {
    type: 'socks5',
    host: 'proxy.local',
    port: '1080',
    resolveHostnameViaProxy: false,
  });
});

test('parseProxyUrl keeps credentials embedded in the URL', () => {
  assert.deepEqual(parseProxyUrl('socks4a://user:pass@proxy.local:1080'), {
    type: 'socks4',
    host: 'proxy.local',
    port: '1080',
    resolveHostnameViaProxy: true,
    username: 'user',
    password: 'pass',
  });
  assert.deepEqual(parseProxyUrl('socks5h://us%65r:pa%40ss@proxy.local:1080'), {
    type: 'socks5',
    host: 'proxy.local',
    port: '1080',
    resolveHostnameViaProxy: true,
    username: 'user',
    password: 'pa@ss',
  });
});

test('formFromSettings falls back to credentials embedded in the URL', () => {
  const form = formFromSettings({
    enabled: true,
    url: 'socks5h://alice:secret@proxy.local:1080',
    username: null,
    password: null,
  });
  assert.equal(form.authentication, true);
  assert.equal(form.username, 'alice');
  assert.equal(form.password, 'secret');
  assert.equal(form.type, 'socks5');
  assert.equal(form.host, 'proxy.local');
  assert.equal(form.port, '1080');
});

test('formFromSettings prefers explicit credentials over URL credentials', () => {
  const form = formFromSettings({
    enabled: true,
    url: 'socks5h://urluser:urlpass@proxy.local:1080',
    username: 'user',
    password: 'pass',
  });
  assert.equal(form.username, 'user');
  assert.equal(form.password, 'pass');
});

test('formFromSettings reads enabled flag and credentials', () => {
  const form = formFromSettings({
    enabled: true,
    url: 'socks5h://proxy.local:1080',
    username: 'user',
    password: 'pass',
  });
  assert.equal(form.enabled, true);
  assert.equal(form.type, 'socks5');
  assert.equal(form.host, 'proxy.local');
  assert.equal(form.port, '1080');
  assert.equal(form.authentication, true);
  assert.equal(form.username, 'user');
  assert.equal(form.password, 'pass');
});

test('formFromSettings keeps authentication off without credentials', () => {
  const form = formFromSettings({ enabled: true, url: 'socks5h://proxy.local:1080' });
  assert.equal(form.authentication, false);
  assert.equal(form.username, '');
  assert.equal(form.password, '');
});

test('settingsFromForm drops credentials when authentication is disabled', () => {
  const form = {
    ...emptyProxyForm(),
    enabled: true,
    host: 'proxy.local',
    username: 'user',
    password: 'pass',
    authentication: false,
  };
  assert.deepEqual(settingsFromForm(form), {
    enabled: true,
    url: 'socks5h://proxy.local:1080',
    username: null,
    password: null,
  });
});

test('settingsFromForm keeps credentials when authentication is enabled', () => {
  const form = {
    ...emptyProxyForm(),
    enabled: true,
    host: 'proxy.local',
    authentication: true,
    username: 'user',
    password: 'pass',
  };
  assert.deepEqual(settingsFromForm(form), {
    enabled: true,
    url: 'socks5h://proxy.local:1080',
    username: 'user',
    password: 'pass',
  });
});

test('validateProxyForm skips validation when the proxy is disabled', () => {
  assert.equal(validateProxyForm(emptyProxyForm()), null);
});

test('validateProxyForm requires a host and a valid port', () => {
  const enabled = { ...emptyProxyForm(), enabled: true };
  assert.ok(validateProxyForm(enabled));
  assert.ok(validateProxyForm({ ...enabled, host: 'proxy.local', port: '0' }));
  assert.ok(validateProxyForm({ ...enabled, host: 'proxy.local', port: '70000' }));
  assert.equal(validateProxyForm({ ...enabled, host: 'proxy.local', port: '1080' }), null);
});

test('validateProxyForm requires a username when authentication is enabled', () => {
  const form = {
    ...emptyProxyForm(),
    enabled: true,
    host: 'proxy.local',
    port: '1080',
    authentication: true,
  };
  assert.ok(validateProxyForm(form));
  assert.equal(validateProxyForm({ ...form, username: 'user' }), null);
});

test('form and settings round trip preserves fields', () => {
  const form = {
    ...emptyProxyForm(),
    enabled: true,
    type: 'socks4',
    host: 'proxy.local',
    port: '9050',
    resolveHostnameViaProxy: false,
    authentication: true,
    username: 'user',
    password: 'pass',
  };
  assert.deepEqual(formFromSettings(settingsFromForm(form)), form);
});

test('errorText passes through string errors from Tauri', () => {
  assert.equal(
    errorText('Proxy request failed: connection refused'),
    'Proxy request failed: connection refused'
  );
});

test('errorText reads Error messages', () => {
  assert.equal(errorText(new Error('boom')), 'boom');
});

test('errorText falls back for empty or unknown errors', () => {
  assert.equal(errorText(''), 'Unknown error');
  assert.equal(errorText(null), 'Unknown error');
  assert.equal(errorText(undefined), 'Unknown error');
  assert.equal(errorText({}), 'Unknown error');
});
