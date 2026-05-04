"use client"

import { useEffect, useRef, useCallback, useState } from "react"
import { useStore } from "@nanostores/react"
import {
  xrayState,
  setViewTransform,
  selectNode,
  ensureCodeAnalyzerConnection,
  setHoveredNode,
  getVisibleGraph,
} from "./graph/graph-store"
import { WebGLRenderer } from "./render/webgl-renderer"
import { runForceLayout } from "./layout/force-layout"
import { runGlobeLayout } from "./layout/globe-layout"
import { runLayerLayout } from "./layout/layer-layout"

function shortPath(path?: string) {
  if (!path) return "unknown"
  const parts = path.split("/")
  return parts.length > 4 ? parts.slice(-4).join("/") : path
}

export function XrayViewport() {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const rendererRef = useRef<WebGLRenderer | null>(null)
  const state = useStore(xrayState)
  const [isDragging, setIsDragging] = useState(false)
  const [hoverPoint, setHoverPoint] = useState<{ x: number; y: number } | null>(null)
  const dragStart = useRef({ x: 0, y: 0 })
  const dragPanStart = useRef({ x: 0, y: 0 })
  const dragRotationStart = useRef(0)
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
      rotation: s.rotation,
      runtimeMode: s.runtimeMode,
    })
  }, [])

  useEffect(() => {
    ensureCodeAnalyzerConnection()
  }, [])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return

    const renderer = new WebGLRenderer(canvas)
    rendererRef.current = renderer
    renderer.resize()

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

    const stop = renderer.startLoop(renderFrame)

    const onResize = () => renderer.resize()
    window.addEventListener("resize", onResize)

    return () => {
      stop()
      renderer.destroy()
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
  }, [state.layout, state.nodes.size, state.edges.length, state.hiddenFilterKeys.size])

  const findClosestNode = useCallback((e: React.MouseEvent) => {
    const canvas = canvasRef.current
    if (!canvas || !rendererRef.current) return null

    const rect = canvas.getBoundingClientRect()
    const dpr = window.devicePixelRatio || 1
    const sx = (e.clientX - rect.left) * dpr
    const sy = (e.clientY - rect.top) * dpr
    const s = xrayState.get()
    const visible = getVisibleGraph(s)
    const [gx, gy] = rendererRef.current.screenToGraphCoords(sx, sy, s.zoom, s.panX, s.panY, s.rotation)

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
    dragMode.current = e.ctrlKey ? "pan" : "rotate"
    dragStart.current = { x: e.clientX, y: e.clientY }
    dragPanStart.current = { x: state.panX, y: state.panY }
    dragRotationStart.current = state.rotation
  }, [state.panX, state.panY, state.rotation])

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    const dx = e.clientX - dragStart.current.x
    const dy = e.clientY - dragStart.current.y
    const s = xrayState.get()

    if (isDragging) {
      if (dragMode.current === "pan") {
        setViewTransform(s.zoom, dragPanStart.current.x + dx / s.zoom, dragPanStart.current.y - dy / s.zoom, s.rotation)
      } else {
        setViewTransform(s.zoom, s.panX, s.panY, dragRotationStart.current + dx * 0.006)
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
    setViewTransform(newZoom, s.panX, s.panY, s.rotation)
  }, [])

  const hoveredNode = state.hoveredId ? state.nodes.get(state.hoveredId) : null
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

      {hoveredNode && hoverPoint ? (
        <div
          className="pointer-events-none fixed z-[80] max-w-[320px] rounded-xl border border-white/10 bg-black/85 px-3 py-2 text-xs shadow-2xl backdrop-blur-md"
          style={{ left: hoverPoint.x + 14, top: hoverPoint.y + 14 }}
        >
          <div className="truncate font-medium text-foreground">{hoveredNode.label}</div>
          <div className="mt-1 truncate font-mono text-[10px] text-muted-foreground">{shortPath(hoveredNode.source?.file)}</div>
          <div className="mt-2 flex flex-wrap gap-1">
            <span className="rounded-full bg-primary/10 px-1.5 py-0.5 font-mono text-[9px] text-primary">{hoveredNode.kind}</span>
            {hoveredNode.language && <span className="rounded-full bg-secondary px-1.5 py-0.5 font-mono text-[9px] text-muted-foreground">{hoveredNode.language}</span>}
            {hoveredNode.layer && <span className="rounded-full bg-secondary px-1.5 py-0.5 font-mono text-[9px] text-muted-foreground">{hoveredNode.layer}</span>}
          </div>
          <div className="mt-1 font-mono text-[10px] text-muted-foreground">{relatedCount} closest relation{relatedCount === 1 ? "" : "s"}</div>
        </div>
      ) : null}
    </div>
  )
}
