export function getForwardedPort(status) {
  return status?.forwarded_port ?? status?.forwardedPort ?? null;
}

export function getPeerListenerPort(status) {
  return status?.peer_listener_port ?? status?.peerListenerPort ?? null;
}

export function getPeerListenerError(status) {
  return status?.peer_listener_error ?? status?.peerListenerError ?? null;
}

export function isNetworkConfigured(status) {
  return status?.configured !== false;
}

/**
 * Mask IP address for privacy (show first and last octets).
 */
export function maskIp(ip) {
  if (!ip) return '---';
  const parts = ip.split('.');
  if (parts.length === 4) {
    return `${parts[0]}.***.***.${parts[3]}`;
  }
  return ip.substring(0, 8) + '...';
}

/**
 * Compact network label for the transfer rail.
 */
export function getNetworkLabel(status) {
  if (!status) return null;
  if (!isNetworkConfigured(status)) {
    return { label: 'No VPN configured', tone: 'ratio' };
  }
  if (status.is_vpn) {
    return { label: status.organization || 'VPN active', tone: 'upload' };
  }
  return { label: 'No VPN', tone: 'ratio' };
}

/**
 * Proxy badge data for the transfer rail, or null when no proxy is active.
 */
export function getProxyStatus(status) {
  const enabled = status?.proxy_enabled ?? status?.proxyEnabled ?? false;
  const url = status?.proxy_url ?? status?.proxyUrl ?? '';
  if (!enabled || !url) return null;
  return {
    url,
    address: String(url).replace(/^[a-zA-Z0-9]+:\/\//, ''),
  };
}
