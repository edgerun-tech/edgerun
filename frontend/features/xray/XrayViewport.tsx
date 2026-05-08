"use client"

type XrayNode = any;
type XrayEdge = any;

import { useEffect, useRef, useCallback, useState, useMemo } from "react"
import { useStore } from "@nanostores/react"
import {
  xrayState,
  setViewTransform,
  selectNode,
  setHoveredNode,
  getVisibleGraph,
} from "./graph/graph-store"
import { WebGLRenderer } from "./render/webgl-renderer"
import { NODE_FS } from "./render/shaders"
import { runForceLayout } from "./layout/force-layout"
import { runGlobeLayout } from "./layout/globe-layout"
import { runLayerLayout } from "./layout/layer-layout"

const XRAY_NODE_SHADER_STORAGE_KEY = "edgerun.xray.nodeFragmentShader"

function shortPath(path?: string) {
  if (!path) return "unknown"
  const parts = path.split("/")
  return parts.length > 4 ? parts.slice(-4).join("/") : path
}

function relationList(
  nodeId: string | null,
  nodes: Map<string, XrayNode>,
  edges: XrayEdge[],
  limit = 8,
) {
  if (!nodeId) return []
  const items: Array<{ id: string; label: string; kind: string; direction: "in" | "out" }> = []
  for (const edge of edges) {
    if (edge.source === nodeId) {
      const node = nodes.get(edge.target)
      if (node) items.push({ id: node.id, label: node.label, kind: edge.kind, direction: "out" })
    } else if (edge.target === nodeId) {
      const node = nodes.get(edge.source)
      if (node) items.push({ id: node.id, label: node.label, kind: edge.kind, direction: "in" })
    }
    if (items.length >= limit) break
  }
  return items
}

export function XrayViewport() {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const rendererRef = useRef<WebGLRenderer | null>(null)
  const state = useStore(xrayState)
  const [isDragging, setIsDragging] = useState(false)
  const [hoverPoint, setHoverPoint] = useState<{ x: number; y: number } | null>(null)
  const [shaderOpen, setShaderOpen] = useState(false)
  const [shaderSource, setShaderSource] = useState(NODE_FS)
  const [shaderStatus, setShaderStatus] = useState("Built-in shader")
  const dragStart = useRef({ x: 0, y: 0 })
  const dragPanStart = useRef({ x: 0, y: 0 })
  const dragCameraStart = useRef({ yaw: 0, pitch: 0 })
  const dragMode = useRef<"pan" | "rotate">("rotate")

  const renderFrame = useCallback(() => {
    const s = xrayState.get()
    const renderer = rendererRef.current
    if (!renderer) return
    const visible = getVisibleGraph(s)

    renderer.resize()
    renderer.render({
      nodes: visible.nodes,
      edges: visible.edges,
      runtimeStats: s.runtimeStats,
      selectedId: s.selectedId,
      highlightedIds: s.highlightedIds,
      zoom: s.zoom,
      panX: s.panX,
      panY: s.panY,
      yaw: s.yaw,
      pitch: s.pitch,
      runtimeMode: s.runtimeMode,
    })
  }, [])

  const applyShader = useCallback((source: string, save: boolean) => {
    const renderer = rendererRef.current
    if (!renderer) return
    const result = renderer.setNodeFragmentShader(source)
    if (!result.ok) {
      setShaderStatus(result.error)
      return
    }
    if (save) window.localStorage.setItem(XRAY_NODE_SHADER_STORAGE_KEY, source)
    setShaderStatus(save ? "Applied and saved" : "Applied for this session")
    renderFrame()
  }, [renderFrame])

  const resetShader = useCallback(() => {
    window.localStorage.removeItem(XRAY_NODE_SHADER_STORAGE_KEY)
    setShaderSource(NODE_FS)
    applyShader(NODE_FS, false)
    setShaderStatus("Reset to built-in shader")
  }, [applyShader])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return;

    const renderer = new WebGLRenderer(canvas)
    rendererRef.current = renderer
    renderer.resize()

    const savedShader = window.localStorage.getItem(XRAY_NODE_SHADER_STORAGE_KEY)
    if (savedShader?.trim()) {
      setShaderSource(savedShader)
      const result = renderer.setNodeFragmentShader(savedShader)
      setShaderStatus(result.ok ? "Loaded saved shader" : result.error)
    }

    const s = xrayState.get()
    const visible = getVisibleGraph(s)
    const currentLayout = s.layout
    if (currentLayout === "force") {
      runForceLayout(visible.nodes, visible.edges)
    } else if (currentLayout === "globe") {
      runGlobeLayout(visible.nodes)
    } else if (currentLayout === "layers") {
      runLayerLayout(visible.nodes)
    }

    renderFrame()

    const onResize = () => renderer.resize()

    window.addEventListener("resize", onResize)

    return () => {
      renderer.destroy()
      rendererRef.current = null
      window.removeEventListener("resize", onResize)
    }
  }, [renderFrame])

  useEffect(() => {
    const s = xrayState.get()
    const visible = getVisibleGraph(s)
    if (s.layout === "force") {
      runForceLayout(visible.nodes, visible.edges)
    } else if (s.layout === "globe") {
      runGlobeLayout(visible.nodes)
    } else if (s.layout === "layers") {
      runLayerLayout(visible.nodes)
    }
    renderFrame()
  }, [state.layout, state.nodes.size, state.edges.length, state.hiddenFilterKeys.size, renderFrame])

  useEffect(() => {
    renderFrame()
  }, [state.zoom, state.panX, state.panY, state.yaw, state.pitch, state.selectedId, state.highlightedIds, renderFrame])

  const findClosestNode = useCallback((e: React.MouseEvent) => {
    const canvas = canvasRef.current
    if (!canvas || !rendererRef.current) return null

    const rect = canvas.getBoundingClientRect()
    const dpr = window.devicePixelRatio || 1
    const sx = (e.clientX - rect.left) * dpr
    const sy = (e.clientY - rect.top) * dpr
    const s = xrayState.get()
    const visible = getVisibleGraph(s)
    const [gx, gy] = rendererRef.current.screenToGraphCoords(sx, sy, s.zoom, s.panX, s.panY, s.yaw)

    let closest: string | null = null
    let closestDist = Infinity
    const hitRadius = Math.max(10, 18 / s.zoom)
    for (const [id, node] of visible.nodes) {
      const dx = (node.x ?? 0) - gx
      const dy = (node.y ?? 0) - gy
      const dist = dx * dx + dy * dy
      if (dist < hitRadius * hitRadius && dist < closestDist) {
        closest = id
        closestDist = dist
      }
    }
    return closest
  }, [])

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (e.button !== 0) return
    setIsDragging(true)
    dragMode.current = e.ctrlKey || e.shiftKey ? "pan" : "rotate"
    dragStart.current = { x: e.clientX, y: e.clientY }
    dragPanStart.current = { x: state.panX, y: state.panY }
    dragCameraStart.current = { yaw: state.yaw, pitch: state.pitch }
  }, [state.panX, state.panY, state.yaw, state.pitch])

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    const dx = e.clientX - dragStart.current.x
    const dy = e.clientY - dragStart.current.y
    const s = xrayState.get()

    if (isDragging) {
      if (dragMode.current === "pan") {
        setViewTransform(s.zoom, dragPanStart.current.x + dx / s.zoom, dragPanStart.current.y - dy / s.zoom, s.yaw, s.pitch)
      } else {
        setViewTransform(
          s.zoom,
          s.panX,
          s.panY,
          dragCameraStart.current.yaw + dx * 0.006,
          dragCameraStart.current.pitch - dy * 0.006,
        )
      }
      return
    }

    const closest = findClosestNode(e)
    setHoveredNode(closest)
    setHoverPoint(closest ? { x: e.clientX, y: e.clientY } : null)
  }, [findClosestNode, isDragging])

  const handleMouseUp = useCallback(() => {
    setIsDragging(false)
  }, [])

  const handleClick = useCallback((e: React.MouseEvent) => {
    if (isDragging) return
    selectNode(findClosestNode(e))
  }, [findClosestNode, isDragging])

  const handleWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault()
    const s = xrayState.get()
    const factor = e.deltaY > 0 ? 0.9 : 1.1
    const newZoom = Math.max(0.1, Math.min(10, s.zoom * factor))
    setViewTransform(newZoom, s.panX, s.panY, s.yaw, s.pitch)
  }, [])

  const inspectedId = state.hoveredId || state.selectedId
  const inspectedNode = inspectedId ? state.nodes.get(inspectedId) : null
  const inspectedRelations = useMemo(
    () => relationList(inspectedId, state.nodes, state.edges, 8),
    [inspectedId, state.nodes, state.edges],
  )
  const relatedCount = state.highlightedIds.size > 0 ? state.highlightedIds.size - 1 : 0

  return (
    <div className="relative h-full w-full">
      <canvas
        ref={canvasRef}
        className="h-full w-full cursor-grab active:cursor-grabbing"
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={() => {
          handleMouseUp()
          setHoveredNode(null)
          setHoverPoint(null)
        }}
        onClick={handleClick}
        onWheel={handleWheel}
      />

      <button type="button" onClick={() => setShaderOpen((open) => !open)} className="absolute left-3 top-3 z-[70] rounded-full border border-white/10 bg-black/70 px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.18em] text-primary shadow-xl backdrop-blur-md hover:bg-black/85">
        shader
      </button>

      {shaderOpen ? (
        <div className="absolute left-3 top-12 z-[75] flex max-h-[calc(100%-4.5rem)] w-[min(680px,calc(100%-1.5rem))] flex-col overflow-hidden rounded-2xl border border-white/10 bg-black/88 text-xs shadow-2xl backdrop-blur-md">
          <div className="flex items-center justify-between gap-3 border-b border-white/10 px-3 py-2">
            <div className="min-w-0"><div className="font-medium text-foreground">Node fragment shader</div><div className="truncate font-mono text-[10px] text-muted-foreground">localStorage: {XRAY_NODE_SHADER_STORAGE_KEY}</div></div>
            <button type="button" onClick={() => setShaderOpen(false)} className="rounded-full border border-white/10 px-2 py-1 text-[10px] text-muted-foreground hover:text-foreground">close</button>
          </div>
          <div className="flex min-h-0 flex-col gap-2 p-3">
            <textarea value={shaderSource} onChange={(event) => setShaderSource(event.target.value)} spellCheck={false} className="h-[420px] min-h-[240px] resize-none rounded-xl border border-white/10 bg-zinc-950/95 p-3 font-mono text-[11px] leading-relaxed text-foreground outline-none ring-primary/40 focus:ring-2" />
            <div className="flex flex-wrap items-center gap-2">
              <button type="button" onClick={() => applyShader(shaderSource, true)} className="rounded-full bg-primary px-3 py-1.5 text-[10px] font-medium uppercase tracking-[0.16em] text-primary-foreground hover:bg-primary/90">apply + save</button>
              <button type="button" onClick={() => applyShader(shaderSource, false)} className="rounded-full border border-white/10 px-3 py-1.5 text-[10px] font-medium uppercase tracking-[0.16em] text-foreground hover:bg-white/10">test only</button>
              <button type="button" onClick={resetShader} className="rounded-full border border-white/10 px-3 py-1.5 text-[10px] font-medium uppercase tracking-[0.16em] text-muted-foreground hover:bg-white/10 hover:text-foreground">reset</button>
              <span className="min-w-0 flex-1 truncate font-mono text-[10px] text-muted-foreground">{shaderStatus}</span>
            </div>
            <div className="rounded-lg border border-amber-400/20 bg-amber-400/5 p-2 font-mono text-[10px] leading-relaxed text-amber-100/80">Keep this GLSL contract: in vec4 v_color; in float v_selected; out vec4 fragColor; gl_PointCoord is available.</div>
          </div>
        </div>
      ) : null}

      {inspectedNode ? (
        <div
          className="pointer-events-auto fixed z-[80] w-[320px] rounded-xl border border-white/10 bg-black/85 px-3 py-2 text-xs shadow-2xl backdrop-blur-md"
          style={{
            left: hoverPoint ? Math.min(hoverPoint.x + 18, window.innerWidth - 340) : undefined,
            top: hoverPoint ? Math.min(hoverPoint.y + 18, window.innerHeight - 260) : undefined,
            right: hoverPoint ? undefined : 24,
            bottom: hoverPoint ? undefined : 92,
          }}
        >
          <div className="flex items-start justify-between gap-2">
            <div className="min-w-0">
              <div className="truncate font-medium text-foreground">{inspectedNode.label}</div>
              <div className="mt-1 truncate font-mono text-[10px] text-muted-foreground">{shortPath(inspectedNode.source?.file)}</div>
            </div>
            {state.selectedId === inspectedNode.id && <span className="shrink-0 rounded-full bg-primary/10 px-1.5 py-0.5 font-mono text-[9px] text-primary">selected</span>}
          </div>
          <div className="mt-2 flex flex-wrap gap-1">
            <span className="rounded-full bg-primary/10 px-1.5 py-0.5 font-mono text-[9px] text-primary">{inspectedNode.kind}</span>
            {inspectedNode.language && <span className="rounded-full bg-secondary px-1.5 py-0.5 font-mono text-[9px] text-muted-foreground">{inspectedNode.language}</span>}
            {inspectedNode.layer && <span className="rounded-full bg-secondary px-1.5 py-0.5 font-mono text-[9px] text-muted-foreground">{inspectedNode.layer}</span>}
          </div>
          <div className="mt-2 font-mono text-[10px] text-muted-foreground">{relatedCount} closest relation{relatedCount === 1 ? "" : "s"}</div>
          {inspectedRelations.length > 0 && (
            <div className="mt-2 max-h-36 space-y-1 overflow-auto">
              {inspectedRelations.map((relation, index) => (
                <button key={`${relation.direction}-${relation.kind}-${relation.id}-${index}`} type="button" onClick={() => selectNode(relation.id)} className="flex w-full items-center gap-2 rounded-lg bg-white/5 px-2 py-1 text-left text-[10px] hover:bg-white/10">
                  <span className="shrink-0 font-mono text-primary">{relation.direction === "out" ? "→" : "←"}</span>
                  <span className="min-w-0 flex-1 truncate text-foreground">{relation.label}</span>
                  <span className="shrink-0 font-mono text-muted-foreground">{relation.kind}</span>
                </button>
              ))}
            </div>
          )}
        </div>
      ) : null}
    </div>
  )
}
