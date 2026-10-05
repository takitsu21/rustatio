import { normalizePath } from './watchTree.js';

/**
 * Map duplicate watch files to the first file that loaded the same torrent.
 * The watch engine dedupes by info-hash, so every file after the first one
 * shares the canonical file's instance.
 *
 * Returns Map<filename, canonicalFilename>.
 */
export function findDuplicateFiles(files = []) {
  const canonical = new Map();
  const duplicates = new Map();

  if (!Array.isArray(files)) return duplicates;

  for (const file of files) {
    const hash = file?.info_hash;
    const filename = normalizePath(file?.filename);
    if (!hash || !filename) continue;
    if (!canonical.has(hash)) {
      canonical.set(hash, filename);
      continue;
    }

    const first = canonical.get(hash);
    if (first !== filename) {
      duplicates.set(filename, first);
    }
  }

  return duplicates;
}
