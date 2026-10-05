// Pure helpers for bulk instance hydration.
// Grid bulk edit and backend reconciliation need many instances in the standard
// store; both hydrate from a single backend read instead of one read per id.

function indexById(items) {
  const index = new Map();
  for (const item of items || []) {
    if (!item || item.id === undefined || item.id === null) continue;
    index.set(String(item.id), item);
  }
  return index;
}

/**
 * Hydrate a list of instance ids using exactly one backend read.
 *
 * @param {Array<string|number>} ids - Instance ids to ensure.
 * @param {Array<object>} summaries - Optional grid summaries to pass through.
 * @param {() => Promise<Array<object>>} fetchServerInstances - Single backend read.
 * @param {(id: string, summary: object|null, serverInst: object|null) => Promise<any>} apply - Per-id hydrator.
 * @returns {Promise<Array<string>>} Ids that were successfully ensured.
 */
export async function hydrateInstances(ids, summaries, fetchServerInstances, apply) {
  const list = Array.isArray(ids) ? ids : [];
  if (list.length === 0) return [];

  let serverInstances = [];
  try {
    serverInstances = (await fetchServerInstances()) || [];
  } catch {
    // Backend list unavailable (Tauri/WASM) - hydrate from summaries only.
    serverInstances = [];
  }

  const serverIndex = indexById(serverInstances);
  const summaryIndex = indexById(summaries);

  const ensured = [];
  for (const rawId of list) {
    const id = String(rawId);
    const summary = summaryIndex.get(id) ?? null;
    const serverInst = serverIndex.get(id) ?? null;
    const result = await apply(id, summary, serverInst);
    if (result !== null && result !== undefined) {
      ensured.push(String(result));
    }
  }
  return ensured;
}
