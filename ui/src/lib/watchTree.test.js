import assert from 'node:assert/strict';
import test from 'node:test';
import {
  filterWatchFiles,
  isWithinPath,
  buildWatchTree,
  flattenTree,
  folderId,
} from './watchTree.js';

test('filterWatchFiles filters by search and status', () => {
  const files = [
    { filename: 'alpha/file-one.torrent', status: 'pending', name: 'File One' },
    { filename: 'alpha/file-two.torrent', status: 'loaded', name: 'File Two' },
    { filename: 'beta/file-three.torrent', status: 'invalid', name: 'File Three' },
  ];

  const filtered = filterWatchFiles(files, 'two', 'loaded');
  assert.equal(filtered.length, 1);
  assert.equal(filtered[0].filename, 'alpha/file-two.torrent');
});

test('isWithinPath respects folder nesting', () => {
  assert.ok(isWithinPath('root/file.torrent', ''));
  assert.ok(isWithinPath('alpha/file.torrent', 'alpha'));
  assert.ok(isWithinPath('alpha/nested/file.torrent', 'alpha'));
  assert.equal(isWithinPath('alpha/file.torrent', 'beta'), false);
});

test('flattenTree returns rows with folder ids', () => {
  const files = [{ filename: 'root.torrent' }, { filename: 'alpha/file.torrent' }];

  const tree = buildWatchTree(files);
  const rows = flattenTree(tree, new Set([folderId('')]));
  const rootRow = rows.find(row => row.type === 'folder' && row.depth === 0);
  assert.ok(rootRow);
  assert.equal(rootRow.id, folderId(''));
});
