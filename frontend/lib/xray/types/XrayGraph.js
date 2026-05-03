/**
 * Xray universal graph types for EdgeRun xray architecture.
 * These types represent the clean graph data model extracted from codeanalyzer.
 */

/**
 * @typedef {Object} XrayNode
 * @property {string} id - Unique node identifier
 * @property {string} kind - Node kind (function, class, module, etc.)
 * @property {string} label - Display label
 * @property {string[]} tags - Tag filters (language, layer, etc.)
 * @property {string} [layer] - Architecture layer
 * @property {string} [language] - Programming language
 * @property {string} [source] - Source file/path
 * @property {{x: number, y: number}} [position] - Graph position
 * @property {number} [connections] - Connection count for sizing
 * @property {object} [runtime] - Runtime stats (when runtime mode on)
 * @property {number} [runtime.hits] - Call/access count
 * @property {number} [runtime.openSpans] - Currently open spans
 * @property {number} [runtime.errors] - Error count
 * @property {number} [runtime.totalUs] - Total time in microseconds
 * @property {number} [runtime.maxUs] - Max single call time
 * @property {number} [runtime.lastTsUs] - Last timestamp
 */

/**
 * @typedef {Object} XrayEdge
 * @property {string} id - Unique edge identifier
 * @property {string} source - Source node id
 * @property {string} target - Target node id
 * @property {string} kind - Edge kind (calls, imports, etc.)
 * @property {string[]} tags - Edge tags
 * @property {object} [runtime] - Runtime stats
 * @property {number} [runtime.hits]
 * @property {number} [runtime.errors]
 * @property {number} [runtime.totalUs]
 * @property {number} [runtime.maxUs]
 * @property {number} [runtime.lastTsUs]
 */

/**
 * @typedef {Object} XrayGraph
 * @property {Map<string, XrayNode>} nodes - Nodes indexed by id
 * @property {XrayEdge[]} edges - Edge list
 * @property {Object} [meta] - Optional metadata
 */

/**
 * Create an empty XrayGraph
 * @returns {XrayGraph}
 */
export function createXrayGraph() {
  return {
    nodes: new Map(),
    edges: [],
    meta: {},
  };
}

/**
 * Add a node to the graph
 * @param {XrayGraph} graph
 * @param {XrayNode} node
 */
export function addXrayNode(graph, node) {
  graph.nodes.set(node.id, { ...node, connections: 0 });
  return graph;
}

/**
 * Add an edge to the graph and update connection counts
 * @param {XrayGraph} graph
 * @param {XrayEdge} edge
 */
export function addXrayEdge(graph, edge) {
  graph.edges.push(edge);
  const src = graph.nodes.get(edge.source);
  const tgt = graph.nodes.get(edge.target);
  if (src) src.connections = (src.connections || 0) + 1;
  if (tgt) tgt.connections = (tgt.connections || 0) + 1;
  return graph;
}

/**
 * Runtime stats placeholders
 */
export const RuntimeNodeStats = {
  hits: 0,
  openSpans: 0,
  errors: 0,
  totalUs: 0,
  maxUs: 0,
  lastTsUs: 0,
};

export const RuntimeEdgeStats = {
  hits: 0,
  errors: 0,
  totalUs: 0,
  maxUs: 0,
  lastTsUs: 0,
};
