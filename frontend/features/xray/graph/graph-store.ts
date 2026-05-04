import { atom } from "nanostores"
import type { XrayState, LayoutType } from "./types"
import { createMockGraph, createMockRuntimeStats } from "./mock-graph"
import { CodeAnalyzerWsService } from "../services/codeanalyzer-ws"

let wsService: CodeAnalyzerWsService | null = null

function initialState(): XrayState {
  const graph = createMockGraph()
  return {
    nodes: graph.nodes,
    edges: graph.edges,
    runtimeStats: createMockRuntimeStats(graph.nodes),
    selectedId: null,
    highlightedIds: new Set(),
    layout: "force",
    runtimeMode: false,
    zoom: 1,
    panX: 0,
    panY: 0,
    rotation: 0,
    loading: false,
    error: null,
  }
}

export const xrayState = atom<XrayState>(initialState())

export function initCodeAnalyzerConnection(url?: string): void {
  if (wsService) {
    wsService.disconnect()
  }

  wsService = new CodeAnalyzerWsService({ wsUrl: url || "ws://localhost:13337/ws" })

  xrayState.set({ ...xrayState.get(), loading: true, error: null })

  wsService.onGraphUpdate((data) => {
    const nodeMap = new Map()
    for (const node of data.nodes) {
      nodeMap.set(node.id, { ...node, x: node.x ?? 0, y: node.y ?? 0 })
    }

    xrayState.set({
      ...xrayState.get(),
      nodes: nodeMap,
      edges: data.edges,
      runtimeStats: createMockRuntimeStats(nodeMap),
      loading: false,
      error: null,
    })
  })

  wsService.onError((error) => {
    console.error("[xray] WebSocket error:", error)
    xrayState.set({ ...xrayState.get(), loading: false, error: error.message })
  })

  wsService.connect()
}

export function requestAnalysis(path: string): void {
  if (wsService) {
    xrayState.set({ ...xrayState.get(), loading: true })
    wsService.requestAnalyze(path)
  }
}

export function setLayout(layout: LayoutType) {
  const s = xrayState.get()
  for (const node of s.nodes.values()) {
    node.prevX = node.x ?? 0
    node.prevY = node.y ?? 0
  }
  xrayState.set({ ...s, layout })
}

export function setRuntimeMode(enabled: boolean) {
  const s = xrayState.get()
  xrayState.set({ ...s, runtimeMode: enabled })
}

export function focusNode(nodeId: string) {
  const s = xrayState.get()
  const node = s.nodes.get(nodeId)
  if (!node) return
  xrayState.set({ ...s, selectedId: nodeId, highlightedIds: new Set(), panX: -(node.x ?? 0), panY: -(node.y ?? 0), zoom: 1.5 })
}

export function highlightNodes(nodeIds: string[]) {
  const s = xrayState.get()
  xrayState.set({ ...s, highlightedIds: new Set(nodeIds) })
}

export function clearHighlight() {
  const s = xrayState.get()
  xrayState.set({ ...s, highlightedIds: new Set() })
}

export function selectNode(nodeId: string | null) {
  const s = xrayState.get()
  xrayState.set({ ...s, selectedId: nodeId })
}

export function setViewTransform(zoom: number, panX: number, panY: number, rotation: number) {
  const s = xrayState.get()
  xrayState.set({ ...s, zoom, panX, panY, rotation })
}

export function resetView() {
  const s = xrayState.get()
  xrayState.set({ ...s, selectedId: null, highlightedIds: new Set(), zoom: 1, panX: 0, panY: 0, rotation: 0 })
}

export function updateNodePositions(positions: Map<string, { x: number; y: number }>) {
  const s = xrayState.get()
  for (const [id, pos] of positions) {
    const node = s.nodes.get(id)
    if (node) {
      node.x = pos.x
      node.y = pos.y
    }
  }
}

export function getXrayState() {
  const s = xrayState.get()
  return {
    layout: s.layout,
    runtimeMode: s.runtimeMode,
    nodeCount: s.nodes.size,
    edgeCount: s.edges.length,
    selectedId: s.selectedId,
    highlightedCount: s.highlightedIds.size,
    zoom: s.zoom,
    panX: s.panX,
    panY: s.panY,
  }
}
