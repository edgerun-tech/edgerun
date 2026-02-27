/** @typedef {import("../mcp/types").ToolResponse} ToolResponse */

/**
 * @typedef {object} StoredResult
 * @property {string} id
 * @property {string} query
 * @property {ToolResponse} response
 * @property {boolean=} pinned
 * @property {string} createdAt
 */

/** @type {StoredResult[]} */
let results = [];
function getAllResults() {
  return [...results];
}
function getPinnedResults() {
  return results.filter((r) => r.pinned);
}
/**
 * @param {{query: string, response: ToolResponse}} input
 * @returns {StoredResult}
 */
function addResult(input) {
  const item = {
    id: `result-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
    query: input.query,
    response: input.response,
    createdAt: (/* @__PURE__ */ new Date()).toISOString()
  };
  results = [item, ...results];
  return item;
}
function removeResult(id) {
  results = results.filter((r) => r.id !== id);
}
function pinResult(id) {
  results = results.map((r) => r.id === id ? { ...r, pinned: !r.pinned } : r);
}
function clearResults() {
  results = [];
}
export {
  addResult,
  clearResults,
  getAllResults,
  getPinnedResults,
  pinResult,
  removeResult
};
