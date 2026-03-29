export const GRID_STATE_FILTER_OPTIONS = [
  { value: 'all', label: 'All States', icon: 'circle', tone: 'text-muted-foreground' },
  { value: 'starting', label: 'Starting', icon: 'loader', tone: 'text-primary', spin: true },
  { value: 'running', label: 'Running', icon: 'circle', tone: 'text-stat-upload' },
  { value: 'stopping', label: 'Stopping', icon: 'loader', tone: 'text-stat-danger', spin: true },
  { value: 'paused', label: 'Paused', icon: 'pause', tone: 'text-stat-ratio' },
  { value: 'idle', label: 'Idle', icon: 'moon', tone: 'text-violet-500' },
  { value: 'stopped', label: 'Stopped', icon: 'square', tone: 'text-muted-foreground' },
];

export function getGridStateFilterOption(value) {
  return (
    GRID_STATE_FILTER_OPTIONS.find(option => option.value === value) || GRID_STATE_FILTER_OPTIONS[0]
  );
}

export function getGridTagFilterOptions(tags = []) {
  const values = [...new Set(tags.filter(Boolean))].sort((a, b) => a.localeCompare(b));

  return [{ value: '', label: 'All Tags' }, ...values.map(tag => ({ value: tag, label: tag }))];
}
