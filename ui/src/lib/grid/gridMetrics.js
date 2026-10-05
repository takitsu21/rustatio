export function isGridLiveState(state) {
  const value = state?.toLowerCase();
  return value === 'running' || value === 'starting' || value === 'idle';
}

export function getGridLiveRate(state, rate) {
  return isGridLiveState(state) ? rate || 0 : 0;
}

export function getGridLivePeers(state, seeders, leechers) {
  if (!isGridLiveState(state)) {
    return { seeders: null, leechers: null };
  }

  return {
    seeders: seeders ?? null,
    leechers: leechers ?? null,
  };
}
