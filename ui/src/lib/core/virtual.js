/**
 * Fixed-row-height windowing math shared by the grid table and the sidebar.
 * All values are in rendered pixels; callers pass the current row height,
 * which already accounts for the interface zoom.
 */
export function getWindow({ total, scrollTop, viewportHeight, rowHeight, buffer = 0 }) {
  const count = Number.isFinite(total) && total > 0 ? Math.floor(total) : 0;
  const height = Number.isFinite(rowHeight) && rowHeight > 0 ? rowHeight : 1;
  const viewport = Number.isFinite(viewportHeight) && viewportHeight > 0 ? viewportHeight : 0;
  const top = Number.isFinite(scrollTop) && scrollTop > 0 ? scrollTop : 0;
  const padding = Number.isFinite(buffer) && buffer > 0 ? Math.floor(buffer) : 0;

  const totalHeight = count * height;
  const firstVisible = Math.floor(top / height);
  const startIndex = Math.max(0, Math.min(count, firstVisible - padding));
  const visibleCount = Math.ceil(viewport / height) + padding * 2 + 1;
  const endIndex = Math.max(startIndex, Math.min(count, startIndex + visibleCount));

  return {
    startIndex,
    endIndex,
    offsetY: startIndex * height,
    totalHeight,
  };
}

/**
 * Smallest scrollTop that keeps the row at `index` fully visible,
 * or the current scrollTop when it already is (or the index is invalid).
 */
export function getScrollTopForIndex({ index, currentScrollTop, viewportHeight, rowHeight }) {
  const count = Number.isFinite(index) ? Math.floor(index) : -1;
  const top = Number.isFinite(currentScrollTop) && currentScrollTop > 0 ? currentScrollTop : 0;
  if (count < 0) return top;

  const height = Number.isFinite(rowHeight) && rowHeight > 0 ? rowHeight : 1;
  const viewport = Number.isFinite(viewportHeight) && viewportHeight > 0 ? viewportHeight : 0;

  const rowTop = count * height;
  const rowBottom = rowTop + height;

  if (rowTop < top) return rowTop;
  if (rowBottom > top + viewport) return Math.max(0, rowBottom - viewport);
  return top;
}
