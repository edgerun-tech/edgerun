/**
 * Adapter: codeanalyzer GraphData -> XrayGraph
 * Converts the prototype codeanalyzer data model into clean Xray types.
 */

import { createXrayGraph, addXrayNode, addXrayEdge } from "../types/XrayGraph.js";

/**
 * Adapt codeanalyzer GraphData to XrayGraph.
 *
 * codeanalyzer GraphData format (from parser/analyzer):
 * - nodes: Map<string, { id, kind, label, language, tags, x, y, ... }>
 * - edges: Array<{ source, target, kind, ... }>
 *
 * @param {object} graphData - codeanalyzer graph data
 * @param {Map<string, object>} graphData.nodes - Nodes map
 * @param {object[]} graphData.edges - Edges array
 * @returns {object} XrayGraph
 */
export function adaptCodeanalyzerToXray(graphData) {
  const xrayGraph = createXrayGraph();

  // Adapt nodes
  for (const [id, node] of graphData.nodes) {
    const xrayNode = {
      id: node.id || id,
      kind: node.kind || "unknown",
      label: node.label || id,
      tags: Array.isArray(node.tags) ? [...node.tags] : [],
      layer: node.layer || undefined,
      language: node.language || "unknown",
      source: node.source || undefined,
      position: {
        x: node.x ?? 0,
        y: node.y ?? 0,
      },
      connections: node.connections || 0,
    };
    addXrayNode(xrayGraph, xrayNode);
  }

  // Adapt edges
  for (const edge of graphData.edges) {
    const xrayEdge = {
      id: edge.id || `${edge.source}->${edge.target}`,
      source: edge.source,
      target: edge.target,
      kind: edge.kind || "direct",
      tags: Array.isArray(edge.tags) ? [...edge.tags] : [],
    };
    addXrayEdge(xrayGraph, xrayEdge);
  }

  return xrayGraph;
}

/**
 * Adapt codeanalyzer nodes map to XrayNode array.
 * Useful for incremental updates.
 *
 * @param {Map<string, object>} nodesMap
 * @returns {object[]} Array of XrayNode
 */
export function adaptNodes(nodesMap) {
  const nodes = [];
  for (const [id, node] of nodesMap) {
    nodes.push({
      id: node.id || id,
      kind: node.kind || "unknown",
      label: node.label || id,
      tags: Array.isArray(node.tags) ? [...node.tags] : [],
      layer: node.layer || undefined,
      language: node.language || "unknown",
      source: node.source || undefined,
      position: { x: node.x ?? 0, y: node.y ?? 0 },
    });
  }
  return nodes;
}

/**
 * Adapt codeanalyzer edges array to XrayEdge array.
 *
 * @param {object[]} edges
 * @returns {object[]} Array of XrayEdge
 */
export function adaptEdges(edges) {
  return edges.map((edge) => ({
    id: edge.id || `${edge.source}->${edge.target}`,
    source: edge.source,
    target: edge.target,
    kind: edge.kind || "direct",
    tags: Array.isArray(edge.tags) ? [...edge.tags] : [],
  }));
}
