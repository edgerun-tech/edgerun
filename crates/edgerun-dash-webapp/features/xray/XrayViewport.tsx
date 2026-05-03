"use client";

import { useEffect, useRef, useState, useCallback } from "react";
import { createGraphRenderer } from "../../lib/xray/render/GraphRenderer.js";
import { createLayoutState, stepLayout, buildSpatialGrid } from "../../lib/xray/layout/forceLayout.js";
import { hitTest, boxSelect } from "../../lib/xray/input/hitTest.js";

const LAYER_ORDER = ["ui", "api", "runtime", "protocol", "crypto", "storage", "network", "agent"];

/**
 * XrayViewport - Center graph viewport component for EdgeRun dashboard.
 * Replaces the globe component as the primary visualization.
 * Uses extracted renderer, layout, and input modules.
 */
export default function XrayViewport(props: {
  graph: any;
  runtimeMode?: boolean;
  layoutType?: "force" | "grid" | "hierarchical";
  externalHighlightedIds?: Set<string>;
  viewportCommand?: any;
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
  const graphVersionRef = useRef(0);
  const lastLayoutRef = useRef<string | null>(null);
  const lastCommandNonceRef = useRef<any>(null);

  const [zoom, setZoom] = useState(1);
  const [panX, setPanX] = useState(0);
  const [panY, setPanY] = useState(0);
  const [rotation, setRotation] = useState(0);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [highlightedIds, setHighlightedIds] = useState<Set<string>>(new Set());
  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const [boxSelectRect, setBoxSelectRect] = useState<any>(null);

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
  const layoutType = props.layoutType ?? "force";

  useEffect(() => {
    const canvas = canvasRef.current;
    const overlayCanvas = overlayCanvasRef.current;
    if (!canvas || !overlayCanvas) return;

    const renderer = createGraphRenderer(canvas, overlayCanvas);
    rendererRef.current = renderer;
    renderer.resize();

    const ro = new ResizeObserver(() => {
      renderer.resize();
      fitView();
    });
    ro.observe(canvas.parentElement!);

    initEvents(canvas);
    renderLoop(0);

    return () => {
      cancelAnimationFrame(animFrameRef.current);
      renderer.destroy();
      ro.disconnect();
      cleanupEvents(canvas);
    };
  }, []);

  useEffect(() => {
    if (!props.graph?.nodes) return;
    graphVersionRef.current++;
    spatialGridBuiltRef.current = false;
    applyLayout(layoutType, true);
    requestAnimationFrame(() => fitView());
  }, [props.graph]);

  useEffect(() => {
    if (!props.graph?.nodes) return;
    if (lastLayoutRef.current === layoutType) return;
    lastLayoutRef.current = layoutType;
    applyLayout(layoutType, false);
    requestAnimationFrame(() => fitView());
  }, [layoutType, props.graph]);

  useEffect(() => {
    const external = props.externalHighlightedIds ?? new Set<string>();
    setHighlightedIds(new Set(external));
  }, [props.externalHighlightedIds]);

  useEffect(() => {
    const cmd = props.viewportCommand;
    if (!cmd || lastCommandNonceRef.current === cmd.nonce) return;
    lastCommandNonceRef.current = cmd.nonce;

    switch (cmd.type) {
      case "focus_node":
        focusNode(cmd.nodeId);
        break;
      case "highlight_nodes":
        setHighlightedIds(new Set(cmd.nodeIds ?? []));
        break;
      case "fit_view":
      case "reset_view":
        fitView();
        break;
      default:
        break;
    }
  }, [props.viewportCommand]);

  function initEvents(canvas: HTMLCanvasElement) {
    canvas.addEventListener("mousemove", onMouseMove);
    canvas.addEventListener("mousedown", onMouseDown);
    canvas.addEventListener("mouseup", onMouseUp);
    canvas.addEventListener("mouseleave", onMouseUp);
    canvas.addEventListener("wheel", onWheel, { passive: false });
    canvas.addEventListener("contextmenu", (e) => e.preventDefault());
    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("keyup", onKeyUp);
  }

  function cleanupEvents(canvas: HTMLCanvasElement) {
    canvas.removeEventListener("mousemove", onMouseMove);
    canvas.removeEventListener("mousedown", onMouseDown);
    canvas.removeEventListener("mouseup", onMouseUp);
    canvas.removeEventListener("mouseleave", onMouseUp);
    canvas.removeEventListener("wheel", onWheel);
    document.removeEventListener("keydown", onKeyDown);
    document.removeEventListener("keyup", onKeyUp);
  }

  function applyLayout(layout: string, initial: boolean) {
    const nodes = props.graph?.nodes;
    const edges = props.graph?.edges ?? [];
    if (!nodes || nodes.size === 0) return;

    if (layout === "grid") {
      applyGridLayout(nodes);
      layoutStateRef.current.running = false;
    } else if (layout === "hierarchical") {
      applyLayerLayout(nodes);
      layoutStateRef.current.running = false;
    } else {
      ensureRandomPositions(nodes, edges, initial);
      layoutStateRef.current = createLayoutState({ MAX_LAYOUT_FRAMES: initial ? 240 : 160 });
      layoutStateRef.current.running = true;
    }

    spatialGridBuiltRef.current = false;
  }

  function ensureRandomPositions(nodes: Map<string, any>, edges: any[], initial: boolean) {
    const count = nodes.size;
    const spread = Math.max(260, Math.sqrt(count) * 42);
    let index = 0;
    for (const [, node] of nodes) {
      const missing = node.x === undefined || node.y === undefined || (node.x === 0 && node.y === 0);
      if (initial || missing) {
        const angle = index * 2.399963229728653;
        const radius = spread * Math.sqrt((index + 1) / Math.max(count, 1));
        node.x = Math.cos(angle) * radius + (Math.random() - 0.5) * 24;
        node.y = Math.sin(angle) * radius + (Math.random() - 0.5) * 24;
        node.vx = 0;
        node.vy = 0;
      }
      index++;
    }
  }

  function applyGridLayout(nodes: Map<string, any>) {
    const list = Array.from(nodes.values()).sort((a, b) => String(a.layer ?? a.language ?? "").localeCompare(String(b.layer ?? b.language ?? "")) || String(a.label ?? a.id).localeCompare(String(b.label ?? b.id)));
    const cols = Math.ceil(Math.sqrt(list.length));
    const spacing = 56;
    const offsetX = -((cols - 1) * spacing) / 2;
    const rows = Math.ceil(list.length / cols);
    const offsetY = ((rows - 1) * spacing) / 2;
    list.forEach((node, index) => {
      node.x = offsetX + (index % cols) * spacing;
      node.y = offsetY - Math.floor(index / cols) * spacing;
      node.vx = 0;
      node.vy = 0;
    });
  }

  function applyLayerLayout(nodes: Map<string, any>) {
    const groups = new Map<string, any[]>();
    for (const [, node] of nodes) {
      const key = node.layer || node.language || "unknown";
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key)!.push(node);
    }

    const keys = Array.from(groups.keys()).sort((a, b) => {
      const ai = LAYER_ORDER.indexOf(a);
      const bi = LAYER_ORDER.indexOf(b);
      if (ai !== -1 || bi !== -1) return (ai === -1 ? 999 : ai) - (bi === -1 ? 999 : bi);
      return a.localeCompare(b);
    });

    const layerGap = 150;
    const nodeGap = 42;
    const startX = -((keys.length - 1) * layerGap) / 2;
    keys.forEach((key, layerIndex) => {
      const group = groups.get(key) ?? [];
      group.sort((a, b) => String(a.label ?? a.id).localeCompare(String(b.label ?? b.id)));
      const startY = ((group.length - 1) * nodeGap) / 2;
      group.forEach((node, i) => {
        node.x = startX + layerIndex * layerGap;
        node.y = startY - i * nodeGap;
        node.vx = 0;
        node.vy = 0;
      });
    });
  }

  function getBounds(nodes = props.graph?.nodes) {
    if (!nodes || nodes.size === 0) return null;
    let minX = Infinity;
    let minY = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;
    for (const [, node] of nodes) {
      const x = node.x ?? 0;
      const y = node.y ?? 0;
      minX = Math.min(minX, x);
      minY = Math.min(minY, y);
      maxX = Math.max(maxX, x);
      maxY = Math.max(maxY, y);
    }
    if (!Number.isFinite(minX)) return null;
    return { minX, minY, maxX, maxY };
  }

  function fitView() {
    const canvas = canvasRef.current;
    const bounds = getBounds();
    if (!canvas || !bounds) return;
    const width = Math.max(bounds.maxX - bounds.minX, 80);
    const height = Math.max(bounds.maxY - bounds.minY, 80);
    const margin = 0.72;
    const nextZoom = Math.max(0.18, Math.min(8, Math.min(canvas.width / width, canvas.height / height) * margin));
    setZoom(nextZoom);
    setPanX((bounds.minX + bounds.maxX) / 2);
    setPanY((bounds.minY + bounds.maxY) / 2);
    setRotation(0);
  }

  function focusNode(nodeId: string) {
    const node = props.graph?.nodes?.get(nodeId);
    if (!node) return;
    setSelectedIds(new Set([nodeId]));
    setHighlightedIds((prev) => new Set([...prev, nodeId]));
    setPanX(node.x ?? 0);
    setPanY(node.y ?? 0);
    setZoom((z) => Math.max(z, 1.8));
    props.onNodeSelect?.(nodeId);
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
      setHighlightedIds(new Set(props.externalHighlightedIds ?? []));
      props.onNodeSelect?.(null);
    }
    if (e.key.toLowerCase() === "f") fitView();
  }, [props.externalHighlightedIds]);

  const onKeyUp = useCallback((e: KeyboardEvent) => {
    ctrlDownRef.current = e.ctrlKey || e.metaKey;
  }, []);

  const onMouseMove = useCallback((e: MouseEvent) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const [sx, sy] = getMousePos(e, canvas);

    if (isDraggingNodeRef.current && dragNodeIdRef.current) {
      const gx = (sx - canvas.width * 0.5) / zoom + panX;
      const gy = (canvas.height * 0.5 - sy) / zoom + panY;
      const node = props.graph?.nodes?.get(dragNodeIdRef.current);
      if (node) {
        node.x = gx;
        node.y = gy;
        spatialGridBuiltRef.current = false;
      }
      return;
    }

    if (isRotatingRef.current) {
      setRotation((r) => r + e.movementX * 0.005);
      return;
    }

    if (isPanningRef.current) {
      setPanX((p) => p - e.movementX / zoom);
      setPanY((p) => p + e.movementY / zoom);
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
        setHighlightedIds(new Set([...(props.externalHighlightedIds ?? []), nodeId]));
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
  }, [zoom, panX, panY, rotation, selectedIds, props.graph, props.externalHighlightedIds]);

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
    setZoom((z) => Math.max(0.1, Math.min(12, z * zoomFactor)));
  }, []);

  function renderLoop(now: number) {
    animFrameRef.current = requestAnimationFrame(renderLoop) as any;

    frameCountRef.current++;
    if (now - fpsTimeRef.current >= 1000) {
      frameCountRef.current = 0;
      fpsTimeRef.current = now;
    }

    if (layoutStateRef.current.running) {
      stepLayout(layoutStateRef.current, props.graph?.nodes, props.graph?.edges);
      spatialGridBuiltRef.current = false;
    }

    if (!spatialGridBuiltRef.current && props.graph?.nodes) {
      spatialGridRef.current = buildSpatialGrid(props.graph.nodes);
      spatialGridBuiltRef.current = true;
    }

    const hl = new Set([...highlightedIds]);
    if (hoveredId) hl.add(hoveredId);

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
