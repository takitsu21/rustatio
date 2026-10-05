export function normalizeTrackerHost(value) {
  if (!value) return '';
  return value.trim().replace(/\.+$/, '').toLowerCase();
}

export function extractTrackerHost(value) {
  if (!value) return '';

  const trimmed = value.trim();
  if (!trimmed) return '';

  try {
    const parsed = new URL(trimmed);
    return normalizeTrackerHost(parsed.hostname);
  } catch {
    // ignore and try fallback parsing
  }

  try {
    const parsed = new URL(`https://${trimmed}`);
    return normalizeTrackerHost(parsed.hostname);
  } catch {
    const withoutScheme = trimmed.replace(/^[a-z]+:\/\//i, '');
    const host = withoutScheme.split('/')[0]?.split(':')[0] || '';
    return normalizeTrackerHost(host);
  }
}

export function getPrimaryTrackerHost(instance) {
  const explicit = normalizeTrackerHost(
    instance?.primaryTrackerHost || instance?.primary_tracker_host
  );
  if (explicit) {
    return explicit;
  }

  return extractTrackerHost(instance?.torrent?.announce || '');
}

export function buildTrackerFilterEntries(instances, filters = {}) {
  const counts = new Map();
  const trackerSearch = normalizeTrackerHost(filters.trackerSearch || '');

  for (const instance of instances || []) {
    const host = getPrimaryTrackerHost(instance);
    if (!host) continue;

    if (trackerSearch && !host.includes(trackerSearch)) {
      continue;
    }

    counts.set(host, (counts.get(host) || 0) + 1);
  }

  return [...counts.entries()]
    .map(([host, count]) => ({
      value: host,
      label: host,
      count,
      initial: host.charAt(0).toUpperCase(),
      iconUrl: `https://${host}/favicon.ico`,
    }))
    .sort((left, right) => {
      if (right.count !== left.count) {
        return right.count - left.count;
      }
      return left.label.localeCompare(right.label);
    });
}
