// Check if running in Tauri
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// Check if running in server mode (served by rustatio-server)
// We detect this by checking if /api/health endpoint responds
let isServerMode = false;
let serverBaseUrl = '';

let initialized = false;
let wasm = null;

// Log event listener registry for web version
let logListeners = [];

// =============================================================================
// Authentication Token Management
// =============================================================================

const AUTH_TOKEN_KEY = 'rustatio-auth-token';

/**
 * Get the stored authentication token
 * @returns {string|null} The stored token or null if not set
 */
export function getAuthToken() {
  return localStorage.getItem(AUTH_TOKEN_KEY);
}

/**
 * Set the authentication token
 * @param {string} token - The token to store
 */
export function setAuthToken(token) {
  if (token && token.trim()) {
    localStorage.setItem(AUTH_TOKEN_KEY, token.trim());
  } else {
    localStorage.removeItem(AUTH_TOKEN_KEY);
  }
}

/**
 * Clear the stored authentication token
 */
export function clearAuthToken() {
  localStorage.removeItem(AUTH_TOKEN_KEY);
}

/**
 * Check if authentication is enabled on the server
 * @returns {Promise<{authEnabled: boolean}>}
 */
export async function checkAuthStatus() {
  if (!isServerMode) {
    return { authEnabled: false };
  }

  try {
    const response = await fetch(`${serverBaseUrl}/api/auth/status`);
    const data = await response.json();
    return { authEnabled: data.data?.auth_enabled || false };
  } catch (error) {
    console.warn('Failed to check auth status:', error);
    return { authEnabled: false };
  }
}

/**
 * Verify the current authentication token
 * @returns {Promise<{valid: boolean, error?: string}>}
 */
export async function verifyAuthToken() {
  if (!isServerMode) {
    return { valid: true };
  }

  const token = getAuthToken();
  if (!token) {
    return { valid: false, error: 'No token set' };
  }

  try {
    const response = await fetch(`${serverBaseUrl}/api/auth/verify`, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });

    if (response.ok) {
      return { valid: true };
    }

    const data = await response.json();
    return { valid: false, error: data.error || 'Invalid token' };
  } catch (error) {
    return { valid: false, error: error.message };
  }
}

// =============================================================================
// Server Detection
// =============================================================================

// Detect server mode by checking the public /api/auth/status endpoint
async function detectServerMode() {
  try {
    const response = await fetch('/api/auth/status', { method: 'GET' });
    if (response.ok) {
      // Verify it's actually JSON (not Vite's HTML fallback)
      const contentType = response.headers.get('content-type');
      if (contentType && contentType.includes('application/json')) {
        isServerMode = true;
        serverBaseUrl = '';

        return true;
      }
    }
  } catch {
    // Server not available, will use WASM
  }
  return false;
}

// Only import WASM if not in Tauri
if (!isTauri) {
  try {
    const wasmModule = await import('$lib/wasm/rustatio_wasm.js');
    wasm = wasmModule;
  } catch {
    // WASM not available, will use server mode
  }
}

export async function initWasm() {
  if (isTauri) {
    // In Tauri, no WASM initialization needed
    initialized = true;
    return;
  }

  // Try server mode first
  const serverAvailable = await detectServerMode();
  if (serverAvailable) {
    initialized = true;
    return;
  }

  // Fall back to WASM
  if (!initialized && wasm) {
    const { default: init } = wasm;
    await init();
    wasm.init();

    // Set up log callback for WASM
    wasm.set_log_callback((level, message) => {
      emitLog(level, message);
    });

    initialized = true;
  }
}

// Proxy configuration helpers
export function getProxyUrl() {
  return localStorage.getItem('rustatio-proxy-url') || '';
}

export function setProxyUrl(url) {
  if (url && url.trim()) {
    localStorage.setItem('rustatio-proxy-url', url.trim());
  } else {
    localStorage.removeItem('rustatio-proxy-url');
  }
}

// Logging infrastructure
export async function listenToLogs(callback) {
  if (isTauri) {
    // For Tauri, use event listener
    try {
      const { listen } = await import('@tauri-apps/api/event');
      await listen('log-event', event => {
        callback(event.payload);
      });
    } catch (error) {
      console.error('Failed to set up log listener:', error);
    }
  } else if (isServerMode) {
    // For server mode, use both:
    // 1. SSE for backend logs from rustatio-core
    // 2. logListeners for frontend-generated logs
    logListeners.push(callback);

    try {
      // Include auth token as query parameter since EventSource doesn't support headers
      const token = getAuthToken();
      const authQuery = token ? `?token=${encodeURIComponent(token)}` : '';
      const eventSource = new EventSource(`${serverBaseUrl}/api/logs${authQuery}`);

      eventSource.addEventListener('log', event => {
        try {
          const logEvent = JSON.parse(event.data);
          callback(logEvent);
        } catch (e) {
          console.error('Failed to parse log event:', e);
        }
      });

      eventSource.onerror = error => {
        console.warn('SSE connection error, will retry:', error);
      };

      // Return cleanup function
      return () => eventSource.close();
    } catch (error) {
      console.error('Failed to set up SSE log listener:', error);
    }
  } else {
    // For WASM, register callback to receive web console logs
    logListeners.push(callback);
  }
}

// Instance events subscription (for real-time sync with watch folder)
// Only available in server mode
export function listenToInstanceEvents(callback) {
  if (!isServerMode) {
    // Not available in WASM or Tauri mode
    return () => {};
  }

  try {
    // Include auth token as query parameter since EventSource doesn't support headers
    const token = getAuthToken();
    const authQuery = token ? `?token=${encodeURIComponent(token)}` : '';
    const eventSource = new EventSource(`${serverBaseUrl}/api/events${authQuery}`);

    eventSource.addEventListener('instance', event => {
      try {
        const instanceEvent = JSON.parse(event.data);
        callback(instanceEvent);
      } catch (e) {
        console.error('Failed to parse instance event:', e);
      }
    });

    eventSource.onerror = error => {
      console.warn('Instance SSE connection error, will retry:', error);
    };

    // Return cleanup function
    return () => eventSource.close();
  } catch (error) {
    console.error('Failed to set up instance event listener:', error);
    return () => {};
  }
}

// Web-version logging wrapper (called when WASM logs to console)
export function emitLog(level, message) {
  if (!isTauri) {
    const logEvent = {
      timestamp: Date.now(),
      level,
      message,
    };

    // Notify all registered listeners
    logListeners.forEach(listener => listener(logEvent));
  }
}

// Server API helper with logging and authentication
async function serverFetch(endpoint, options = {}, logMessage = null) {
  const url = `${serverBaseUrl}/api${endpoint}`;
  const token = getAuthToken();

  // Build headers with optional auth
  const headers = {
    'Content-Type': 'application/json',
    ...options.headers,
  };

  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  try {
    const response = await fetch(url, {
      ...options,
      headers,
    });

    const data = await response.json();

    // Handle authentication errors
    if (response.status === 401 || response.status === 403) {
      const error = new Error(data.error || 'Authentication required');
      error.authRequired = true;
      error.statusCode = response.status;
      throw error;
    }

    if (!data.success) {
      emitLog('error', `API error: ${data.error || 'Unknown error'}`);
      throw new Error(data.error || 'Unknown error');
    }

    if (logMessage) {
      emitLog('info', logMessage);
    }

    return data.data;
  } catch (error) {
    if (error.message !== 'Unknown error') {
      emitLog('error', `Request failed: ${error.message}`);
    }
    throw error;
  }
}

// Server API implementation
const serverApi = {
  createInstance: async () => {
    const result = await serverFetch('/instances', { method: 'POST' }, 'Created new instance');
    return result.id; // Return string ID (nanoid)
  },
  deleteInstance: async (id, force = false) => {
    const query = force ? '?force=true' : '';
    await serverFetch(
      `/instances/${id}${query}`,
      { method: 'DELETE' },
      `[Instance ${id}] Instance deleted`
    );
  },
  listInstances: async () => {
    return serverFetch('/instances', { method: 'GET' });
  },
  loadTorrent: async file => {
    const formData = new FormData();
    formData.append('file', file);

    emitLog('info', `Loading torrent: ${file.name}`);

    const token = getAuthToken();
    const headers = {};
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }

    const response = await fetch(`${serverBaseUrl}/api/torrent/load`, {
      method: 'POST',
      body: formData,
      headers,
    });

    const data = await response.json();

    // Handle authentication errors
    if (response.status === 401 || response.status === 403) {
      const error = new Error(data.error || 'Authentication required');
      error.authRequired = true;
      error.statusCode = response.status;
      throw error;
    }

    if (!data.success) {
      emitLog('error', `Failed to load torrent: ${data.error}`);
      throw new Error(data.error || 'Failed to load torrent');
    }

    emitLog(
      'info',
      `Torrent loaded: ${data.data.torrent.name} (${formatBytes(data.data.torrent.total_size)})`
    );
    return data.data.torrent;
  },
  // Load torrent for a specific instance (creates idle instance on server)
  // This allows the instance to persist across page refreshes
  loadInstanceTorrent: async (id, file) => {
    const formData = new FormData();
    formData.append('file', file);

    emitLog('info', `[Instance ${id}] Loading torrent: ${file.name}`);

    const token = getAuthToken();
    const headers = {};
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }

    const response = await fetch(`${serverBaseUrl}/api/instances/${id}/torrent`, {
      method: 'POST',
      body: formData,
      headers,
    });

    const data = await response.json();

    // Handle authentication errors
    if (response.status === 401 || response.status === 403) {
      const error = new Error(data.error || 'Authentication required');
      error.authRequired = true;
      error.statusCode = response.status;
      throw error;
    }

    if (!data.success) {
      emitLog('error', `[Instance ${id}] Failed to load torrent: ${data.error}`);
      throw new Error(data.error || 'Failed to load torrent');
    }

    emitLog(
      'info',
      `[Instance ${id}] Torrent loaded: ${data.data.torrent.name} (${formatBytes(data.data.torrent.total_size)})`
    );
    return data.data.torrent;
  },
  startFaker: async (id, torrent, config) => {
    emitLog('info', `[Instance ${id}] Starting faker for ${torrent.name}`);
    await serverFetch(`/faker/${id}/start`, {
      method: 'POST',
      body: JSON.stringify({ torrent, config }),
    });
    emitLog(
      'info',
      `[Instance ${id}] Faker started - emulating ${config.client_type} v${config.client_version}`
    );
  },
  updateFaker: async id => {
    emitLog('debug', `[Instance ${id}] Sending tracker announce...`);
    const result = await serverFetch(`/faker/${id}/update`, { method: 'POST' });
    emitLog(
      'info',
      `[Instance ${id}] Tracker announce complete - Seeders: ${result.seeders}, Leechers: ${result.leechers}`
    );
    return result;
  },
  stopFaker: async id => {
    emitLog('info', `[Instance ${id}] Stopping faker...`);
    const result = await serverFetch(`/faker/${id}/stop`, { method: 'POST' });
    emitLog('info', `[Instance ${id}] Faker stopped`);
    return result;
  },
  pauseFaker: async id => {
    await serverFetch(`/faker/${id}/pause`, { method: 'POST' }, `[Instance ${id}] Faker paused`);
  },
  resumeFaker: async id => {
    await serverFetch(`/faker/${id}/resume`, { method: 'POST' }, `[Instance ${id}] Faker resumed`);
  },
  updateStatsOnly: async id => {
    return serverFetch(`/faker/${id}/stats-only`, { method: 'POST' });
  },
  getStats: async id => {
    return serverFetch(`/faker/${id}/stats`, { method: 'GET' });
  },
  scrapeTracker: async _id => {
    emitLog('warn', 'scrapeTracker not implemented in server mode');
    return null;
  },
  getClientTypes: async () => {
    return serverFetch('/clients', { method: 'GET' });
  },
  getClientInfos: async () => {
    return serverFetch('/clients/info', { method: 'GET' });
  },
  getNetworkStatus: async () => {
    return serverFetch('/network/status', { method: 'GET' });
  },
  getConfig: () => {
    const stored = localStorage.getItem('rustatio-config');
    return stored ? JSON.parse(stored) : null;
  },
  updateConfig: config => {
    localStorage.setItem('rustatio-config', JSON.stringify(config));
  },
  // Watch folder endpoints (server mode only)
  getWatchStatus: async () => {
    return serverFetch('/watch/status', { method: 'GET' });
  },
  listWatchFiles: async () => {
    return serverFetch('/watch/files', { method: 'GET' });
  },
  deleteWatchFile: async filename => {
    await serverFetch(`/watch/files/${encodeURIComponent(filename)}`, { method: 'DELETE' });
  },
  reloadWatchFile: async filename => {
    await serverFetch(`/watch/files/${encodeURIComponent(filename)}/reload`, { method: 'POST' });
  },
  reloadAllWatchFiles: async () => {
    return serverFetch('/watch/reload', { method: 'POST' });
  },
  // Default config for new instances (watch folder, etc.)
  getDefaultConfig: async () => {
    return serverFetch('/config/default', { method: 'GET' });
  },
  setDefaultConfig: async config => {
    await serverFetch('/config/default', {
      method: 'PUT',
      body: JSON.stringify(config),
    });
  },
  clearDefaultConfig: async () => {
    await serverFetch('/config/default', { method: 'DELETE' });
  },
  // Update instance config (without starting the faker)
  // Used to persist form changes before the user clicks Start
  updateInstanceConfig: async (id, config) => {
    await serverFetch(`/instances/${id}/config`, {
      method: 'PATCH',
      body: JSON.stringify(config),
    });
  },
};

// Helper to format bytes for log messages
function formatBytes(bytes) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

// Known VPN provider patterns for detection
const VPN_PROVIDERS = [
  ['proton', 'ProtonVPN'],
  ['mullvad', 'Mullvad'],
  ['nordvpn', 'NordVPN'],
  ['nord', 'NordVPN'],
  ['expressvpn', 'ExpressVPN'],
  ['express', 'ExpressVPN'],
  ['surfshark', 'Surfshark'],
  ['private internet access', 'Private Internet Access'],
  ['pia', 'Private Internet Access'],
  ['windscribe', 'Windscribe'],
  ['cyberghost', 'CyberGhost'],
  ['ipvanish', 'IPVanish'],
  ['tunnelbear', 'TunnelBear'],
  ['hotspot shield', 'Hotspot Shield'],
  ['vyprvpn', 'VyprVPN'],
  ['hide.me', 'Hide.me'],
  ['perfect privacy', 'Perfect Privacy'],
  ['airvpn', 'AirVPN'],
  ['privatevpn', 'PrivateVPN'],
  ['torguard', 'TorGuard'],
  ['ivpn', 'IVPN'],
  ['ovpn', 'OVPN'],
  ['m247', 'M247 (VPN Infrastructure)'],
  ['datacamp', 'Datacamp (VPN/Proxy)'],
  ['hostwinds', 'Hostwinds (VPN/VPS)'],
  ['choopa', 'Choopa/Vultr (VPN/VPS)'],
  ['linode', 'Linode (VPN/VPS)'],
  ['digitalocean', 'DigitalOcean (VPN/VPS)'],
];

// Detect VPN provider from organization string
function detectVpnProvider(org) {
  if (!org) return null;
  const orgLower = org.toLowerCase();
  for (const [pattern, provider] of VPN_PROVIDERS) {
    if (orgLower.includes(pattern)) {
      return provider;
    }
  }
  return null;
}

// Fetch network status with fallback providers
async function fetchNetworkStatusWithFallbacks() {
  // Try ip-api.com first (more reliable, HTTP only but CORS-friendly)
  try {
    const response = await fetch('http://ip-api.com/json', {
      method: 'GET',
    });
    if (response.ok) {
      const data = await response.json();
      if (data.query) {
        const org = data.isp || data.org;
        const vpnProvider = detectVpnProvider(org);
        return {
          ip: data.query,
          country: data.countryCode || data.country,
          city: data.city,
          org: org,
          is_vpn: !!vpnProvider,
          vpn_provider: vpnProvider,
          detection_method: 'heuristic',
        };
      }
    }
  } catch (e) {
    console.warn('ip-api.com failed, trying fallback:', e.message);
  }

  // Fallback to ipinfo.io
  try {
    const response = await fetch('https://ipinfo.io/json', {
      method: 'GET',
      headers: { Accept: 'application/json' },
    });
    if (response.ok) {
      const data = await response.json();
      const vpnProvider = detectVpnProvider(data.org);
      return {
        ip: data.ip,
        country: data.country,
        city: data.city,
        org: data.org,
        is_vpn: !!vpnProvider,
        vpn_provider: vpnProvider,
        detection_method: 'heuristic',
      };
    }
  } catch (e) {
    console.warn('ipinfo.io failed, trying fallback:', e.message);
  }

  // Last resort: ipify (just IP, no other info)
  try {
    const response = await fetch('https://api.ipify.org?format=json', {
      method: 'GET',
    });
    if (response.ok) {
      const data = await response.json();
      return {
        ip: data.ip,
        country: null,
        city: null,
        org: null,
        is_vpn: false,
        vpn_provider: null,
        detection_method: 'heuristic',
      };
    }
  } catch (e) {
    console.warn('ipify.org failed:', e.message);
  }

  throw new Error('All IP lookup services failed');
}

// Tauri API implementation
const tauriApi = {
  createInstance: async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('create_instance');
  },
  deleteInstance: async (id, force = false) => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('delete_instance', { instanceId: id, force });
  },
  listInstances: async () => {
    // Tauri doesn't persist state across restarts yet
    return [];
  },
  loadTorrent: async file => {
    const { invoke } = await import('@tauri-apps/api/core');
    // For Tauri, we need file path not file object
    // This will be called from TorrentSelector with file path
    return invoke('load_torrent', { path: file });
  },
  startFaker: async (id, torrent, config) => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('start_faker', {
      instanceId: id,
      torrent: torrent,
      config: config,
    });
  },
  updateFaker: async id => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('update_faker', { instanceId: id });
  },
  stopFaker: async id => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('stop_faker', { instanceId: id });
  },
  pauseFaker: async id => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('pause_faker', { instanceId: id });
  },
  resumeFaker: async id => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('resume_faker', { instanceId: id });
  },
  updateStatsOnly: async id => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('update_stats_only', { instanceId: id });
  },
  getStats: async id => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('get_stats', { instanceId: id });
  },
  scrapeTracker: async id => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('scrape_tracker', { instanceId: id });
  },
  getClientTypes: async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('get_client_types');
  },
  getClientInfos: async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('get_client_infos');
  },
  getNetworkStatus: async () => {
    // Use fallback function for desktop
    try {
      return await fetchNetworkStatusWithFallbacks();
    } catch (error) {
      throw new Error(`Failed to get network status: ${error.message}`);
    }
  },
  getConfig: async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('get_config');
  },
  updateConfig: async config => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('update_config', { config });
  },
  // Watch folder not available in Tauri
  getWatchStatus: async () => null,
  listWatchFiles: async () => [],
  deleteWatchFile: async () => {},
  reloadWatchFile: async () => {},
  reloadAllWatchFiles: async () => ({ reloaded: 0 }),
  // Default config (use localStorage for Tauri since there's no server)
  getDefaultConfig: async () => {
    try {
      const stored = localStorage.getItem('rustatio-default-preset');
      if (!stored) return null;
      const preset = JSON.parse(stored);
      return preset?.settings || null;
    } catch {
      return null;
    }
  },
  setDefaultConfig: async () => {
    // No-op for Tauri - localStorage is managed by defaultPreset.js
  },
  clearDefaultConfig: async () => {
    // No-op for Tauri - localStorage is managed by defaultPreset.js
  },
  // Config sync not needed in Tauri (state is in-memory)
  updateInstanceConfig: async () => {},
};

// WASM API implementation
const wasmApi = {
  createInstance: () => wasm.create_instance(),
  deleteInstance: (id, force = false) => wasm.delete_instance(id, force),
  listInstances: async () => {
    // WASM doesn't persist state
    return [];
  },
  loadTorrent: async file => {
    const bytes = new Uint8Array(await file.arrayBuffer());
    return wasm.load_torrent(bytes);
  },
  startFaker: async (id, torrent, config) => {
    return wasm.start_faker(id, torrent, config);
  },
  updateFaker: async id => {
    return wasm.update_faker(id);
  },
  stopFaker: async id => {
    return wasm.stop_faker(id);
  },
  pauseFaker: async id => {
    return wasm.pause_faker(id);
  },
  resumeFaker: async id => {
    return wasm.resume_faker(id);
  },
  updateStatsOnly: async id => {
    return wasm.update_stats_only(id);
  },
  getStats: async id => {
    return wasm.get_stats(id);
  },
  scrapeTracker: async id => {
    return wasm.scrape_tracker(id);
  },
  getClientTypes: () => {
    return wasm.get_client_types();
  },
  getClientInfos: () => {
    return wasm.get_client_infos();
  },
  getNetworkStatus: async () => {
    // Use fallback function for WASM
    // Note: Some services may have CORS issues on GitHub Pages
    try {
      return await fetchNetworkStatusWithFallbacks();
    } catch (error) {
      // CORS may block this on GitHub Pages - return null to indicate unavailable
      console.warn('Network status unavailable:', error.message);
      return null;
    }
  },
  getConfig: () => {
    const stored = localStorage.getItem('rustatio-config');
    return stored ? JSON.parse(stored) : null;
  },
  updateConfig: config => {
    localStorage.setItem('rustatio-config', JSON.stringify(config));
  },
  // Watch folder not available in WASM
  getWatchStatus: async () => null,
  listWatchFiles: async () => [],
  deleteWatchFile: async () => {},
  reloadWatchFile: async () => {},
  reloadAllWatchFiles: async () => ({ reloaded: 0 }),
  // Default config (use localStorage for WASM since there's no server)
  getDefaultConfig: async () => {
    try {
      const stored = localStorage.getItem('rustatio-default-preset');
      if (!stored) return null;
      const preset = JSON.parse(stored);
      return preset?.settings || null;
    } catch {
      return null;
    }
  },
  setDefaultConfig: async () => {
    // No-op for WASM - localStorage is managed by defaultPreset.js
  },
  clearDefaultConfig: async () => {
    // No-op for WASM - localStorage is managed by defaultPreset.js
  },
  // Config sync not needed in WASM (state is in-memory)
  updateInstanceConfig: async () => {},
};

// Dynamic API getter that returns the appropriate implementation
// based on the detected runtime environment
function getApi() {
  if (isTauri) {
    return tauriApi;
  }
  if (isServerMode) {
    return serverApi;
  }
  return wasmApi;
}

// Export a proxy that always uses the correct API
export const api = new Proxy(
  {},
  {
    get(_, prop) {
      const currentApi = getApi();
      return currentApi[prop];
    },
  }
);

// Export mode detection for UI
export function getRunMode() {
  if (isTauri) return 'desktop';
  if (isServerMode) return 'server';
  return 'wasm';
}
