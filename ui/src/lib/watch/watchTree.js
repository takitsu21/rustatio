function normalizePath(value) {
  if (!value) return '';
  return String(value)
    .replace(/\\/g, '/')
    .replace(/\/+?/g, '/')
    .replace(/^\/+/, '')
    .replace(/\/+$/, '');
}

function filterWatchFiles(files = [], query = '', status = 'all') {
  const trimmed = query.trim().toLowerCase();
  const statusKey = status ? String(status).toLowerCase() : 'all';
  const seen = new Set();
  const result = [];

  for (const file of files) {
    const filename = normalizePath(file?.filename);
    if (!filename) continue;
    if (seen.has(filename)) continue;
    seen.add(filename);

    const name = file?.name || filename;
    const haystack = `${name} ${filename}`.toLowerCase();
    if (trimmed && !haystack.includes(trimmed)) continue;

    const fileStatus = String(file?.status || '').toLowerCase();
    if (statusKey !== 'all' && fileStatus !== statusKey) continue;

    result.push({ ...file, filename });
  }

  return result;
}

function isWithinPath(filePath, folderPath) {
  const file = normalizePath(filePath);
  const folder = normalizePath(folderPath);
  if (!folder) return true;
  if (!file) return false;
  if (file === folder) return true;
  return file.startsWith(`${folder}/`);
}

function folderId(path) {
  return `folder:${path || '/'}`;
}

function fileId(path) {
  return `file:${path}`;
}

function createFolder(name, path) {
  return { type: 'folder', name, path, children: new Map() };
}

function sortChildren(children) {
  return Array.from(children.values()).sort((a, b) => {
    if (a.type !== b.type) return a.type === 'folder' ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
}

function countFiles(node) {
  if (node.type === 'file') return 1;
  let total = 0;
  for (const child of node.children.values()) {
    total += countFiles(child);
  }
  return total;
}

function buildWatchTree(files = []) {
  const root = createFolder('Root', '');

  for (const file of files) {
    const filename = normalizePath(file?.filename);
    if (!filename) continue;

    const parts = filename.split('/').filter(Boolean);
    if (parts.length === 0) continue;

    const fileName = parts[parts.length - 1];
    let current = root;
    let currentPath = '';

    for (const part of parts.slice(0, -1)) {
      currentPath = currentPath ? `${currentPath}/${part}` : part;
      let next = current.children.get(part);
      if (!next) {
        next = createFolder(part, currentPath);
        current.children.set(part, next);
      }
      current = next;
    }

    if (!current.children.has(fileName)) {
      current.children.set(fileName, {
        type: 'file',
        name: fileName,
        path: filename,
        file,
      });
    }
  }

  return root;
}

function flattenTree(tree, expandedIds = new Set()) {
  const rows = [];
  const expanded = expandedIds instanceof Set ? expandedIds : new Set();

  function walk(node, depth) {
    if (node.type === 'folder') {
      const id = folderId(node.path);
      const count = countFiles(node);
      const isExpanded = expanded.has(id);

      rows.push({
        id,
        type: 'folder',
        name: node.name,
        path: node.path,
        depth,
        count,
        childCount: node.children.size,
        isExpanded,
      });

      if (!isExpanded) return;

      for (const child of sortChildren(node.children)) {
        walk(child, depth + 1);
      }
      return;
    }

    rows.push({
      id: fileId(node.path),
      type: 'file',
      name: node.name,
      path: node.path,
      depth,
      file: node.file,
    });
  }

  if (tree) walk(tree, 0);
  return rows;
}

function collectFolderIds(tree) {
  const ids = new Set();

  function walk(node) {
    if (!node || node.type !== 'folder') return;
    ids.add(folderId(node.path));
    for (const child of node.children.values()) {
      if (child.type === 'folder') walk(child);
    }
  }

  walk(tree);
  return ids;
}

export {
  normalizePath,
  filterWatchFiles,
  isWithinPath,
  folderId,
  buildWatchTree,
  flattenTree,
  collectFolderIds,
};
