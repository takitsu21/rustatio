export function applyBaseGridFilters(instances, filters) {
  let result = instances;

  if (filters.search) {
    const search = filters.search.toLowerCase();
    result = result.filter(
      inst =>
        inst.name.toLowerCase().includes(search) ||
        inst.infoHash?.toLowerCase().includes(search) ||
        inst.tags?.some(t => t.toLowerCase().includes(search))
    );
  }

  if (filters.stateFilter !== 'all') {
    result = result.filter(inst => inst.state.toLowerCase() === filters.stateFilter);
  }

  if (filters.tagFilter) {
    result = result.filter(inst => inst.tags?.includes(filters.tagFilter));
  }

  return result;
}
