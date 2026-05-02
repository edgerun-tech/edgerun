"use client";

import { useEffect, useRef, useState, useCallback } from "react";
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
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const overlayCanvasRef = useRef<HTMLCanvasElement>(null);
  const rendererRef = useRef<any>(null);
  const animFrameRef = useRef(0);
  const layoutStateRef = useRef(createLayoutState());
  const spatialGridRef = useRef(new Map());
  const spatialGridBuiltRef = useRef(false);

  const [zoom, setZoom] = useState(1);
  const [panX, setPanX] = useState(0);
  const [panY, setPanY] = useState(0);
  const [rotation, setRotation] = useState(0);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [highlightedIds, setHighlightedIds] = useState<Set<string>>(new Set());
  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const [boxSelectRect, setBoxSelectRect] = useState<any>(null);
  const [fps, setFps] = useState(0);

  // Interaction state refs
  const isPanningRef = useRef(false);
  const isRotatingRef = useRef(false);
  const isDraggingNodeRef = useRef(false);
  const isBoxSelectingRef = useRef(false);
  const dragNodeIdRef = useRef("");
  const boxStartXRef = useRef(0);
  const boxStartYRef = useRef(0);
  const ctrlDownRef = useRef(false);
  const frameCountRef = useRef(0);
  const fpsTimeRef = useRef(0);

  const runtimeMode = props.runtimeMode ?? false;

  useEffect(() => {
    const canvas = canvasRef.current;
    const overlayCanvas = overlayCanvasRef.current;
    if (!canvas || !overlayCanvas) return;

    const renderer = createGraphRenderer(canvas, overlayCanvas);
    rendererRef.current = renderer;
    renderer.resize();

    const ro = new ResizeObserver(() => renderer.resize());
    ro.observe(canvas.parentElement!);

    initEvents(canvas);
    initPositions();
    layoutStateRef.current.running = true;
    renderLoop(0);

    return () => {
      cancelAnimationFrame(animFrameRef.current);
      renderer.destroy();
      ro.disconnect();
      cleanupEvents(canvas);
    };
  }, []);

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

  function initEvents(canvas: HTMLCanvasElement) {
    canvas.addEventListener("mousemove", onMouseMove);
    canvas.addEventListener("mousedown", onMouseDown);
    canvas.addEventListener("mouseup", onMouseUp);
    canvas.addEventListener("wheel", onWheel, { passive: false });
    canvas.addEventListener("contextmenu", (e) => e.preventDefault());
    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("keyup", onKeyUp);
  }

  function cleanupEvents(canvas: HTMLCanvasElement) {
    canvas.removeEventListener("mousemove", onMouseMove);
    canvas.removeEventListener("mousedown", onMouseDown);
    canvas.removeEventListener("mouseup", onMouseUp);
    canvas.removeEventListener("wheel", onWheel);
    document.removeEventListener("keydown", onKeyDown);
    document.removeEventListener("keyup", onKeyUp);
  }

  function getMousePos(e: MouseEvent, canvas: HTMLCanvasElement) {
    const rect = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    return [(e.clientX - rect.left) * dpr, (e.clientY - rect.top) * dpr];
  }

  const onKeyDown = useCallback((e: KeyboardEvent) => {
    ctrlDownRef.current = e.ctrlKey || e.metaKey;
    if (e.key === "Escape") {
      setSelectedIds(new Set());
      setHighlightedIds(new Set());
      props.onNodeSelect?.(null);
    }
  }, []);

  const onKeyUp = useCallback((e: KeyboardEvent) => {
    ctrlDownRef.current = e.ctrlKey || e.metaKey;
  }, []);

  const onMouseMove = useCallback((e: MouseEvent) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const [sx, sy] = getMousePos(e, canvas);

    if (isDraggingNodeRef.current && dragNodeIdRef.current) {
      const dpr = window.devicePixelRatio || 1;
      const gx = (sx - canvas.width * 0.5) / zoom + panX;
      const gy = (canvas.height * 0.5 - sy) / zoom + panY;
      const node = props.graph?.nodes?.get(dragNodeIdRef.current);
      if (node) {
        node.x = gx;
        node.y = gy;
      }
      return;
    }

    if (isRotatingRef.current) {
      setRotation((r) => r + e.movementX * 0.005);
      return;
    }

    if (isPanningRef.current) {
      setPanX((p) => p + e.movementX / zoom);
      setPanY((p) => p - e.movementY / zoom);
      return;
    }

    if (isBoxSelectingRef.current) {
      setBoxSelectRect({ x1: boxStartXRef.current, y1: boxStartYRef.current, x2: e.clientX, y2: e.clientY });
      return;
    }

    const nodeId = hitTest(
      sx, sy,
      props.graph?.nodes,
      { zoom, panX, panY, rotation },
      { width: canvas.width, height: canvas.height },
      spatialGridRef.current
    );

    if (nodeId !== hoveredId) {
      setHoveredId(nodeId);
      canvas.style.cursor = nodeId ? "pointer" : "default";
      props.onNodeHover?.(nodeId);
    }
  }, [zoom, panX, panY, rotation, hoveredId, props.graph]);

  const onMouseDown = useCallback((e: MouseEvent) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    if (e.button !== 0) return;

    const [sx, sy] = getMousePos(e, canvas);
    const nodeId = hitTest(
      sx, sy,
      props.graph?.nodes,
      { zoom, panX, panY, rotation },
      { width: canvas.width, height: canvas.height },
      spatialGridRef.current
    );

    if (nodeId) {
      if (ctrlDownRef.current) {
        const sel = new Set(selectedIds);
        if (sel.has(nodeId)) sel.delete(nodeId);
        else sel.add(nodeId);
        setSelectedIds(sel);
        setHighlightedIds(sel);
      } else {
        setSelectedIds(new Set([nodeId]));
        setHighlightedIds(new Set([nodeId]));
        isDraggingNodeRef.current = true;
        dragNodeIdRef.current = nodeId;
      }
      props.onNodeSelect?.(nodeId);
      return;
    }

    if (ctrlDownRef.current) {
      isRotatingRef.current = true;
      return;
    }

    if (e.shiftKey) {
      isBoxSelectingRef.current = true;
      boxStartXRef.current = e.clientX;
      boxStartYRef.current = e.clientY;
      return;
    }

    isPanningRef.current = true;
  }, [zoom, panX, panY, rotation, selectedIds, props.graph]);

  const onMouseUp = useCallback((e: MouseEvent) => {
    if (isBoxSelectingRef.current) {
      const rect = boxSelectRect;
      if (rect && Math.abs(rect.x2 - rect.x1) > 5 && Math.abs(rect.y2 - rect.y1) > 5) {
        const dpr = window.devicePixelRatio || 1;
        const inBox = boxSelect(rect, props.graph?.nodes, { zoom, panX, panY, rotation }, { width: canvasRef.current!.width, height: canvasRef.current!.height }, dpr);
        if (inBox.length > 0) {
          const sel = ctrlDownRef.current ? new Set(selectedIds) : new Set();
          for (const id of inBox) sel.add(id);
          setSelectedIds(sel);
          setHighlightedIds(sel);
          props.onNodeSelect?.(inBox[0] ?? null);
        }
      }
      setBoxSelectRect(null);
    }

    isPanningRef.current = false;
    isRotatingRef.current = false;
    isDraggingNodeRef.current = false;
    isBoxSelectingRef.current = false;
    dragNodeIdRef.current = "";
  }, [zoom, panX, panY, rotation, selectedIds, boxSelectRect, props.graph]);

  const onWheel = useCallback((e: WheelEvent) => {
    e.preventDefault();
    const zoomFactor = e.deltaY > 0 ? 0.9 : 1.1;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const [mx, my] = getMousePos(e, canvas);
    const newZoom = Math.max(0.1, Math.min(10, zoom * zoomFactor));
    setZoom(newZoom);
  }, [zoom]);

  function renderLoop(now: number) {
    animFrameRef.current = requestAnimationFrame(renderLoop) as any;

    frameCountRef.current++;
    if (now - fpsTimeRef.current >= 1000) {
      setFps(frameCountRef.current);
      frameCountRef.current = 0;
      fpsTimeRef.current = now;
    }

    // Layout step
    if (layoutStateRef.current.running) {
      stepLayout(layoutStateRef.current, props.graph?.nodes, props.graph?.edges);
    } else if (!spatialGridBuiltRef.current) {
      spatialGridRef.current = buildSpatialGrid(props.graph?.nodes);
      spatialGridBuiltRef.current = true;
    }

    // Build highlighted set
    const hl = new Set([...highlightedIds]);
    if (hoveredId) hl.add(hoveredId);

    // Render
    rendererRef.current?.render({
      nodes: props.graph?.nodes,
      edges: props.graph?.edges,
      selectedIds,
      highlightedIds: hl,
      zoom,
      panX,
      panY,
      rotation,
      boxSelectRect,
      rotationIndicator: isRotatingRef.current ? rotation : null,
      runtimeMode,
    });
  }

  return (
    <div className="relative w-full h-full">
      <canvas ref={canvasRef} id="xray-canvas" className="block w-full h-full" />
      <canvas
        ref={overlayCanvasRef}
        id="xray-overlay"
        className="absolute top-0 left-0 w-full h-full pointer-events-none z-10"
      />
    </div>
  );
}
