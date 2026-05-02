/**
 * Force-directed layout simulation for xray graph.
 * Extracted from codeanalyzer GraphCanvas.jsx.
 * Uses spatial grid for performance.
 */

const DEFAULTS = {
  MAX_LAYOUT_FRAMES: 180,
  REPULSION: 2000,
  ATTRACTION: 0.001,
  GRAVITY: 0.001,
  DAMPING: 0.9,
  CELL_SIZE: 80,
};

/**
 * Create a new layout simulation state.
 * @param {object} options - Override default constants
 * @returns {object} Layout state
 */
export function createLayoutState(options = {}) {
  return {
    running: false,
    frames: 0,
    ...DEFAULTS,
    ...options,
    spatialGrid: new Map(),
    spatialGridBuilt: false,
  };
}

/**
 * Initialize node positions with random spread if not set.
 * @param {Map<string, object>} nodes - Graph nodes
 */
export function initPositions(nodes) {
  const count = nodes.size;
  if (count === 0) return;
  const spread = Math.sqrt(count) * 30;
  for (const [, node] of nodes) {
    if (node.x === undefined || (node.x === 0 && node.y === 0)) {
      node.x = (Math.random() - 0.5) * spread;
      node.y = (Math.random() - 0.5) * spread;
      node.vx = 0;
      node.vy = 0;
    }
  }
}

/**
 * Build spatial grid for efficient neighbor queries.
 * @param {Map<string, object>} nodes
 * @param {number} cellSize
 * @returns {Map} spatial grid
 */
export function buildSpatialGrid(nodes, cellSize = DEFAULTS.CELL_SIZE) {
  const grid = new Map();
  for (const [id, node] of nodes) {
    if (node._hidden) continue;
    const cx = Math.floor((node.x ?? 0) / cellSize);
    const cy = Math.floor((node.y ?? 0) / cellSize);
    const key = `${cx},${cy}`;
    if (!grid.has(key)) grid.set(key, []);
    grid.get(key).push(id);
  }
  return grid;
}

/**
 * Step the layout simulation forward one frame.
 * @param {object} state - Layout state (mutable)
 * @param {Map<string, object>} nodes
 * @param {object[]} edges
 */
export function stepLayout(state, nodes, edges) {
  if (!state.running) return;

  const {
    REPULSION,
    ATTRACTION,
    GRAVITY,
    DAMPING,
    CELL_SIZE,
  } = state;

  // Build spatial grid for repulsion phase
  state.spatialGrid.clear();
  for (const [id, node] of nodes) {
    const cx = Math.floor((node.x ?? 0) / CELL_SIZE);
    const cy = Math.floor((node.y ?? 0) / CELL_SIZE);
    const key = cx * 100000 + cy; // Numeric key for performance
    if (!state.spatialGrid.has(key)) state.spatialGrid.set(key, []);
    state.spatialGrid.get(key).push({ x: node.x ?? 0, y: node.y ?? 0, node });
  }

  // Repulsion
  for (const [, node] of nodes) {
    let fx = 0;
    let fy = 0;
    const cx = Math.floor((node.x ?? 0) / CELL_SIZE);
    const cy = Math.floor((node.y ?? 0) / CELL_SIZE);
    for (let dx = -1; dx <= 1; dx++) {
      for (let dy = -1; dy <= 1; dy++) {
        const key = (cx + dx) * 100000 + (cy + dy);
        const cell = state.spatialGrid.get(key);
        if (!cell) continue;
        for (const other of cell) {
          if (other.node === node) continue;
          const ddx = (node.x ?? 0) - other.x;
          const ddy = (node.y ?? 0) - other.y;
          const distSq = ddx * ddx + ddy * ddy + 1;
          const dist = Math.sqrt(distSq);
          const force = REPULSION / distSq;
          fx += (ddx / dist) * force;
          fy += (ddy / dist) * force;
        }
      }
    }
    node.vx = (node.vx ?? 0) * DAMPING + fx * 0.5;
    node.vy = (node.vy ?? 0) * DAMPING + fy * 0.5;
  }

  // Attraction (edges pull nodes together)
  for (const edge of edges) {
    const src = nodes.get(edge.source);
    const tgt = nodes.get(edge.target);
    if (!src || !tgt) continue;
    const dx = (tgt.x ?? 0) - (src.x ?? 0);
    const dy = (tgt.y ?? 0) - (src.y ?? 0);
    const dist = Math.sqrt(dx * dx + dy * dy) + 0.1;
    const force = dist * ATTRACTION;
    const ffx = (dx / dist) * force;
    const ffy = (dy / dist) * force;
    src.vx = (src.vx ?? 0) + ffx;
    src.vy = (src.vy ?? 0) + ffy;
    tgt.vx = (tgt.vx ?? 0) - ffx;
    tgt.vy = (tgt.vy ?? 0) - ffy;
  }

  // Gravity + apply velocity
  let totalEnergy = 0;
  for (const [, node] of nodes) {
    node.vx = (node.vx ?? 0) - (node.x ?? 0) * GRAVITY;
    node.vy = (node.vy ?? 0) - (node.y ?? 0) * GRAVITY;
    const vx = Math.max(-10, Math.min(10, node.vx));
    const vy = Math.max(-10, Math.min(10, node.vy));
    node.x = Math.max(-10000, Math.min(10000, (node.x ?? 0) + vx));
    node.y = Math.max(-10000, Math.min(10000, (node.y ?? 0) + vy));
    node.vx = vx * DAMPING;
    node.vy = vy * DAMPING;
    totalEnergy += node.vx * node.vx + node.vy * node.vy;
  }

  state.frames++;
  if (state.frames >= state.MAX_LAYOUT_FRAMES || totalEnergy < 50) {
    state.running = false;
    for (const [, node] of nodes) {
      node.vx = 0;
      node.vy = 0;
    }
  }
}
