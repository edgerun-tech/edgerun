/**
 * Xray GraphRenderer - WebGL2 renderer for graph visualization.
 * Extracted from codeanalyzer GraphCanvas.jsx.
 * Handles WebGL setup, buffer management, and render loop.
 * Does NOT handle app state, panels, or UI - only rendering.
 */

import {
  buildNodeProgram,
  buildEdgeProgram,
} from "./gl.js";
import { getNodeColor, getNodeSize, getEdgeColor } from "./colorPolicy.js";

const MAX_BUFFERED_NODES = 20000;
const MAX_BUFFERED_EDGES = 100000;

/**
 * Create a new GraphRenderer instance.
 * @param {HTMLCanvasElement} canvas - WebGL canvas
 * @param {HTMLCanvasElement} overlayCanvas - 2D overlay canvas for UI
 * @returns {object} Renderer instance
 */
export function createGraphRenderer(canvas, overlayCanvas) {
  const gl = canvas.getContext("webgl2", { alpha: true, antialias: true });
  if (!gl) throw new Error("WebGL2 not supported");

  const nodeProgram = buildNodeProgram(gl);
  const edgeProgram = buildEdgeProgram(gl);

  // Pre-allocate WebGL buffers
  const nodeBuffers = {
    positions: gl.createBuffer(),
    colors: gl.createBuffer(),
    sizes: gl.createBuffer(),
    selected: gl.createBuffer(),
  };
  const edgeBuffer = gl.createBuffer();

  // Pre-allocated TypedArrays (reuse to avoid GC)
  const positions = new Float32Array(MAX_BUFFERED_NODES * 2);
  const colors = new Float32Array(MAX_BUFFERED_NODES * 4);
  const sizes = new Float32Array(MAX_BUFFERED_NODES);
  const sels = new Float32Array(MAX_BUFFERED_NODES);
  const edgeFloat = new Float32Array(MAX_BUFFERED_EDGES * 2 * 5);

  // Cache uniform locations
  const nodeUniforms = {
    u_resolution: gl.getUniformLocation(nodeProgram, "u_resolution"),
    u_zoom: gl.getUniformLocation(nodeProgram, "u_zoom"),
    u_pan: gl.getUniformLocation(nodeProgram, "u_pan"),
    u_rotation: gl.getUniformLocation(nodeProgram, "u_rotation"),
    a_position: gl.getAttribLocation(nodeProgram, "a_position"),
    a_color: gl.getAttribLocation(nodeProgram, "a_color"),
    a_size: gl.getAttribLocation(nodeProgram, "a_size"),
    a_selected: gl.getAttribLocation(nodeProgram, "a_selected"),
  };

  const edgeUniforms = {
    u_resolution: gl.getUniformLocation(edgeProgram, "u_resolution"),
    u_zoom: gl.getUniformLocation(edgeProgram, "u_zoom"),
    u_pan: gl.getUniformLocation(edgeProgram, "u_pan"),
    u_rotation: gl.getUniformLocation(edgeProgram, "u_rotation"),
    a_position: gl.getAttribLocation(edgeProgram, "a_position"),
    a_color: gl.getAttribLocation(edgeProgram, "a_color"),
  };

  return {
    gl,
    canvas,
    overlayCanvas,
    nodeProgram,
    edgeProgram,
    nodeBuffers,
    edgeBuffer,
    positions,
    colors,
    sizes,
    sels,
    edgeFloat,
    nodeUniforms,
    edgeUniforms,

    /**
     * Resize canvases to match container.
     */
    resize() {
      const container = canvas.parentElement;
      if (!container) return;
      const w = container.clientWidth;
      const h = container.clientHeight;
      if (w <= 0 || h <= 0) return;
      const dpr = window.devicePixelRatio || 1;
      canvas.width = w * dpr;
      canvas.height = h * dpr;
      canvas.style.width = `${w}px`;
      canvas.style.height = `${h}px`;
      gl.viewport(0, 0, canvas.width, canvas.height);
      if (overlayCanvas) {
        overlayCanvas.width = w * dpr;
        overlayCanvas.height = h * dpr;
        overlayCanvas.style.width = `${w}px`;
        overlayCanvas.style.height = `${h}px`;
      }
    },

    /**
     * Render one frame.
     * @param {object} state - Render state from app
     * @param {Map<string, object>} state.nodes - Graph nodes
     * @param {object[]} state.edges - Graph edges
     * @param {Set<string>} state.selectedIds - Selected node ids
     * @param {Set<string>} state.highlightedIds - Highlighted node ids
     * @param {number} state.zoom - Zoom level
     * @param {number} state.panX - Pan X offset
     * @param {number} state.panY - Pan Y offset
     * @param {number} state.rotation - Rotation angle
     * @param {object} [state.boxSelectRect] - Box selection rect
     * @param {number} [state.rotationIndicator] - Rotation indicator angle
     * @param {boolean} [state.runtimeMode=false] - Runtime mode on/off
     */
    render(state) {
      const {
        nodes = [],
        edges = [],
        selectedIds = new Set(),
        highlightedIds = new Set(),
        zoom = 1,
        panX = 0,
        panY = 0,
        rotation = 0,
        boxSelectRect = null,
        rotationIndicator = null,
        runtimeMode = false,
      } = state;

      const vw = canvas.width;
      const vh = canvas.height;
      const nodeCount = nodes.size;

      gl.clearColor(0.06, 0.06, 0.08, 1.0);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.enable(gl.BLEND);
      gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

      if (nodeCount === 0) {
        this.renderOverlay({ boxSelectRect, rotationIndicator, zoom: () => zoom, panX: () => panX, panY: () => panY, rotation: () => rotation });
        return;
      }

      // Build node buffers
      let ni = 0;
      for (const [id, node] of nodes) {
        positions[ni * 2] = node.x ?? 0;
        positions[ni * 2 + 1] = node.y ?? 0;
        const isSelected = selectedIds.has(id);
        const isHighlighted = highlightedIds.has(id);
        const color = getNodeColor(node.language ?? "unknown", isHighlighted, isSelected, node.runtime, runtimeMode);
        colors[ni * 4] = color[0];
        colors[ni * 4 + 1] = color[1];
        colors[ni * 4 + 2] = color[2];
        colors[ni * 4 + 3] = color[3];
        sizes[ni] = getNodeSize(node.connections ?? 0, isHighlighted, isSelected, node.runtime, runtimeMode);
        sels[ni] = isSelected ? 1 : 0;
        node._renderIndex = ni;
        ni++;
      }

      // Build edge buffer
      let edgeVertCount = 0;
      for (const edge of edges) {
        if (edgeVertCount >= MAX_BUFFERED_EDGES * 2) break;
        const src = nodes.get(edge.source);
        const tgt = nodes.get(edge.target);
        const si = src?._renderIndex;
        const ti = tgt?._renderIndex;
        if (si === undefined || ti === undefined) continue;
        const ec = getEdgeColor(edge.kind ?? "", highlightedIds.has(edge.source) || highlightedIds.has(edge.target), edge.runtime, runtimeMode);

        // Source vertex
        const ei = edgeVertCount * 5;
        edgeFloat[ei] = positions[si * 2];
        edgeFloat[ei + 1] = positions[si * 2 + 1];
        edgeFloat[ei + 2] = ec[0];
        edgeFloat[ei + 3] = ec[1];
        edgeFloat[ei + 4] = ec[2];
        edgeVertCount++;

        // Target vertex
        const ti2 = edgeVertCount * 5;
        edgeFloat[ti2] = positions[ti * 2];
        edgeFloat[ti2 + 1] = positions[ti * 2 + 1];
        edgeFloat[ti2 + 2] = ec[3];
        edgeFloat[ti2 + 3] = ec[4];
        edgeFloat[ti2 + 4] = ec[5];
        edgeVertCount++;
      }

      // Draw edges
      if (edgeProgram && edgeVertCount > 0) {
        gl.useProgram(edgeProgram);
        for (let i = 0; i < 16; i++) gl.disableVertexAttribArray(i);

        gl.bindBuffer(gl.ARRAY_BUFFER, edgeBuffer);
        gl.bufferData(gl.ARRAY_BUFFER, edgeFloat.subarray(0, edgeVertCount * 5), gl.DYNAMIC_DRAW);

        if (edgeUniforms.u_resolution) gl.uniform2f(edgeUniforms.u_resolution, vw, vh);
        if (edgeUniforms.u_zoom) gl.uniform1f(edgeUniforms.u_zoom, zoom);
        if (edgeUniforms.u_pan) gl.uniform2f(edgeUniforms.u_pan, panX, panY);
        if (edgeUniforms.u_rotation) gl.uniform1f(edgeUniforms.u_rotation, rotation);

        if (edgeUniforms.a_position >= 0 && edgeUniforms.a_color >= 0) {
          gl.enableVertexAttribArray(edgeUniforms.a_position);
          gl.enableVertexAttribArray(edgeUniforms.a_color);
          gl.vertexAttribPointer(edgeUniforms.a_position, 2, gl.FLOAT, false, 20, 0);
          gl.vertexAttribPointer(edgeUniforms.a_color, 3, gl.FLOAT, false, 20, 8);
          gl.drawArrays(gl.LINES, 0, edgeVertCount);
        }
      }

      // Draw nodes
      if (nodeProgram && nodeCount > 0) {
        gl.useProgram(nodeProgram);
        for (let i = 0; i < 16; i++) gl.disableVertexAttribArray(i);

        gl.bindBuffer(gl.ARRAY_BUFFER, nodeBuffers.positions);
        gl.bufferData(gl.ARRAY_BUFFER, positions.subarray(0, nodeCount * 2), gl.DYNAMIC_DRAW);
        if (nodeUniforms.a_position >= 0) {
          gl.enableVertexAttribArray(nodeUniforms.a_position);
          gl.vertexAttribPointer(nodeUniforms.a_position, 2, gl.FLOAT, false, 0, 0);
        }

        gl.bindBuffer(gl.ARRAY_BUFFER, nodeBuffers.colors);
        gl.bufferData(gl.ARRAY_BUFFER, colors.subarray(0, nodeCount * 4), gl.DYNAMIC_DRAW);
        if (nodeUniforms.a_color >= 0) {
          gl.enableVertexAttribArray(nodeUniforms.a_color);
          gl.vertexAttribPointer(nodeUniforms.a_color, 4, gl.FLOAT, false, 0, 0);
        }

        gl.bindBuffer(gl.ARRAY_BUFFER, nodeBuffers.sizes);
        gl.bufferData(gl.ARRAY_BUFFER, sizes.subarray(0, nodeCount), gl.DYNAMIC_DRAW);
        if (nodeUniforms.a_size >= 0) {
          gl.enableVertexAttribArray(nodeUniforms.a_size);
          gl.vertexAttribPointer(nodeUniforms.a_size, 1, gl.FLOAT, false, 0, 0);
        }

        gl.bindBuffer(gl.ARRAY_BUFFER, nodeBuffers.selected);
        gl.bufferData(gl.ARRAY_BUFFER, sels.subarray(0, nodeCount), gl.DYNAMIC_DRAW);
        if (nodeUniforms.a_selected >= 0) {
          gl.enableVertexAttribArray(nodeUniforms.a_selected);
          gl.vertexAttribPointer(nodeUniforms.a_selected, 1, gl.FLOAT, false, 0, 0);
        }

        if (nodeUniforms.u_resolution) gl.uniform2f(nodeUniforms.u_resolution, vw, vh);
        if (nodeUniforms.u_zoom) gl.uniform1f(nodeUniforms.u_zoom, zoom);
        if (nodeUniforms.u_pan) gl.uniform2f(nodeUniforms.u_pan, panX, panY);
        if (nodeUniforms.u_rotation) gl.uniform1f(nodeUniforms.u_rotation, rotation);

        gl.drawArrays(gl.POINTS, 0, nodeCount);
      }

      this.renderOverlay({ boxSelectRect, rotationIndicator, zoom: () => zoom, panX: () => panX, panY: () => panY, rotation: () => rotation });
    },

    /**
     * Render 2D overlay (selection box, rotation indicator, FPS).
     */
    renderOverlay({ boxSelectRect, rotationIndicator, zoom, panX, panY, rotation }) {
      if (!overlayCanvas) return;
      const ctx = overlayCanvas.getContext("2d");
      if (!ctx) return;
      const dpr = window.devicePixelRatio || 1;
      const w = overlayCanvas.width;
      const h = overlayCanvas.height;
      ctx.clearRect(0, 0, w, h);

      // Box selection
      const bs = boxSelectRect;
      if (bs) {
        const x = Math.min(bs.x1, bs.x2) * dpr;
        const y = Math.min(bs.y1, bs.y2) * dpr;
        const bw = Math.abs(bs.x2 - bs.x1) * dpr;
        const bh = Math.abs(bs.y2 - bs.y1) * dpr;
        ctx.fillStyle = "rgba(100, 180, 255, 0.12)";
        ctx.fillRect(x, y, bw, bh);
        ctx.strokeStyle = "rgba(100, 180, 255, 0.7)";
        ctx.lineWidth = 1.5 * dpr;
        ctx.setLineDash([6 * dpr, 4 * dpr]);
        ctx.strokeRect(x, y, bw, bh);
        ctx.setLineDash([]);
      }

      // Rotation indicator
      const rot = rotationIndicator;
      if (rot !== null && Math.abs(rot) > 0.01) {
        const cx = w / 2;
        const cy = h / 2;
        const radius = 50 * dpr;
        ctx.beginPath();
        ctx.arc(cx, cy, radius, 0, rot, rot < 0);
        ctx.strokeStyle = "rgba(255, 200, 50, 0.6)";
        ctx.lineWidth = 3 * dpr;
        ctx.lineCap = "round";
        ctx.stroke();
        const deg = ((rot * 180) / Math.PI).toFixed(0);
        ctx.fillStyle = "rgba(255, 200, 50, 0.8)";
        ctx.font = `${12 * dpr}px monospace`;
        ctx.textAlign = "center";
        ctx.fillText(`${deg}°`, cx, cy + radius + 18 * dpr);
      }
    },

    /**
     * Cleanup WebGL resources.
     */
    destroy() {
      if (gl) {
        gl.deleteProgram(nodeProgram);
        gl.deleteProgram(edgeProgram);
        gl.deleteBuffer(nodeBuffers.positions);
        gl.deleteBuffer(nodeBuffers.colors);
        gl.deleteBuffer(nodeBuffers.sizes);
        gl.deleteBuffer(nodeBuffers.selected);
        gl.deleteBuffer(edgeBuffer);
      }
    },
  };
}
