function formatNumber(value, digits = 1) {
  return parseFloat(value.toFixed(digits));
}

export function formatBytes(bytes, digits = 1) {
  if (!bytes || bytes <= 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  const i = Math.min(sizes.length - 1, Math.floor(Math.log(bytes) / Math.log(k)));
  return `${formatNumber(bytes / Math.pow(k, i), digits)} ${sizes[i]}`;
}

export function formatBytesCompact(bytes) {
  if (!bytes || bytes <= 0) return '0';
  const k = 1024;
  const sizes = ['B', 'K', 'M', 'G', 'T', 'P'];
  const i = Math.min(sizes.length - 1, Math.floor(Math.log(bytes) / Math.log(k)));
  return `${formatNumber(bytes / Math.pow(k, i), 1)}${sizes[i]}`;
}

/**
 * Rates are expressed in KB/s.
 */
export function formatRate(rate, digits = 1) {
  if (!rate || rate <= 0) return '0 KB/s';
  if (rate >= 1000000) return `${formatNumber(rate / 1024 / 1024, digits)} GB/s`;
  if (rate >= 1000) return `${formatNumber(rate / 1024, digits)} MB/s`;
  return `${formatNumber(rate, digits)} KB/s`;
}

export function formatRateCompact(rate) {
  if (!rate || rate <= 0) return '0';
  if (rate >= 1000000) return `${formatNumber(rate / 1024 / 1024, 1)}G`;
  if (rate >= 1000) return `${formatNumber(rate / 1024, 1)}M`;
  return `${formatNumber(rate, 1)}K`;
}
