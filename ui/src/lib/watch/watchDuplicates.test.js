import assert from 'node:assert/strict';
import test from 'node:test';

import { findDuplicateFiles } from './watchDuplicates.js';

function file(filename, info_hash = 'hash-1') {
  return { filename, info_hash, status: 'loaded' };
}

test('findDuplicateFiles returns an empty map for no files', () => {
  assert.equal(findDuplicateFiles([]).size, 0);
  assert.equal(findDuplicateFiles(undefined).size, 0);
  assert.equal(findDuplicateFiles('nope').size, 0);
});

test('findDuplicateFiles ignores unique hashes', () => {
  const duplicates = findDuplicateFiles([file('a.torrent', 'h1'), file('b.torrent', 'h2')]);
  assert.equal(duplicates.size, 0);
});

test('findDuplicateFiles maps the second file of a hash to the first', () => {
  const duplicates = findDuplicateFiles([file('a.torrent', 'h1'), file('b.torrent', 'h1')]);
  assert.equal(duplicates.size, 1);
  assert.equal(duplicates.get('b.torrent'), 'a.torrent');
});

test('findDuplicateFiles handles three files sharing one hash', () => {
  const duplicates = findDuplicateFiles([
    file('a.torrent', 'h1'),
    file('b.torrent', 'h1'),
    file('c.torrent', 'h1'),
  ]);
  assert.equal(duplicates.size, 2);
  assert.equal(duplicates.get('b.torrent'), 'a.torrent');
  assert.equal(duplicates.get('c.torrent'), 'a.torrent');
});

test('findDuplicateFiles skips files without an info hash', () => {
  const duplicates = findDuplicateFiles([
    file('broken.torrent', null),
    file('other.torrent', undefined),
    file('valid.torrent', 'h1'),
  ]);
  assert.equal(duplicates.size, 0);
});

test('findDuplicateFiles normalizes path separators', () => {
  const duplicates = findDuplicateFiles([
    file('folder\\a.torrent', 'h1'),
    file('folder/b.torrent', 'h1'),
  ]);
  assert.equal(duplicates.get('folder/b.torrent'), 'folder/a.torrent');
});
