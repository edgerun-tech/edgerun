"use client"

import { useEffect, useRef, useCallback, useState } from "react"
import { useStore } from "@nanostores/react"
import { xrayState, setViewTransform, selectNode, ensureCodeAnalyzerConnection } from "./graph/graph-store"
import { WebGLRenderer } from "./render/webgl-renderer"
import { runForceLayout } from "./layout/force-layout"
import { runGlobeLayout } from "./layout/globe-layout"
import { runLayerLayout } from "./layout/layer-layout"

export function XrayViewport() {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const rendererRef = useRef<WebGLRenderer | null>(null)
  const state = useStore(xrayState)
  const [isDragging, setIsDragging] = useState(false)
  const dragStart = useRef({ x: 0, y: 0 })
  const dragPanStart = useRef({ x: 0, y: 0 })

  const renderFrame = useCallback(() => {
    const s = xrayState.get()
    const renderer = rendererRef.current
    if (!renderer) return

    renderer.resize()
    renderer.render({
      nodes: s.nodes,
      edges: s.edges,
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
    const currentLayout = s.layout
    if (currentLayout === "force") {
      runForceLayout(s.nodes, s.edges)
    } else if (currentLayout === "globe") {
      runGlobeLayout(s.nodes)
    } else if (currentLayout === "layers") {
      runLayerLayout(s.nodes)
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
    if (s.layout === "force") {
      runForceLayout(s.nodes, s.edges)
    } else if (s.layout === "globe") {
      runGlobeLayout(s.nodes)
    } else if (s.layout === "layers") {
      runLayerLayout(s.nodes)
    }
  }, [state.layout, state.nodes.size, state.edges.length])

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (e.button !== 0) return
    setIsDragging(true)
    dragStart.current = { x: e.clientX, y: e.clientY }
    dragPanStart.current = { x: state.panX, y: state.panY }
  }, [state.panX, state.panY])

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (!isDragging) return
    const dx = e.clientX - dragStart.current.x
    const dy = e.clientY - dragStart.current.y
    const s = xrayState.get()
    setViewTransform(s.zoom, dragPanStart.current.x + dx / s.zoom, dragPanStart.current.y - dy / s.zoom, s.rotation)
  }, [isDragging])

  const handleMouseUp = useCallback(() => {
    setIsDragging(false)
  }, [])

  const handleClick = useCallback((e: React.MouseEvent) => {
    if (isDragging) return
    const canvas = canvasRef.current
    if (!canvas || !rendererRef.current) return

    const rect = canvas.getBoundingClientRect()
    const dpr = window.devicePixelRatio || 1
    const sx = (e.clientX - rect.left) * dpr
    const sy = (e.clientY - rect.top) * dpr
    const s = xrayState.get()
    const [gx, gy] = rendererRef.current.screenToGraphCoords(sx, sy, s.zoom, s.panX, s.panY, s.rotation)

    let closest: string | null = null
    let closestDist = Infinity
    for (const [id, node] of s.nodes) {
      const dx = (node.x ?? 0) - gx
      const dy = (node.y ?? 0) - gy
      const dist = dx * dx + dy * dy
      const hitRadius = 15
      if (dist < hitRadius * hitRadius && dist < closestDist) {
        closest = id
        closestDist = dist
      }
    }
    selectNode(closest)
  }, [isDragging])

  const handleWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault()
    const s = xrayState.get()
    const factor = e.deltaY > 0 ? 0.9 : 1.1
    const newZoom = Math.max(0.1, Math.min(10, s.zoom * factor))
    setViewTransform(newZoom, s.panX, s.panY, s.rotation)
  }, [])

  return (
    <canvas
      ref={canvasRef}
      className="w-full h-full cursor-grab active:cursor-grabbing"
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onMouseLeave={handleMouseUp}
      onClick={handleClick}
      onWheel={handleWheel}
    />
  )
}
