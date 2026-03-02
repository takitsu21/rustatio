import { writable } from 'svelte/store';

export const watchFocusQuery = writable('');

export function focusWatchQuery(query) {
  watchFocusQuery.set(String(query || '').trim());
}
