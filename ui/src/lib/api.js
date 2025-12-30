// Check if running in Tauri
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

let initialized = false;
let wasm = null;

// Log event listener registry for web version
let logListeners = [];

// Only import WASM if not in Tauri
if (!isTauri) {
  const wasmModule = await import('$lib/wasm/rustatio_wasm.js');
  wasm = wasmModule;
}

export async function initWasm() {
  if (isTauri) {
    // In Tauri, no WASM initialization needed
    initialized = true;
    return;
  }
  
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
      await listen('log-event', (event) => {
        callback(event.payload);
      });
    } catch (error) {
      console.error('Failed to set up log listener:', error);
    }
  } else {
    // For web, register callback to receive web console logs
    logListeners.push(callback);
  }
}

// Web-version logging wrapper (called when WASM logs to console)
export function emitLog(level, message) {
  if (!isTauri) {
    const logEvent = {
      timestamp: Date.now(),
      level,
      message
    };
    
    // Notify all registered listeners
    logListeners.forEach(listener => listener(logEvent));
  }
}

// Dynamic API based on environment
export const api = isTauri ? {
  // Tauri API - uses invoke
  createInstance: async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('create_instance');
  },
  deleteInstance: async (id) => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('delete_instance', { instanceId: id });
  },
  loadTorrent: async (file) => {
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
      config: config 
    });
  },
  updateFaker: async (id) => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('update_faker', { instanceId: id });
  },
  stopFaker: async (id) => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('stop_faker', { instanceId: id });
  },
  pauseFaker: async (id) => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('pause_faker', { instanceId: id });
  },
  resumeFaker: async (id) => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('resume_faker', { instanceId: id });
  },
  updateStatsOnly: async (id) => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('update_stats_only', { instanceId: id });
  },
  getStats: async (id) => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('get_stats', { instanceId: id });
  },
  scrapeTracker: async (id) => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('scrape_tracker', { instanceId: id });
  },
  getClientTypes: async () => {
    return ['utorrent', 'qbittorrent', 'transmission', 'deluge'];
  },
  getConfig: async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('get_config');
  },
  updateConfig: async (config) => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('update_config', { config });
  }
} : {
  // WASM API
  createInstance: () => wasm.create_instance(),
  deleteInstance: (id) => wasm.delete_instance(id),
  loadTorrent: async (file) => {
    const bytes = new Uint8Array(await file.arrayBuffer());
    return wasm.load_torrent(bytes);
  },
  startFaker: async (id, torrent, config) => {
    return wasm.start_faker(id, torrent, config);
  },
  updateFaker: async (id) => {
    return wasm.update_faker(id);
  },
  stopFaker: async (id) => {
    return wasm.stop_faker(id);
  },
  pauseFaker: async (id) => {
    return wasm.pause_faker(id);
  },
  resumeFaker: async (id) => {
    return wasm.resume_faker(id);
  },
  updateStatsOnly: async (id) => {
    return wasm.update_stats_only(id);
  },
  getStats: async (id) => {
    return wasm.get_stats(id);
  },
  scrapeTracker: async (id) => {
    return wasm.scrape_tracker(id);
  },
  getClientTypes: () => {
    return wasm.get_client_types();
  },
  getConfig: () => {
    const stored = localStorage.getItem('rustatio-config');
    return stored ? JSON.parse(stored) : null;
  },
  updateConfig: (config) => {
    localStorage.setItem('rustatio-config', JSON.stringify(config));
  }
};
