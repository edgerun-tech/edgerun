import { atom } from "nanostores"
import type { XrayState, LayoutType, XrayNode, XrayEdge } from "./types"
import { createMockGraph, createMockRuntimeStats } from "./mock-graph"
import { CodeAnalyzerWsService } from "../services/codeanalyzer-ws"

let wsService: CodeAnalyzerWsService | null = null
let codeAnalyzerInitialized = false

const MAX_VISIBLE_NODES = 2800
const MAX_VISIBLE_EDGES = 9000
const MAX_FILE_NODES = 360

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

function edgeScore(edge: XrayEdge, degree: Map<string, number>) {
  return (degree.get(edge.source) || 0) + (degree.get(edge.target) || 0)
}

function projectGraphData(data: { nodes: XrayNode[]; edges: XrayEdge[] }) {
  const degree = new Map<string, number>()
  for (const edge of data.edges) {
    degree.set(edge.source, (degree.get(edge.source) || 0) + 1)
    degree.set(edge.target, (degree.get(edge.target) || 0) + 1)
  }

  const sourceNodes = data.nodes.filter((node) => node.id)
  const fileNodes: XrayNode[] = []
  const seenFiles = new Set<string>()

  const rankedFiles = [...sourceNodes]
    .filter((node) => node.source?.file)
    .sort((a, b) => (degree.get(b.id) || 0) - (degree.get(a.id) || 0))

  for (const node of rankedFiles) {
    const file = node.source?.file
    if (!file || seenFiles.has(file)) continue
    seenFiles.add(file)
    fileNodes.push({
      id: `file:${file}`,
      kind: "file",
      label: file.split("/").slice(-1)[0] || file,
      tags: ["file", node.layer || "code", node.language || "unknown"].filter(Boolean) as string[],
      layer: node.layer,
      language: node.language,
      source: { file },
      x: 0,
      y: 0,
    })
    if (fileNodes.length >= MAX_FILE_NODES) break
  }

  const rankedNodes = [...sourceNodes]
    .sort((a, b) => {
      const scoreA = degree.get(a.id) || 0
      const scoreB = degree.get(b.id) || 0
      if (scoreA !== scoreB) return scoreB - scoreA
      return (a.source?.file || "").localeCompare(b.source?.file || "")
    })
    .slice(0, Math.max(0, MAX_VISIBLE_NODES - fileNodes.length))

  const visibleIds = new Set(rankedNodes.map((node) => node.id))
  for (const node of fileNodes) visibleIds.add(node.id)

  const visibleEdges = [...data.edges]
    .filter((edge) => visibleIds.has(edge.source) && visibleIds.has(edge.target))
    .sort((a, b) => edgeScore(b, degree) - edgeScore(a, degree))
    .slice(0, MAX_VISIBLE_EDGES)

  const ownershipEdges: XrayEdge[] = []
  const fileNodeIds = new Set(fileNodes.map((node) => node.id))
  for (const node of rankedNodes) {
    const file = node.source?.file
    const fileId = file ? `file:${file}` : null
    if (!fileId || !fileNodeIds.has(fileId)) continue
    ownershipEdges.push({
      id: `${fileId}->${node.id}:owns`,
      source: fileId,
      target: node.id,
      kind: "owns",
      tags: ["file", "symbol"],
    })
    if (visibleEdges.length + ownershipEdges.length >= MAX_VISIBLE_EDGES) break
  }

  return {
    nodes: [...fileNodes, ...rankedNodes],
    edges: [...ownershipEdges, ...visibleEdges],
  }
}

function applyGraphData(data: { nodes: XrayNode[]; edges: XrayEdge[] }) {
  const projected = projectGraphData(data)
  const nodeMap = new Map<string, XrayNode>()
  for (const node of projected.nodes) {
    nodeMap.set(node.id, { ...node, x: node.x ?? 0, y: node.y ?? 0 })
  }

  xrayState.set({
    ...xrayState.get(),
    nodes: nodeMap,
    edges: projected.edges,
    runtimeStats: createMockRuntimeStats(nodeMap),
    selectedId: null,
    highlightedIds: new Set(),
    loading: false,
    error: null,
    zoom: 1,
    panX: 0,
    panY: 0,
    rotation: 0,
  })
}

export const xrayState = atom<XrayState>(initialState())

export function initCodeAnalyzerConnection(url?: string): void {
  if (wsService) {
    wsService.disconnect()
  }

  codeAnalyzerInitialized = true
  const endpoint = url || "http://localhost:13337/graph"
  const isHttp = endpoint.startsWith("http://") || endpoint.startsWith("https://")
  const wsUrl = isHttp ? endpoint.replace(/^http/, "ws").replace(/\/graph$/, "/ws") : endpoint
  const httpUrl = isHttp ? endpoint : endpoint.replace(/^ws/, "http").replace(/\/ws$/, "/graph")

  wsService = new CodeAnalyzerWsService({ wsUrl, httpUrl })

  xrayState.set({ ...xrayState.get(), loading: true, error: null })

  wsService.onGraphUpdate(applyGraphData)

  wsService.onError((error) => {
    console.error("[xray] Code analyzer error:", error)
    xrayState.set({ ...xrayState.get(), loading: false, error: error.message })
  })

  void wsService.loadGraph()
}

export function ensureCodeAnalyzerConnection(): void {
  if (codeAnalyzerInitialized || wsService) return
  initCodeAnalyzerConnection()
}

export function requestAnalysis(path: string): void {
  if (!wsService) initCodeAnalyzerConnection()
  xrayState.set({ ...xrayState.get(), loading: true })
  wsService?.requestAnalyze(path)
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
