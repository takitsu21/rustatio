// Proxy settings helpers shared by the settings dialog.
// Pure functions only so they can be unit tested with node:test.

export const PROXY_TYPES = [
  { id: 'socks5', label: 'SOCKS5', scheme: 'socks5', defaultPort: 1080 },
  { id: 'socks4', label: 'SOCKS4', scheme: 'socks4', defaultPort: 1080 },
];

export function defaultPortFor(type) {
  const found = PROXY_TYPES.find(entry => entry.id === type);
  return found ? found.defaultPort : 1080;
}

export function emptyProxyForm() {
  return {
    enabled: false,
    type: 'socks5',
    host: '',
    port: String(defaultPortFor('socks5')),
    resolveHostnameViaProxy: true,
    authentication: false,
    username: '',
    password: '',
  };
}

export function parseProxyUrl(url) {
  const raw = String(url ?? '').trim();
  if (!raw) return {};

  const lower = raw.toLowerCase();
  let type = 'socks5';
  let resolve = true;
  if (lower.startsWith('socks5h://')) {
    type = 'socks5';
  } else if (lower.startsWith('socks5://')) {
    type = 'socks5';
    resolve = false;
  } else if (lower.startsWith('socks4a://')) {
    type = 'socks4';
  } else if (lower.startsWith('socks4://')) {
    type = 'socks4';
    resolve = false;
  }

  let host = '';
  let port = '';
  let username = '';
  let password = '';
  if (URL.canParse(raw)) {
    const parsed = new URL(raw);
    host = parsed.hostname;
    port = parsed.port;
    username = decodeCredential(parsed.username);
    password = decodeCredential(parsed.password);
  }

  return {
    type,
    host,
    port: port || String(defaultPortFor(type)),
    resolveHostnameViaProxy: resolve,
    ...(username ? { username } : {}),
    ...(password ? { password } : {}),
  };
}

// URL keeps credentials percent-encoded; fall back to the raw value when the
// encoding is malformed so the form never throws.
function decodeCredential(value) {
  if (!value) return '';
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

export function formFromSettings(settings) {
  const source = settings || {};
  const parsed = parseProxyUrl(source.url);
  const form = emptyProxyForm();
  form.enabled = Boolean(source.enabled);
  form.username = String(source.username ?? parsed.username ?? '');
  form.password = String(source.password ?? parsed.password ?? '');
  form.authentication = Boolean(form.username || form.password);
  return { ...form, ...parsed, username: form.username, password: form.password };
}

export function formatProxyUrl(form) {
  const type = PROXY_TYPES.find(entry => entry.id === form.type) ?? PROXY_TYPES[0];
  let scheme = type.scheme;
  if (form.resolveHostnameViaProxy) {
    if (type.id === 'socks5') scheme = 'socks5h';
    if (type.id === 'socks4') scheme = 'socks4a';
  }

  const host = String(form.host ?? '').trim();
  const port = String(form.port ?? '').trim();
  return port ? `${scheme}://${host}:${port}` : `${scheme}://${host}`;
}

export function validateProxyForm(form) {
  if (!form.enabled) return null;
  if (!String(form.host ?? '').trim()) {
    return 'Host is required when the proxy is enabled';
  }

  const port = Number(form.port);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    return 'Port must be a number between 1 and 65535';
  }

  if (form.authentication && !String(form.username ?? '').trim()) {
    return 'Username is required when authentication is enabled';
  }

  return null;
}

export function settingsFromForm(form) {
  return {
    enabled: Boolean(form.enabled),
    url: formatProxyUrl(form),
    username: form.authentication ? String(form.username ?? '') || null : null,
    password: form.authentication ? String(form.password ?? '') || null : null,
  };
}

// Tauri rejects commands with plain string errors, while server/browser code
// throws Error objects. Normalize both so dialogs never show "undefined".
export function errorText(error) {
  if (typeof error === 'string') return error || 'Unknown error';
  return error?.message || 'Unknown error';
}
