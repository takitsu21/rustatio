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

// Detect server mode
async function detectServerMode() {
  try {
    const response = await fetch('/api/clients', { method: 'GET' });
    if (response.ok) {
      // Verify it's actually JSON (not Vite's HTML fallback)
      const contentType = response.headers.get('content-type');
      if (contentType && contentType.includes('application/json')) {
        isServerMode = true;
        serverBaseUrl = '';
        console.log('Running in server mode');
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
    console.log('WASM not available, will try server mode');
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
      const eventSource = new EventSource(`${serverBaseUrl}/api/logs`);

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

// Server API helper with logging
async function serverFetch(endpoint, options = {}, logMessage = null) {
  const url = `${serverBaseUrl}/api${endpoint}`;

  try {
    const response = await fetch(url, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
    });

    const data = await response.json();

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
    return parseInt(result.id, 10);
  },
  deleteInstance: async id => {
    await serverFetch(`/instances/${id}`, { method: 'DELETE' }, `Deleted instance ${id}`);
  },
  listInstances: async () => {
    return serverFetch('/instances', { method: 'GET' });
  },
  loadTorrent: async file => {
    const formData = new FormData();
    formData.append('file', file);

    emitLog('info', `Loading torrent: ${file.name}`);

    const response = await fetch(`${serverBaseUrl}/api/torrent/load`, {
      method: 'POST',
      body: formData,
    });

    const data = await response.json();
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
  startFaker: async (id, torrent, config) => {
    emitLog('info', `Starting faker for ${torrent.name}`);
    await serverFetch(`/faker/${id}/start`, {
      method: 'POST',
      body: JSON.stringify({ torrent, config }),
    });
    emitLog('info', `Faker started - emulating ${config.client_type} v${config.client_version}`);
  },
  updateFaker: async id => {
    emitLog('debug', 'Sending tracker announce...');
    const result = await serverFetch(`/faker/${id}/update`, { method: 'POST' });
    emitLog(
      'info',
      `Tracker announce complete - Seeders: ${result.seeders}, Leechers: ${result.leechers}`
    );
    return result;
  },
  stopFaker: async id => {
    emitLog('info', 'Stopping faker...');
    const result = await serverFetch(`/faker/${id}/stop`, { method: 'POST' });
    emitLog('info', 'Faker stopped');
    return result;
  },
  pauseFaker: async id => {
    await serverFetch(`/faker/${id}/pause`, { method: 'POST' }, 'Faker paused');
  },
  resumeFaker: async id => {
    await serverFetch(`/faker/${id}/resume`, { method: 'POST' }, 'Faker resumed');
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

// Tauri API implementation
const tauriApi = {
  createInstance: async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('create_instance');
  },
  deleteInstance: async id => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('delete_instance', { instanceId: id });
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
    return ['utorrent', 'qbittorrent', 'transmission', 'deluge'];
  },
  getNetworkStatus: async () => {
    // For desktop, fetch directly from ipinfo.io
    try {
      const response = await fetch('https://ipinfo.io/json', {
        method: 'GET',
        headers: { Accept: 'application/json' },
      });
      if (!response.ok) throw new Error('Failed to fetch');
      const data = await response.json();
      const vpnProvider = detectVpnProvider(data.org);
      return {
        ip: data.ip,
        country: data.country,
        city: data.city,
        org: data.org,
        is_vpn: !!vpnProvider,
        vpn_provider: vpnProvider,
      };
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
};

// WASM API implementation
const wasmApi = {
  createInstance: () => wasm.create_instance(),
  deleteInstance: id => wasm.delete_instance(id),
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
  getNetworkStatus: async () => {
    // For WASM, fetch directly from ipinfo.io
    // Note: May have CORS issues on GitHub Pages
    try {
      const response = await fetch('https://ipinfo.io/json', {
        method: 'GET',
        headers: { Accept: 'application/json' },
      });
      if (!response.ok) throw new Error('Failed to fetch');
      const data = await response.json();
      const vpnProvider = detectVpnProvider(data.org);
      return {
        ip: data.ip,
        country: data.country,
        city: data.city,
        org: data.org,
        is_vpn: !!vpnProvider,
        vpn_provider: vpnProvider,
      };
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
