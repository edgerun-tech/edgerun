"use client";

import { onMount, onCleanup, createSignal, createEffect } from "solid-js";
import { createGraphRenderer } from "../../lib/xray/render/GraphRenderer.js";
import { createLayoutState, stepLayout, buildSpatialGrid } from "../../lib/xray/layout/forceLayout.js";
import { hitTest, boxSelect } from "../../lib/xray/input/hitTest.js";

/**
 * XrayViewport - Center graph viewport component for EdgeRun dashboard.
 * Replaces the globe component as the primary visualization.
 * Uses extracted renderer, layout, and input modules.
 */
export default function XrayViewport(props: {
  graph: any;
  runtimeMode?: boolean;
  onNodeSelect?: (nodeId: string | null) => void;
  onNodeHover?: (nodeId: string | null) => void;
}) {
  let canvasRef: HTMLCanvasElement;
  let overlayCanvasRef: HTMLCanvasElement;
  let renderer: any = null;
  let animFrame = 0;
  let layoutState = createLayoutState();
  let spatialGrid = new Map();
  let spatialGridBuilt = false;

  const [zoom, setZoom] = createSignal(1);
  const [panX, setPanX] = createSignal(0);
  const [panY, setPanY] = createSignal(0);
  const [rotation, setRotation] = createSignal(0);
  const [selectedIds, setSelectedIds] = createSignal(new Set<string>());
  const [highlightedIds, setHighlightedIds] = createSignal(new Set<string>());
  const [hoveredId, setHoveredId] = createSignal<string | null>(null);
  const [boxSelectRect, setBoxSelectRect] = createSignal<any>(null);
  const [fps, setFps] = createSignal(0);

  // Interaction state
  let isPanning = false;
  let isRotating = false;
  let isDraggingNode = false;
  let isBoxSelecting = false;
  let dragNodeId = "";
  let boxStartX = 0;
  let boxStartY = 0;
  let ctrlDown = false;
  let frameCount = 0;
  let fpsTime = 0;

  const runtimeMode = () => props.runtimeMode ?? false;

  onMount(() => {
    renderer = createGraphRenderer(canvasRef, overlayCanvasRef);
    renderer.resize();

    const ro = new ResizeObserver(() => renderer.resize());
    ro.observe(canvasRef.parentElement!);

    initEvents();
    initPositions();
    layoutState.running = true;
    renderLoop(0);

    onCleanup(() => {
      cancelAnimationFrame(animFrame);
      renderer.destroy();
      ro.disconnect();
      cleanupEvents();
    });
  });

  function initPositions() {
    const nodes = props.graph?.nodes;
    if (!nodes) return;
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

  function initEvents() {
    canvasRef.addEventListener("mousemove", onMouseMove);
    canvasRef.addEventListener("mousedown", onMouseDown);
    canvasRef.addEventListener("mouseup", onMouseUp);
    canvasRef.addEventListener("wheel", onWheel, { passive: false });
    canvasRef.addEventListener("contextmenu", (e) => e.preventDefault());
    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("keyup", onKeyUp);
  }

  function cleanupEvents() {
    canvasRef.removeEventListener("mousemove", onMouseMove);
    canvasRef.removeEventListener("mousedown", onMouseDown);
    canvasRef.removeEventListener("mouseup", onMouseUp);
    canvasRef.removeEventListener("wheel", onWheel);
    document.removeEventListener("keydown", onKeyDown);
    document.removeEventListener("keyup", onKeyUp);
  }

  function getMousePos(e: MouseEvent) {
    const rect = canvasRef.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    return [(e.clientX - rect.left) * dpr, (e.clientY - rect.top) * dpr];
  }

  function onKeyDown(e: KeyboardEvent) {
    ctrlDown = e.ctrlKey || e.metaKey;
    if (e.key === "Escape") {
      setSelectedIds(new Set());
      setHighlightedIds(new Set());
      props.onNodeSelect?.(null);
    }
  }

  function onKeyUp(e: KeyboardEvent) {
    ctrlDown = e.ctrlKey || e.metaKey;
  }

  function onMouseMove(e: MouseEvent) {
    const [sx, sy] = getMousePos(e);

    if (isDraggingNode && dragNodeId) {
      const dpr = window.devicePixelRatio || 1;
      const gx = (sx - canvasRef.width * 0.5) / zoom() + panX();
      const gy = (canvasRef.height * 0.5 - sy) / zoom() + panY();
      const node = props.graph?.nodes?.get(dragNodeId);
      if (node) {
        node.x = gx;
        node.y = gy;
      }
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

    const nodeId = hitTest(
      sx, sy,
      props.graph?.nodes,
      { zoom: zoom(), panX: panX(), panY: panY(), rotation: rotation() },
      { width: canvasRef.width, height: canvasRef.height },
      spatialGrid
    );

    if (nodeId !== hoveredId()) {
      setHoveredId(nodeId);
      canvasRef.style.cursor = nodeId ? "pointer" : "default";
      props.onNodeHover?.(nodeId);
    }
  }

  function onMouseDown(e: MouseEvent) {
    if (e.button !== 0) return;
    const [sx, sy] = getMousePos(e);
    const nodeId = hitTest(
      sx, sy,
      props.graph?.nodes,
      { zoom: zoom(), panX: panX(), panY: panY(), rotation: rotation() },
      { width: canvasRef.width, height: canvasRef.height },
      spatialGrid
    );

    if (nodeId) {
      if (ctrlDown) {
        const sel = new Set(selectedIds());
        if (sel.has(nodeId)) sel.delete(nodeId);
        else sel.add(nodeId);
        setSelectedIds(sel);
        setHighlightedIds(sel);
      } else {
        setSelectedIds(new Set([nodeId]));
        setHighlightedIds(new Set([nodeId]));
        isDraggingNode = true;
        dragNodeId = nodeId;
      }
      props.onNodeSelect?.(nodeId);
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

  function onMouseUp(e: MouseEvent) {
    if (isBoxSelecting) {
      const rect = boxSelectRect();
      if (rect && Math.abs(rect.x2 - rect.x1) > 5 && Math.abs(rect.y2 - rect.y1) > 5) {
        const dpr = window.devicePixelRatio || 1;
        const inBox = boxSelect(rect, props.graph?.nodes, { zoom: zoom(), panX: panX(), panY: panY(), rotation: rotation() }, { width: canvasRef.width, height: canvasRef.height }, dpr);
        if (inBox.length > 0) {
          const sel = ctrlDown ? new Set(selectedIds()) : new Set();
          for (const id of inBox) sel.add(id);
          setSelectedIds(sel);
          setHighlightedIds(sel);
          props.onNodeSelect?.(inBox[0] ?? null);
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

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    const zoomFactor = e.deltaY > 0 ? 0.9 : 1.1;
    const [mx, my] = getMousePos(e);
    const newZoom = Math.max(0.1, Math.min(10, zoom() * zoomFactor));

    setZoom(newZoom);
    // Adjust pan to zoom towards mouse position
    const gx = (mx - canvasRef.width * 0.5) / newZoom + panX();
    const gy = (canvasRef.height * 0.5 - my) / newZoom + panY();
    setPanX(panX() + gx - ((mx - canvasRef.width * 0.5) / zoom() + panX()));
    setPanY(panY() + gy - ((canvasRef.height * 0.5 - my) / zoom() + panY()));
  }

  function renderLoop(now: number) {
    animFrame = requestAnimationFrame(renderLoop);

    frameCount++;
    if (now - fpsTime >= 1000) {
      setFps(frameCount);
      frameCount = 0;
      fpsTime = now;
    }

    // Layout step
    if (layoutState.running) {
      stepLayout(layoutState, props.graph?.nodes, props.graph?.edges);
    } else if (!spatialGridBuilt) {
      spatialGrid = buildSpatialGrid(props.graph?.nodes);
      spatialGridBuilt = true;
    }

    // Render
    renderer.render({
      nodes: props.graph?.nodes,
      edges: props.graph?.edges,
      selectedIds: selectedIds(),
      highlightedIds: new Set([...highlightedIds(), ...(hoveredId() ? [hoveredId()!] : [])]),
      zoom: zoom(),
      panX: panX(),
      panY: panY(),
      rotation: rotation(),
      boxSelectRect: boxSelectRect(),
      rotationIndicator: isRotating ? rotation() : null,
      runtimeMode: runtimeMode(),
    });
  }

  return (
    <div class="relative w-full h-full">
      <canvas ref={canvasRef!} id="xray-canvas" class="block w-full h-full" />
      <canvas
        ref={overlayCanvasRef!}
        id="xray-overlay"
        class="absolute top-0 left-0 w-full h-full pointer-events-none z-10"
      />
    </div>
  );
}
