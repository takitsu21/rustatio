import { buildTrackerFilterEntries, getPrimaryTrackerHost } from './trackerUtils.js';

export const UNTAGGED_FILTER_VALUE = '__untagged__';

function hasAny(items) {
  return Array.isArray(items) && items.length > 0;
}

export function applySearchFilter(instances, filters) {
  if (!filters.search) {
    return instances;
  }

  const search = filters.search.toLowerCase();
  return instances.filter(
    inst =>
      inst.name.toLowerCase().includes(search) ||
      inst.infoHash?.toLowerCase().includes(search) ||
      inst.tags?.some(t => t.toLowerCase().includes(search))
  );
}

export function applyStateFilter(instances, filters) {
  if (filters.stateFilter === 'all') {
    return instances;
  }

  return instances.filter(inst => inst.state.toLowerCase() === filters.stateFilter);
}

export function applyTagFilter(instances, filters) {
  if (!hasAny(filters.tagFilter)) {
    return instances;
  }

  const selected = new Set(filters.tagFilter);
  return instances.filter(inst => {
    const tags = inst.tags || [];
    if (tags.length === 0) {
      return selected.has(UNTAGGED_FILTER_VALUE);
    }

    return tags.some(tag => selected.has(tag));
  });
}

export function applyTrackerFilter(instances, filters) {
  if (!hasAny(filters.trackerFilter)) {
    return instances;
  }

  const selected = new Set(filters.trackerFilter);
  return instances.filter(inst => selected.has(getPrimaryTrackerHost(inst)));
}

export function applyBaseGridFilters(instances, filters) {
  return applyTagFilter(applyStateFilter(applySearchFilter(instances, filters), filters), filters);
}

export function applyAllGridFilters(instances, filters) {
  return applyTrackerFilter(applyBaseGridFilters(instances, filters), filters);
}

export function buildStateFilterEntries(instances) {
  const counts = new Map();

  for (const instance of instances || []) {
    const state = String(instance.state || 'stopped').toLowerCase();
    counts.set(state, (counts.get(state) || 0) + 1);
  }

  const order = ['running', 'paused', 'idle', 'starting', 'stopping', 'stopped'];

  return order
    .filter(state => counts.has(state))
    .map(state => ({ value: state, count: counts.get(state) || 0 }));
}

export function buildTagFilterEntries(instances, filters = {}) {
  const counts = new Map();
  const query = String(filters.tagSearch || '')
    .toLowerCase()
    .trim();
  let untaggedCount = 0;

  for (const instance of instances || []) {
    const tags = instance.tags || [];

    if (tags.length === 0) {
      untaggedCount += 1;
      continue;
    }

    for (const tag of tags) {
      if (query && !tag.toLowerCase().includes(query)) {
        continue;
      }
      counts.set(tag, (counts.get(tag) || 0) + 1);
    }
  }

  const entries = [...counts.entries()]
    .map(([value, count]) => ({ value, label: value, count }))
    .sort((left, right) => {
      if (right.count !== left.count) {
        return right.count - left.count;
      }
      return left.label.localeCompare(right.label);
    });

  if (untaggedCount > 0 && (!query || 'untagged'.includes(query))) {
    entries.unshift({
      value: UNTAGGED_FILTER_VALUE,
      label: 'Untagged',
      count: untaggedCount,
      muted: true,
    });
  }

  return entries;
}

export function clearAllGridFilters(filters) {
  return {
    ...filters,
    stateFilter: 'all',
    tagFilter: [],
    trackerFilter: [],
    tagSearch: '',
    trackerSearch: '',
  };
}

export { buildTrackerFilterEntries };
