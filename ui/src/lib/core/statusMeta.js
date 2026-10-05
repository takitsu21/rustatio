/**
 * Presentation metadata for instance states and status banner types.
 * Keeps one visual language for state across table, sidebar, and detail views.
 */

const TONES = {
  upload: {
    text: 'text-stat-upload',
    border: 'border-stat-upload/40',
    bg: 'bg-stat-upload/10',
    dot: 'bg-stat-upload',
  },
  download: {
    text: 'text-stat-download',
    border: 'border-stat-download/40',
    bg: 'bg-stat-download/10',
    dot: 'bg-stat-download',
  },
  ratio: {
    text: 'text-stat-ratio',
    border: 'border-stat-ratio/40',
    bg: 'bg-stat-ratio/10',
    dot: 'bg-stat-ratio',
  },
  danger: {
    text: 'text-stat-danger',
    border: 'border-stat-danger/40',
    bg: 'bg-stat-danger/10',
    dot: 'bg-stat-danger',
  },
  primary: {
    text: 'text-primary',
    border: 'border-primary/40',
    bg: 'bg-primary/10',
    dot: 'bg-primary',
  },
  idle: {
    text: 'text-violet-400',
    border: 'border-violet-400/40',
    bg: 'bg-violet-400/10',
    dot: 'bg-violet-400',
  },
  muted: {
    text: 'text-muted-foreground',
    border: 'border-border',
    bg: 'bg-muted',
    dot: 'bg-muted-foreground',
  },
};

const STATE_META = {
  running: { label: 'Running', tone: 'upload', icon: 'circle' },
  idle: { label: 'Idle', tone: 'idle', icon: 'moon' },
  paused: { label: 'Paused', tone: 'ratio', icon: 'pause' },
  stopped: { label: 'Stopped', tone: 'muted', icon: 'square' },
  starting: { label: 'Starting', tone: 'primary', icon: 'loader', spin: true },
  stopping: { label: 'Stopping', tone: 'danger', icon: 'loader', spin: true },
  error: { label: 'Error', tone: 'danger', icon: 'alert' },
};

const TYPE_META = {
  idle: { label: 'Ready', tone: 'upload', icon: 'circle' },
  running: { label: 'Running', tone: 'primary', icon: 'rocket' },
  paused: { label: 'Paused', tone: 'ratio', icon: 'pause' },
  idling: { label: 'Idling', tone: 'ratio', icon: 'moon' },
  success: { label: 'Done', tone: 'upload', icon: 'check' },
  warning: { label: 'Warning', tone: 'ratio', icon: 'alert' },
  error: { label: 'Error', tone: 'danger', icon: 'alert' },
};

export const STATUS_TONES = TONES;

export function getTone(name) {
  return TONES[name] || TONES.muted;
}

export function getStateMeta(state) {
  const key = String(state ?? '').toLowerCase();
  const meta = STATE_META[key] || STATE_META.stopped;
  return { key: STATE_META[key] ? key : 'stopped', ...meta };
}

export function getStatusTypeMeta(type) {
  const key = String(type ?? '').toLowerCase();
  const meta = TYPE_META[key] || TYPE_META.idle;
  return { key: TYPE_META[key] ? key : 'idle', ...meta };
}
