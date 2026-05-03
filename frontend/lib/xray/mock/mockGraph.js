/**
 * Mock XrayGraph generator for testing the xray architecture.
 * Generates a simple graph that mimics codeanalyzer output.
 */

import { createXrayGraph, addXrayNode, addXrayEdge } from "../types/XrayGraph.js";

/**
 * Generate a mock xray graph with the given node count.
 * Creates a simple structure: modules -> classes -> functions.
 * @param {number} nodeCount - Number of nodes to generate
 * @returns {object} XrayGraph
 */
export function generateMockXrayGraph(nodeCount = 50) {
  const graph = createXrayGraph();
  const languages = ["rust", "typescript", "javascript", "c"];
  const layers = ["core", "service", "ui", "util"];

  // Generate modules
  const moduleCount = Math.max(2, Math.floor(nodeCount / 10));
  const modules = [];
  for (let i = 0; i < moduleCount && i < nodeCount; i++) {
    const lang = languages[i % languages.length];
    const layer = layers[i % layers.length];
    const id = `module-${i}`;
    const node = {
      id,
      kind: "module",
      label: `module_${i}`,
      tags: [lang, layer, "module"],
      layer,
      language: lang,
      source: `src/${lang}/module_${i}.rs`,
    };
    addXrayNode(graph, node);
    modules.push(id);
  }

  // Generate functions/classes connected to modules
  for (let i = moduleCount; i < nodeCount; i++) {
    const lang = languages[i % languages.length];
    const layer = layers[i % layers.length];
    const kind = i % 5 === 0 ? "class" : "function";
    const moduleIdx = i % modules.length;
    const id = `${kind}-${i}`;
    const node = {
      id,
      kind,
      label: `${kind}_${i}`,
      tags: [lang, layer, kind],
      layer,
      language: lang,
      source: `src/${lang}/${kind}_${i}.rs`,
    };
    addXrayNode(graph, node);

    // Connect to parent module
    if (modules[moduleIdx]) {
      addXrayEdge(graph, {
        id: `${modules[moduleIdx]}->${id}`,
        source: modules[moduleIdx],
        target: id,
        kind: "direct",
        tags: ["imports"],
      });
    }

    // Connect to previous node sometimes
    if (i > moduleCount + 1 && i % 3 === 0) {
      const prevId = `${kind}-${i - 1}`;
      if (graph.nodes.has(prevId)) {
        addXrayEdge(graph, {
          id: `${prevId}->${id}`,
          source: prevId,
          target: id,
          kind: "direct",
          tags: ["calls"],
        });
      }
    }
  }

  return graph;
}

/**
 * Generate mock runtime stats for a node.
 * @returns {object} Runtime stats
 */
export function generateMockRuntimeStats() {
  return {
    hits: Math.floor(Math.random() * 1000),
    openSpans: Math.floor(Math.random() * 5),
    errors: Math.random() > 0.8 ? Math.floor(Math.random() * 10) : 0,
    totalUs: Math.floor(Math.random() * 100000),
    maxUs: Math.floor(Math.random() * 5000),
    lastTsUs: Date.now() * 1000,
  };
}
