/**
 * Xray color policy for nodes and edges.
 * Extracted from codeanalyzer GraphCanvas.jsx.
 * Runtime mode support: hits increase brightness, errors red, openSpans pulse.
 */

/**
 * Get node color based on node properties and interaction state.
 * @param {string} lang - Programming language
 * @param {boolean} highlighted - Is node highlighted
 * @param {boolean} selected - Is node selected
 * @param {object} [runtime] - Runtime stats (optional, for runtime mode)
 * @param {boolean} [runtimeMode=false] - Whether runtime mode is on
 * @returns {number[]} RGBA color array
 */
export function getNodeColor(lang, highlighted, selected, runtime = null, runtimeMode = false) {
  const base = getLangColor(lang);

  // Runtime mode: adjust color based on runtime stats
  if (runtimeMode && runtime) {
    return getRuntimeNodeColor(base, runtime);
  }

  // Static mode: selection/highlight takes priority
  if (selected) return [1, 1, 1, 1];
  if (highlighted) return [1, 0.85, 0.18, 1];
  return base;
}

/**
 * Get node color in runtime mode.
 * - errors -> red
 * - openSpans -> pulse white-red
 * - hits -> increase brightness
 * @param {number[]} baseColor - Base RGBA color
 * @param {object} runtime - Runtime stats
 * @returns {number[]} RGBA color array
 */
function getRuntimeNodeColor(baseColor, runtime) {
  const [r, g, b, a] = baseColor;

  // Errors: turn red
  if (runtime.errors > 0) {
    const errorIntensity = Math.min(runtime.errors / 10, 1.0);
    return [
      r + (1.0 - r) * errorIntensity,
      g * (1.0 - errorIntensity),
      b * (1.0 - errorIntensity),
      a
    ];
  }

  // Open spans: pulse white-red effect (simplified)
  if (runtime.openSpans > 0) {
    const pulse = 0.5 + 0.5 * Math.sin(Date.now() / 200);
    return [
      1.0 - (1.0 - r) * (1 - pulse * 0.3),
      0.3 + 0.7 * (1 - pulse * 0.5),
      0.3 + 0.7 * (1 - pulse * 0.5),
      a
    ];
  }

  // Hits: increase brightness
  if (runtime.hits > 0) {
    const hitBoost = Math.min(runtime.hits / 100, 0.5);
    return [
      Math.min(r + hitBoost, 1.0),
      Math.min(g + hitBoost, 1.0),
      Math.min(b + hitBoost, 1.0),
      a
    ];
  }

  return baseColor;
}

/**
 * Get node size based on connections and interaction state.
 * Runtime mode: maxUs increases node size.
 * @param {number} connections - Number of connections
 * @param {boolean} highlighted - Is node highlighted
 * @param {boolean} selected - Is node selected
 * @param {object} [runtime] - Runtime stats (optional)
 * @param {boolean} [runtimeMode=false] - Whether runtime mode is on
 * @returns {number} Node size
 */
export function getNodeSize(connections, highlighted, selected, runtime = null, runtimeMode = false) {
  const base = 6 + Math.min(connections * 0.5, 12);

  // Runtime mode: maxUs increases size
  if (runtimeMode && runtime && runtime.maxUs > 0) {
    const sizeBoost = Math.min(runtime.maxUs / 1000, 2.0); // cap at 2x
    const size = base * (1.0 + sizeBoost);
  } else {
    const size = base;
  }

  if (selected) return size * 1.6;
  if (highlighted) return size * 1.3;
  return size;
}

/**
 * Get edge color based on edge kind and highlight state.
 * @param {string} kind - Edge kind (direct, indirect, macro)
 * @param {boolean} highlighted - Is edge highlighted
 * @param {object} [runtime] - Runtime stats (optional)
 * @param {boolean} [runtimeMode=false] - Whether runtime mode is on
 * @returns {number[]} RGBA color array (6 values for 2 vertices)
 */
export function getEdgeColor(kind, highlighted, runtime = null, runtimeMode = false) {
  if (highlighted) return [0.9, 0.8, 0.2, 0.9, 0.8, 0.2];

  // Runtime mode: adjust based on runtime stats
  if (runtimeMode && runtime) {
    if (runtime.errors > 0) {
      const intensity = Math.min(runtime.errors / 10, 1.0);
      return [
        0.25 + 0.75 * intensity,
        0.25 * (1 - intensity * 0.5),
        0.3 * (1 - intensity * 0.5),
        0.25 + 0.75 * intensity,
        0.3 * (1 - intensity * 0.5),
        0.3 * (1 - intensity * 0.5)
      ];
    }
    if (runtime.hits > 0) {
      const boost = Math.min(runtime.hits / 100, 0.5);
      return [
        0.25 + boost, 0.25 + boost, 0.3 + boost,
        0.25 + boost, 0.3 + boost, 0.3 + boost
      ];
    }
  }

  const kinds = {
    direct: [0.25, 0.25, 0.3, 0.25, 0.3, 0.35],
    indirect: [0.8, 0.3, 0.3, 0.8, 0.3, 0.3],
    macro: [0.7, 0.6, 0.2, 0.7, 0.6, 0.2],
  };
  return kinds[kind] ?? [0.3, 0.3, 0.35, 0.3, 0.3, 0.35];
}

/**
 * Language to color mapping.
 * @param {string} lang
 * @returns {number[]} RGBA color array
 */
export function getLangColor(lang) {
  const LANG_COLORS = {
    c: [0.35, 0.6, 0.95, 0.9],
    rust: [0.95, 0.5, 0.15, 0.9],
    typescript: [0.4, 0.8, 0.4, 0.9],
    javascript: [0.9, 0.8, 0.2, 0.9],
    unknown: [0.5, 0.5, 0.55, 0.7],
  };
  return LANG_COLORS[lang] ?? LANG_COLORS.unknown;
}
