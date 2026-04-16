/** @jsxImportSource solid-js **/
/**
 * WebGL graph canvas with force-directed layout, pan/zoom, and interaction.
 */
import { onMount, onCleanup } from "solid-js";
import {
  nodesMap,
  edges,
  selectedNodeIds,
  highlightedNodeIds,
  hoveredNodeId,
  setHoveredNodeId,
  setFocusedNodeWithHighlight,
  zoom,
  setZoom,
  panX,
  setPanX,
  panY,
  setPanY,
  rotation,
  setRotation,
  boxSelectRect,
  setBoxSelectRect,
  rotationIndicator,
  updateNodePosition,
  fps,
  setFps,
} from "../store.js";
import {
  LANG_COLORS,
  NODE_VS,
  NODE_FS,
  EDGE_VS,
  EDGE_FS,
  buildNodeProgram,
  buildEdgeProgram,
  screenToGraph,
} from "../lib/gl.js";


export function GraphCanvas() {
  let canvasRef;
  let overlayCanvasRef;
  let gl = null;
  let animFrame = 0;
  let frameCount = 0;
  let fpsTime = 0;

  let nodeProgram = null;
  let edgeProgram = null;

  // Interaction state
  let isPanning = false;
  let isRotating = false;
  let isDraggingNode = false;
  let isBoxSelecting = false;
  let dragNodeId = "";
  let boxStartX = 0;
  let boxStartY = 0;
  let ctrlDown = false;

  // Layout simulation
  let layoutRunning = false;
  let layoutFrames = 0;
  const MAX_LAYOUT_FRAMES = 180;
  const REPULSION = 2000;
  const ATTRACTION = 0.001;
  const GRAVITY = 0.001;
  const DAMPING = 0.9;
  const CELL_SIZE = 80;

  // Spatial grid
  let spatialGrid = new Map();
  let spatialGridBuilt = false;

  onMount(() => {
    gl = canvasRef.getContext("webgl2", { alpha: true, antialias: true });
    if (!gl) {
      console.error("WebGL2 not supported");
      return;
    }

    nodeProgram = buildNodeProgram(gl);
    edgeProgram = buildEdgeProgram(gl);

    resize();
    initEvents();
    initPositions();
    layoutRunning = true;
    renderLoop(0);
  });

  onCleanup(() => {
    cancelAnimationFrame(animFrame);
    cleanupEvents();
  });

  function resize() {
    if (!gl) return;
    const container = canvasRef.parentElement;
    if (!container) return;
    const w = container.clientWidth;
    const h = container.clientHeight;
    if (w <= 0 || h <= 0) return;
    const dpr = window.devicePixelRatio || 1;
    canvasRef.width = w * dpr;
    canvasRef.height = h * dpr;
    canvasRef.style.width = `${w}px`;
    canvasRef.style.height = `${h}px`;
    gl.viewport(0, 0, canvasRef.width, canvasRef.height);
    overlayCanvasRef.width = w * dpr;
    overlayCanvasRef.height = h * dpr;
    overlayCanvasRef.style.width = `${w}px`;
    overlayCanvasRef.style.height = `${h}px`;
  }

  function initEvents() {
    const ro = new ResizeObserver(resize);
    ro.observe(canvasRef.parentElement);
    canvasRef.addEventListener("mousemove", onMouseMove);
    canvasRef.addEventListener("mousedown", onMouseDown);
    canvasRef.addEventListener("mouseup", onMouseUp);
    canvasRef.addEventListener("wheel", onWheel, { passive: false });
    canvasRef.addEventListener("contextmenu", (e) => e.preventDefault());
    document.addEventListener("keydown", onKeyDown);
  }

  function cleanupEvents() {
    canvasRef.removeEventListener("mousemove", onMouseMove);
    canvasRef.removeEventListener("mousedown", onMouseDown);
    canvasRef.removeEventListener("mouseup", onMouseUp);
    canvasRef.removeEventListener("wheel", onWheel);
    document.removeEventListener("keydown", onKeyDown);
  }

  function getMousePos(e) {
    const rect = canvasRef.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    return [(e.clientX - rect.left) * dpr, (e.clientY - rect.top) * dpr];
  }

  function hitTest(sx, sy) {
    if (!spatialGridBuilt) return null;
    const [gx, gy] = screenToGraph(sx, sy, zoom(), panX(), panY(), rotation(), canvasRef.width, canvasRef.height);
    const threshold = 10 / zoom();
    const cellSize = CELL_SIZE;
    const cx = Math.floor(gx / cellSize);
    const cy = Math.floor(gy / cellSize);
    let closest = null;
    let closestDist = threshold;
    const nodes = nodesMap();

    for (let dx = -1; dx <= 1; dx++) {
      for (let dy = -1; dy <= 1; dy++) {
        const key = `${cx + dx},${cy + dy}`;
        const cell = spatialGrid.get(key);
        if (!cell) continue;
        for (const id of cell) {
          const node = nodes.get(id);
          if (!node || node._hidden) continue;
          const ddx = (node.x ?? 0) - gx;
          const ddy = (node.y ?? 0) - gy;
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

  function buildSpatialGrid() {
    spatialGrid.clear();
    const nodes = nodesMap();
    const cellSize = CELL_SIZE;
    for (const [id, node] of nodes) {
      if (node._hidden) continue;
      const cx = Math.floor((node.x ?? 0) / cellSize);
      const cy = Math.floor((node.y ?? 0) / cellSize);
      const key = `${cx},${cy}`;
      if (!spatialGrid.has(key)) spatialGrid.set(key, []);
      spatialGrid.get(key).push(id);
    }
    spatialGridBuilt = true;
  }

  function initPositions() {
    const nodes = nodesMap();
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
    spatialGridBuilt = false;
  }

  function stepLayout() {
    if (!layoutRunning) return;
    const nodes = nodesMap();
    const edgeList = edges();
    const cellSize = CELL_SIZE;

    // Build grid
    const grid = new Map();
    for (const [id, node] of nodes) {
      const cx = Math.floor((node.x ?? 0) / cellSize);
      const cy = Math.floor((node.y ?? 0) / cellSize);
      const key = `${cx},${cy}`;
      if (!grid.has(key)) grid.set(key, []);
      grid.get(key).push({ x: node.x ?? 0, y: node.y ?? 0, node: node });
    }

    // Repulsion
    for (const [, node] of nodes) {
      let fx = 0;
      let fy = 0;
      const cx = Math.floor((node.x ?? 0) / cellSize);
      const cy = Math.floor((node.y ?? 0) / cellSize);
      for (let dx = -1; dx <= 1; dx++) {
        for (let dy = -1; dy <= 1; dy++) {
          const key = `${cx + dx},${cy + dy}`;
          const cell = grid.get(key);
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

    // Attraction
    for (const edge of edgeList) {
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

    // Gravity + apply
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

    layoutFrames++;
    if (layoutFrames >= MAX_LAYOUT_FRAMES || totalEnergy < 50) {
      layoutRunning = false;
      for (const [, node] of nodes) {
        node.vx = 0;
        node.vy = 0;
      }
    }
  }

  function renderLoop(now) {
    animFrame = requestAnimationFrame(renderLoop);
    if (!gl) return;

    frameCount++;
    if (now - fpsTime >= 1000) {
      setFps(frameCount);
      frameCount = 0;
      fpsTime = now;
    }

    if (layoutRunning) {
      stepLayout();
    } else if (!spatialGridBuilt) {
      buildSpatialGrid();
    }

    const dpr = window.devicePixelRatio || 1;
    const vw = canvasRef.width;
    const vh = canvasRef.height;
    const nodes = nodesMap();
    const edgeList = edges();
    const highlighted = highlightedNodeIds();
    const selected = selectedNodeIds();

    gl.clearColor(0.06, 0.06, 0.08, 1.0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

    const nodeCount = nodes.size;
    if (nodeCount === 0) {
      renderOverlay();
      return;
    }

    const positions = new Float32Array(nodeCount * 2);
    const colors = new Float32Array(nodeCount * 4);
    const sizes = new Float32Array(nodeCount);
    const sels = new Float32Array(nodeCount);

    let ni = 0;
    for (const [id, node] of nodes) {
      positions[ni * 2] = node.x ?? 0;
      positions[ni * 2 + 1] = node.y ?? 0;
      const isSelected = selected.has(id);
      const isHighlighted = highlighted.has(id);
      const color = getNodeColor(node.language ?? "unknown", isHighlighted, isSelected);
      colors[ni * 4] = color[0];
      colors[ni * 4 + 1] = color[1];
      colors[ni * 4 + 2] = color[2];
      colors[ni * 4 + 3] = color[3];
      sizes[ni] = getNodeSize(node.connections ?? 0, isHighlighted, isSelected);
      sels[ni] = isSelected ? 1 : 0;
      (node)._renderIndex = ni;
      ni++;
    }

    // Edges
    const arr = [];
    for (const edge of edgeList) {
      const src = nodes.get(edge.source);
      const tgt = nodes.get(edge.target);
      // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
      const si = src?._renderIndex;
      // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
      const ti = tgt?._renderIndex;
      if (si === undefined || ti === undefined) continue;
      const ec = getEdgeColor(edge.kind ?? "", highlighted.has(edge.source) || highlighted.has(edge.target));
      edgeData.push(
        positions[si * 2], positions[si * 2 + 1], ec[0], ec[1], ec[2],
        positions[ti * 2], positions[ti * 2 + 1], ec[3], ec[4], ec[5],
      );
    }

    // Draw edges
    if (edgeProgram && edgeData.length > 0) {
      gl.useProgram(edgeProgram);
      const edgeFloat = new Float32Array(edgeData);
      const eBuf = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, eBuf);
      gl.bufferData(gl.ARRAY_BUFFER, edgeFloat, gl.DYNAMIC_DRAW);
      const uRes = gl.getUniformLocation(edgeProgram, "u_resolution");
      const uZoom = gl.getUniformLocation(edgeProgram, "u_zoom");
      const uPan = gl.getUniformLocation(edgeProgram, "u_pan");
      const uRot = gl.getUniformLocation(edgeProgram, "u_rotation");
      if (uRes && uZoom && uPan && uRot) {
        gl.uniform2f(uRes, vw, vh);
        gl.uniform1f(uZoom, zoom());
        gl.uniform2f(uPan, panX(), panY());
        gl.uniform1f(uRot, rotation());
      }
      const loc0 = gl.getAttribLocation(edgeProgram, "a_position");
      const loc1 = gl.getAttribLocation(edgeProgram, "a_color");
      gl.enableVertexAttribArray(loc0);
      gl.enableVertexAttribArray(loc1);
      gl.vertexAttribPointer(loc0, 2, gl.FLOAT, false, 20, 0);
      gl.vertexAttribPointer(loc1, 3, gl.FLOAT, false, 20, 8);
      gl.drawArrays(gl.LINES, 0, edgeData.length / 5);
      gl.deleteBuffer(eBuf);
    }

    // Draw nodes
    if (nodeProgram) {
      gl.useProgram(nodeProgram);
      const pBuf = gl.createBuffer();
      const cBuf = gl.createBuffer();
      const sBuf = gl.createBuffer();
      const selBuf = gl.createBuffer();

      gl.bindBuffer(gl.ARRAY_BUFFER, pBuf);
      gl.bufferData(gl.ARRAY_BUFFER, positions.subarray(0, nodeCount * 2), gl.DYNAMIC_DRAW);
      const loc0 = gl.getAttribLocation(nodeProgram, "a_position");
      gl.enableVertexAttribArray(loc0);
      gl.vertexAttribPointer(loc0, 2, gl.FLOAT, false, 0, 0);

      gl.bindBuffer(gl.ARRAY_BUFFER, cBuf);
      gl.bufferData(gl.ARRAY_BUFFER, colors.subarray(0, nodeCount * 4), gl.DYNAMIC_DRAW);
      const loc1 = gl.getAttribLocation(nodeProgram, "a_color");
      gl.enableVertexAttribArray(loc1);
      gl.vertexAttribPointer(loc1, 4, gl.FLOAT, false, 0, 0);

      gl.bindBuffer(gl.ARRAY_BUFFER, sBuf);
      gl.bufferData(gl.ARRAY_BUFFER, sizes.subarray(0, nodeCount), gl.DYNAMIC_DRAW);
      const loc2 = gl.getAttribLocation(nodeProgram, "a_size");
      gl.enableVertexAttribArray(loc2);
      gl.vertexAttribPointer(loc2, 1, gl.FLOAT, false, 0, 0);

      gl.bindBuffer(gl.ARRAY_BUFFER, selBuf);
      gl.bufferData(gl.ARRAY_BUFFER, sels.subarray(0, nodeCount), gl.DYNAMIC_DRAW);
      const loc3 = gl.getAttribLocation(nodeProgram, "a_selected");
      gl.enableVertexAttribArray(loc3);
      gl.vertexAttribPointer(loc3, 1, gl.FLOAT, false, 0, 0);

      const uRes = gl.getUniformLocation(nodeProgram, "u_resolution");
      const uZoom = gl.getUniformLocation(nodeProgram, "u_zoom");
      const uPan = gl.getUniformLocation(nodeProgram, "u_pan");
      const uRot = gl.getUniformLocation(nodeProgram, "u_rotation");
      if (uRes && uZoom && uPan && uRot) {
        gl.uniform2f(uRes, vw, vh);
        gl.uniform1f(uZoom, zoom());
        gl.uniform2f(uPan, panX(), panY());
        gl.uniform1f(uRot, rotation());
      }

      gl.drawArrays(gl.POINTS, 0, nodeCount);
      [pBuf, cBuf, sBuf, selBuf].forEach((b) => gl.deleteBuffer(b));
    }

    renderOverlay();
  }

  function renderOverlay() {
    const ctx = overlayCanvasRef.getContext("2d");
    if (!ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const w = overlayCanvasRef.width;
    const h = overlayCanvasRef.height;
    ctx.clearRect(0, 0, w, h);

    const bs = boxSelectRect();
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

    const rot = rotationIndicator();
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
  }

  function getNodeColor(lang, highlighted, selected) {
    const base = LANG_COLORS[lang] ?? LANG_COLORS.unknown;
    if (selected) return [1, 1, 1, 1];
    if (highlighted) return [1, 0.85, 0.18, 1];
    return base;
  }

  function getNodeSize(connections, highlighted, selected) {
    const base = 6 + Math.min(connections * 0.5, 12);
    if (selected) return base * 1.6;
    if (highlighted) return base * 1.3;
    return base;
  }

  function getEdgeColor(kind, highlighted) {
    if (highlighted) return [0.9, 0.8, 0.2, 0.9, 0.8, 0.2];
    const kinds = {
      direct: [0.25, 0.25, 0.3, 0.25, 0.3, 0.35],
      indirect: [0.8, 0.3, 0.3, 0.8, 0.3, 0.3],
      macro: [0.7, 0.6, 0.2, 0.7, 0.6, 0.2],
    };
    return kinds[kind] ?? [0.3, 0.3, 0.35, 0.3, 0.3, 0.35];
  }

  function onKeyDown(e) {
    ctrlDown = e.ctrlKey || e.metaKey;
    if (e.key === "Escape") {
      setFocusedNodeWithHighlight(null);
    }
  }

  function onMouseMove(e) {
    const [sx, sy] = getMousePos(e);

    if (isDraggingNode && dragNodeId) {
      const [gx, gy] = screenToGraph(sx, sy, zoom(), panX(), panY(), rotation(), canvasRef.width, canvasRef.height);
      updateNodePosition(dragNodeId, gx, gy);
      return;
    }

    if (isRotating) {
      setRotation(rotation() + e.movementX * 0.005);
      return;
    }

    if (isPanning) {
      setPanX(panX() + e.movementX / zoom());
      setPanY(panY() - e.movementY / zoom());
      return;
    }

    if (isBoxSelecting) {
      setBoxSelectRect({ x1: boxStartX, y1: boxStartY, x2: e.clientX, y2: e.clientY });
      return;
    }

    const nodeId = hitTest(sx, sy);
    if (nodeId !== hoveredNodeId()) {
      setHoveredNodeId(nodeId);
      canvasRef.style.cursor = nodeId ? "pointer" : "default";
    }
  }

  function onMouseDown(e) {
    if (e.button !== 0) return;
    const [sx, sy] = getMousePos(e);
    const nodeId = hitTest(sx, sy);

    if (nodeId) {
      if (ctrlDown) {
        const sel = new Set(selectedNodeIds());
        if (sel.has(nodeId)) sel.delete(nodeId);
        else sel.add(nodeId);
        const hl = new Set(sel);
        for (const id of sel) hl.add(id);
        setFocusedNodeWithHighlight(nodeId);
      } else {
        setFocusedNodeWithHighlight(nodeId);
        isDraggingNode = true;
        dragNodeId = nodeId;
      }
      return;
    }

    if (ctrlDown) {
      isRotating = true;
      return;
    }

    if (e.shiftKey) {
      isBoxSelecting = true;
      boxStartX = e.clientX;
      boxStartY = e.clientY;
      return;
    }

    isPanning = true;
  }

  function onMouseUp(e) {
    if (isBoxSelecting) {
      const rect = boxSelectRect();
      if (rect && Math.abs(rect.x2 - rect.x1) > 5 && Math.abs(rect.y2 - rect.y1) > 5) {
        const dpr = window.devicePixelRatio || 1;
        const x1 = Math.min(rect.x1, rect.x2) * dpr;
        const y1 = Math.min(rect.y1, rect.y2) * dpr;
        const x2 = Math.max(rect.x1, rect.x2) * dpr;
        const y2 = Math.max(rect.y1, rect.y2) * dpr;
        const [gx1, gy1] = screenToGraph(x1, y1, zoom(), panX(), panY(), rotation(), canvasRef.width, canvasRef.height);
        const [gx2, gy2] = screenToGraph(x2, y2, zoom(), panX(), panY(), rotation(), canvasRef.width, canvasRef.height);
        const nodes = nodesMap();
        const arr = [];
        for (const [id, node] of nodes) {
          const nx = node.x ?? 0;
          const ny = node.y ?? 0;
          if (nx >= Math.min(gx1, gx2) && nx <= Math.max(gx1, gx2) && ny >= Math.min(gy1, gy2) && ny <= Math.max(gy1, gy2)) {
            inBox.push(id);
          }
        }
        if (inBox.length > 0) {
          const sel = ctrlDown ? new Set(selectedNodeIds()) : new Set();
          for (const id of inBox) sel.add(id);
          setFocusedNodeWithHighlight(inBox[0] ?? null);
        }
      }
      setBoxSelectRect(null);
    }

    isPanning = false;
    isRotating = false;
    isDraggingNode = false;
    isBoxSelecting = false;
    dragNodeId = "";
  }

  function onWheel(e) {
    e.preventDefault();
    const zoomFactor = e.deltaY > 0 ? 0.9 : 1.1;
    const [mx, my] = getMousePos(e);
    const [gx, gy] = screenToGraph(mx, my, zoom(), panX(), panY(), rotation(), canvasRef.width, canvasRef.height);
    let newZoom = Math.max(0.1, Math.min(10, zoom() * zoomFactor));
    setZoom(newZoom);
    const [newGx, newGy] = screenToGraph(mx, my, newZoom, panX(), panY(), rotation(), canvasRef.width, canvasRef.height);
    setPanX(panX() + (newGx - gx));
    setPanY(panY() + (newGy - gy));
  }

  return (
    <div class="relative w-full h-full">
      <canvas ref={canvasRef} id="graph-canvas" class="block w-full h-full" />
      <canvas
        ref={overlayCanvasRef}
        id="graph-overlay"
        class="absolute top-0 left-0 w-full h-full pointer-events-none z-10"
      />
    </div>
  );
}

// Benchmark comment