/**
 * Development logging helper - only logs in development mode
 * @param {string} level - The console method to use (log, warn, error, etc.)
 * @param  {...any} args - Arguments to pass to console
 */
export function devLog(level = 'log', ...args) {
  if (import.meta.env.DEV) {
    console[level](...args);
  }
}
