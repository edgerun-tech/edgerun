/**
 * Hit-testing for xray graph canvas.
 * Extracted from codeanalyzer GraphCanvas.jsx.
 */

/**
 * Find the closest node within threshold distance of screen point.
 * Uses spatial grid for efficient lookup.
 *
 * @param {number} sx - Screen X coordinate (device pixels)
 * @param {number} sy - Screen Y coordinate (device pixels)
 * @param {Map<string, object>} nodes - Graph nodes
 * @param {object} camera - Camera state { zoom, panX, panY, rotation }
 * @param {{width: number, height: number}} canvasSize - Canvas dimensions in device pixels
 * @param {Map} spatialGrid - Pre-built spatial grid
 * @param {number} threshold - Hit threshold in graph units (default 10/zoom)
 * @param {number} CELL_SIZE - Spatial grid cell size (default 80)
 * @returns {string|null} Node id or null
 */
export function hitTest(
  sx,
  sy,
  nodes,
  camera,
  canvasSize,
  spatialGrid,
  threshold = null,
  CELL_SIZE = 80
) {
  if (!spatialGrid || spatialGrid.size === 0) return null;

  const { zoom, panX, panY, rotation } = camera;
  const { width: vw, height: vh } = canvasSize;

  // Convert screen to graph coords (reuse screenToGraph from gl.js)
  const gx = (sx - vw * 0.5) / zoom + panX;
  const gy = (vh * 0.5 - sy) / zoom + panY;

  // Apply rotation inverse
  const c = Math.cos(-rotation);
  const s = Math.sin(-rotation);
  const rx = gx * c - gy * s;
  const ry = gx * s + gy * c;
  const graphX = rx - panX;
  const graphY = ry - panY;

  const hitThreshold = threshold ?? 10 / zoom;
  const cellSize = CELL_SIZE;
  const cx = Math.floor(graphX / cellSize);
  const cy = Math.floor(graphY / cellSize);

  let closest = null;
  let closestDist = hitThreshold;

  for (let dx = -1; dx <= 1; dx++) {
    for (let dy = -1; dy <= 1; dy++) {
      const key = `${cx + dx},${cy + dy}`;
      const cell = spatialGrid.get(key);
      if (!cell) continue;
      for (const id of cell) {
        const node = nodes.get(id);
        if (!node || node._hidden) continue;
        const ddx = (node.x ?? 0) - graphX;
        const ddy = (node.y ?? 0) - graphY;
        const d = Math.sqrt(ddx * ddx + ddy * ddy);
        if (d < closestDist) {
          closestDist = d;
          closest = id;
        }
      }
    }
  }
  return closest;
}

/**
 * Box selection: find all nodes within a screen-space rectangle.
 *
 * @param {object} rect - { x1, y1, x2, y2 } in screen coords (CSS pixels)
 * @param {Map<string, object>} nodes - Graph nodes
 * @param {object} camera - Camera state
 * @param {{width: number, height: number}} canvasSize - Canvas dimensions in device pixels
 * @param {number} dpr - Device pixel ratio
 * @returns {string[]} Array of node ids in box
 */
export function boxSelect(rect, nodes, camera, canvasSize, dpr = 1) {
  const { zoom, panX, panY, rotation } = camera;
  const { width: vw, height: vh } = canvasSize;

  // Convert screen rect to graph coords
  const x1 = Math.min(rect.x1, rect.x2) * dpr;
  const y1 = Math.min(rect.y1, rect.y2) * dpr;
  const x2 = Math.max(rect.x1, rect.x2) * dpr;
  const y2 = Math.max(rect.y1, rect.y2) * dpr;

  // Convert to graph coords (simplified - ignoring rotation for box select)
  const gx1 = (x1 - vw * 0.5) / zoom + panX;
  const gy1 = (vh * 0.5 - y1) / zoom + panY;
  const gx2 = (x2 - vw * 0.5) / zoom + panX;
  const gy2 = (vh * 0.5 - y2) / zoom + panY;

  const minGx = Math.min(gx1, gx2);
  const maxGx = Math.max(gx1, gx2);
  const minGy = Math.min(gy1, gy2);
  const maxGy = Math.max(gy1, gy2);

  const inBox = [];
  for (const [id, node] of nodes) {
    const nx = node.x ?? 0;
    const ny = node.y ?? 0;
    if (nx >= minGx && nx <= maxGx && ny >= minGy && ny <= maxGy) {
      inBox.push(id);
    }
  }
  return inBox;
}
