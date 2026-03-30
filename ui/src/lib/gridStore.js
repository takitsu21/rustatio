import { writable, derived, get } from 'svelte/store';
import { api, getRunMode } from '$lib/api';
import { normalizeViewMode } from '$lib/viewMode.js';
import { instanceActions } from '$lib/instanceStore.js';
import {
  applyAllGridFilters,
  applyBaseGridFilters,
  applySearchFilter,
  applyStateFilter,
  buildStateFilterEntries,
  buildTagFilterEntries,
  buildTrackerFilterEntries,
} from '$lib/gridFilters.js';

const VIEW_MODE_KEY = 'rustatio-view-mode';

function loadViewMode() {
  try {
    return normalizeViewMode(localStorage.getItem(VIEW_MODE_KEY));
  } catch {
    return 'standard';
  }
}

export const viewMode = writable(loadViewMode());

viewMode.subscribe(mode => {
  try {
    localStorage.setItem(VIEW_MODE_KEY, normalizeViewMode(mode));
  } catch {
    // localStorage unavailable
  }
});

// All instance summaries from the server
export const gridInstances = writable([]);

// Selected instance IDs
export const selectedIds = writable(new Set());

// Filter/search state
export const gridFilters = writable({
  search: '',
  stateFilter: 'all',
  tagFilter: [],
  trackerFilter: [],
  tagSearch: '',
  trackerSearch: '',
});

export const stateFilterEntries = derived(
  [gridInstances, gridFilters],
  ([$instances, $filters]) => {
    const scoped = applyBaseGridFilters(applySearchFilter($instances, $filters), {
      ...$filters,
      stateFilter: 'all',
    });
    return buildStateFilterEntries(scoped);
  }
);

export const tagFilterEntries = derived([gridInstances, gridFilters], ([$instances, $filters]) => {
  const scoped = applyStateFilter(applySearchFilter($instances, $filters), $filters);
  return buildTagFilterEntries(scoped, $filters);
});

export const trackerFilterEntries = derived(
  [gridInstances, gridFilters],
  ([$instances, $filters]) => {
    const scoped = applyBaseGridFilters($instances, { ...$filters, trackerFilter: [] });
    return buildTrackerFilterEntries(scoped, $filters);
  }
);

// Sorting state
export const gridSort = writable({
  column: 'name',
  direction: 'asc',
});

// Filtered + sorted instances (derived)
export const filteredGridInstances = derived(
  [gridInstances, gridFilters, gridSort],
  ([$instances, $filters, $sort]) => {
    let result = applyAllGridFilters($instances, $filters);

    // Sort
    result = [...result].sort((a, b) => {
      const sortKey = $sort.column === 'progress' ? 'torrentCompletion' : $sort.column;
      let aVal = a[sortKey];
      let bVal = b[sortKey];

      if (typeof aVal === 'string') {
        aVal = aVal.toLowerCase();
        bVal = (bVal || '').toLowerCase();
      }

      if (aVal < bVal) return $sort.direction === 'asc' ? -1 : 1;
      if (aVal > bVal) return $sort.direction === 'asc' ? 1 : -1;
      return 0;
    });

    return result;
  }
);

// Polling interval reference
let pollInterval = null;
let isFetching = false;

// Track IDs with in-flight background operations (start/stop).
// Maps id -> { optimistic: 'starting'|'stopping', target: 'running'|'stopped', ts: Date.now() }
// fetchSummaries preserves the optimistic state until the backend catches up or TTL expires.
const PENDING_TTL_MS = 30000;
const pendingStateIds = new Map();

// After grid import, fetch actual backend configs so the standard view shows
// the real per-instance rates (randomized from range) instead of default preset values.
async function syncImportedInstances(imported, importConfig = {}) {
  const importedIds = new Set(imported.map(inst => String(inst.id)));
  try {
    const serverInstances = await api.listInstances();
    if (serverInstances?.length > 0) {
      for (const serverInst of serverInstances) {
        if (importedIds.has(String(serverInst.id))) {
          instanceActions.mergeServerInstance(serverInst);
          importedIds.delete(String(serverInst.id));
        }
      }
    }
  } catch {
    // listInstances may not return config (Tauri/WASM)
  }
  // Fallback for instances not found in listInstances — use the import config
  // so the standard view reflects the actual rates/client/settings from the import dialog
  const defaults = importConfig.baseConfig || {};
  if (importConfig.autoStart) defaults.autoStart = true;
  for (const id of importedIds) {
    const inst = imported.find(i => String(i.id) === id);
    await instanceActions.addInstanceToStore(id, inst?.name || '', defaults);
  }
}

export const gridActions = {
  fetchSummaries: async () => {
    if (isFetching) return;
    isFetching = true;
    try {
      // On desktop/WASM, stats only advance when update() or updateStatsOnly() is called.
      // The server has a background scheduler, but desktop/WASM rely on the frontend.
      // Advance stats for all running instances before reading the snapshot.
      if (getRunMode() !== 'server') {
        const current = get(gridInstances);
        const running = current.filter(i => i.state === 'running');
        if (running.length > 0) {
          await Promise.allSettled(running.map(i => api.updateStatsOnly(i.id)));
        }
      }

      const summaries = await api.listSummaries();
      if (!summaries) return;

      // Reconcile pending optimistic states with backend reality
      const now = Date.now();
      const reconciled = summaries.map(s => {
        const pending = pendingStateIds.get(s.id);
        if (!pending) return s;

        const backendState = s.state?.toLowerCase();

        // Backend caught up to target state — clear pending
        if (backendState === pending.target) {
          pendingStateIds.delete(s.id);
          return s;
        }

        // TTL expired — clear pending, use backend state
        if (now - pending.ts > PENDING_TTL_MS) {
          pendingStateIds.delete(s.id);
          return s;
        }

        // Still pending — preserve optimistic state
        return { ...s, state: pending.optimistic };
      });

      gridInstances.set(reconciled);
    } catch (error) {
      console.error('Failed to fetch summaries:', error);
    } finally {
      isFetching = false;
    }
  },

  startPolling: (intervalMs = 1000) => {
    gridActions.stopPolling();
    gridActions.fetchSummaries();
    pollInterval = setInterval(() => gridActions.fetchSummaries(), intervalMs);
  },

  stopPolling: () => {
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  },

  selectAll: () => {
    const all = get(filteredGridInstances);
    selectedIds.set(new Set(all.map(i => i.id)));
  },

  deselectAll: () => {
    selectedIds.set(new Set());
  },

  getSelectedIds: () => {
    return [...get(selectedIds)];
  },

  toggleSort: column => {
    gridSort.update(s => {
      if (s.column === column) {
        return { column, direction: s.direction === 'asc' ? 'desc' : 'asc' };
      }
      return { column, direction: 'asc' };
    });
  },

  gridStart: async () => {
    const ids = gridActions.getSelectedIds();
    if (ids.length === 0) return;
    const instances = get(gridInstances);
    const startable = ids.filter(id => {
      const inst = instances.find(i => i.id === id);
      return inst?.state?.toLowerCase() === 'stopped';
    });
    if (startable.length === 0) return;
    const idSet = new Set(startable);
    const now = Date.now();
    for (const id of startable) {
      pendingStateIds.set(id, { optimistic: 'starting', target: 'running', ts: now });
    }
    gridInstances.update(list =>
      list.map(i => (idSet.has(i.id) ? { ...i, state: 'starting' } : i))
    );
    const result = await api.gridStart(startable);
    await gridActions.fetchSummaries();
    for (const id of startable) {
      instanceActions.syncInstanceState(id, { isRunning: true, isPaused: false });
    }
    return result;
  },

  gridStop: async () => {
    const ids = gridActions.getSelectedIds();
    if (ids.length === 0) return;
    const instances = get(gridInstances);
    const stoppable = ids.filter(id => {
      const inst = instances.find(i => i.id === id);
      const s = inst?.state?.toLowerCase();
      return s === 'running' || s === 'idle' || s === 'paused' || s === 'starting';
    });
    if (stoppable.length === 0) return;
    const idSet = new Set(stoppable);
    const now = Date.now();
    for (const id of stoppable) {
      pendingStateIds.set(id, { optimistic: 'stopping', target: 'stopped', ts: now });
    }
    gridInstances.update(list =>
      list.map(i => (idSet.has(i.id) ? { ...i, state: 'stopping' } : i))
    );
    const result = await api.gridStop(stoppable);
    await gridActions.fetchSummaries();
    for (const id of stoppable) {
      instanceActions.syncInstanceState(id, { isRunning: false, isPaused: false });
    }
    return result;
  },

  gridPause: async () => {
    const ids = gridActions.getSelectedIds();
    if (ids.length === 0) return;
    const instances = get(gridInstances);
    const pauseable = ids.filter(id => {
      const inst = instances.find(i => i.id === id);
      const s = inst?.state?.toLowerCase();
      return s === 'running' || s === 'idle';
    });
    if (pauseable.length === 0) return;
    const result = await api.gridPause(pauseable);
    await gridActions.fetchSummaries();
    for (const id of pauseable) {
      instanceActions.syncInstanceState(id, { isRunning: true, isPaused: true });
    }
    return result;
  },

  gridResume: async () => {
    const ids = gridActions.getSelectedIds();
    if (ids.length === 0) return;
    const instances = get(gridInstances);
    const resumable = ids.filter(id => {
      const inst = instances.find(i => i.id === id);
      return inst?.state?.toLowerCase() === 'paused';
    });
    if (resumable.length === 0) return;
    const result = await api.gridResume(resumable);
    await gridActions.fetchSummaries();
    for (const id of resumable) {
      instanceActions.syncInstanceState(id, { isRunning: true, isPaused: false });
    }
    return result;
  },

  gridDelete: async () => {
    const ids = gridActions.getSelectedIds();
    if (ids.length === 0) return;
    const result = await api.gridDelete(ids);
    selectedIds.update(s => {
      const next = new Set(s);
      for (const id of ids) next.delete(id);
      return next;
    });
    for (const id of ids) {
      instanceActions.removeInstanceFromStore(id);
    }
    await gridActions.fetchSummaries();
    return result;
  },

  gridTag: async (addTags, removeTags) => {
    const ids = gridActions.getSelectedIds();
    if (ids.length === 0) return;
    const result = await api.gridTag(ids, addTags, removeTags);
    await gridActions.fetchSummaries();
    return result;
  },

  import: async (files, config = {}) => {
    const result = await api.gridImport(files, config);
    if (result?.imported?.length > 0) {
      await syncImportedInstances(result.imported, config);
    }
    await gridActions.fetchSummaries();
    return result;
  },

  importFolder: async (path, config = {}) => {
    const result = await api.gridImportFolder(path, config);
    if (result?.imported?.length > 0) {
      await syncImportedInstances(result.imported, config);
    }
    await gridActions.fetchSummaries();
    return result;
  },

  // Single-instance actions (used by context menu)
  startInstance: async id => {
    pendingStateIds.set(id, { optimistic: 'starting', target: 'running', ts: Date.now() });
    gridInstances.update(instances =>
      instances.map(i => (i.id === id ? { ...i, state: 'starting' } : i))
    );
    const result = await api.gridStart([id]);
    await gridActions.fetchSummaries();
    instanceActions.syncInstanceState(id, { isRunning: true, isPaused: false });
    return result;
  },

  stopInstance: async id => {
    pendingStateIds.set(id, { optimistic: 'stopping', target: 'stopped', ts: Date.now() });
    gridInstances.update(instances =>
      instances.map(i => (i.id === id ? { ...i, state: 'stopping' } : i))
    );
    const result = await api.gridStop([id]);
    await gridActions.fetchSummaries();
    instanceActions.syncInstanceState(id, { isRunning: false, isPaused: false });
    return result;
  },

  pauseInstance: async id => {
    const result = await api.gridPause([id]);
    await gridActions.fetchSummaries();
    instanceActions.syncInstanceState(id, { isRunning: true, isPaused: true });
    return result;
  },

  resumeInstance: async id => {
    const result = await api.gridResume([id]);
    await gridActions.fetchSummaries();
    instanceActions.syncInstanceState(id, { isRunning: true, isPaused: false });
    return result;
  },

  deleteInstance: async id => {
    const result = await api.gridDelete([id]);
    selectedIds.update(s => {
      const next = new Set(s);
      next.delete(id);
      return next;
    });
    instanceActions.removeInstanceFromStore(id);
    await gridActions.fetchSummaries();
    return result;
  },

  selectByState: state => {
    const all = get(filteredGridInstances);
    const matching = all.filter(i => i.state?.toLowerCase() === state);
    selectedIds.set(new Set(matching.map(i => i.id)));
  },

  selectByTag: tag => {
    const all = get(filteredGridInstances);
    const matching = all.filter(i => i.tags?.includes(tag));
    selectedIds.set(new Set(matching.map(i => i.id)));
  },

  invertSelection: () => {
    const all = get(filteredGridInstances);
    const current = get(selectedIds);
    const inverted = new Set();
    for (const inst of all) {
      if (!current.has(inst.id)) inverted.add(inst.id);
    }
    selectedIds.set(inverted);
  },
};
