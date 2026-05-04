import { atom } from "nanostores"
import type { XrayState, LayoutType, XrayNode, XrayEdge, XrayFilterKey } from "./types"
import { createMockGraph, createMockRuntimeStats } from "./mock-graph"
import { CodeAnalyzerWsService } from "../services/codeanalyzer-ws"

let wsService: CodeAnalyzerWsService | null = null
let codeAnalyzerInitialized = false

const MAX_VISIBLE_NODES = 2800
const MAX_VISIBLE_EDGES = 9000
const MAX_FILE_NODES = 360
const DEFAULT_YAW = 0
const DEFAULT_PITCH = 0.72

function initialState(): XrayState {
  const graph = createMockGraph()
  return {
    nodes: graph.nodes,
    edges: graph.edges,
    runtimeStats: createMockRuntimeStats(graph.nodes),
    selectedId: null,
    hoveredId: null,
    highlightedIds: new Set(),
    hiddenFilterKeys: new Set(),
    layout: "force",
    runtimeMode: false,
    zoom: 1,
    panX: 0,
    panY: 0,
    yaw: DEFAULT_YAW,
    pitch: DEFAULT_PITCH,
    rotation: DEFAULT_YAW,
    loading: false,
    error: null,
  }
}

export function nodeFilterKeys(node: XrayNode): XrayFilterKey[] {
  const keys = new Set<XrayFilterKey>()
  if (node.kind === "file" || node.kind === "function") keys.add(node.kind)
  if (node.layer) keys.add(node.layer as XrayFilterKey)
  if (node.language) keys.add(node.language as XrayFilterKey)
  for (const tag of node.tags || []) {
    if (["ui", "runtime", "storage", "network", "crypto", "agent", "rust", "typescript", "javascript", "go", "python", "c", "unknown"].includes(tag)) {
      keys.add(tag as XrayFilterKey)
    }
  }
  return [...keys]
}

export function getVisibleGraph(state: XrayState = xrayState.get()) {
  if (state.hiddenFilterKeys.size === 0) return { nodes: state.nodes, edges: state.edges }
  const nodes = new Map<string, XrayNode>()
  for (const [id, node] of state.nodes) {
    const hidden = nodeFilterKeys(node).some((key) => state.hiddenFilterKeys.has(key))
    if (!hidden) nodes.set(id, node)
  }
  const edges = state.edges.filter((edge) => nodes.has(edge.source) && nodes.has(edge.target))
  return { nodes, edges }
}

function edgeScore(edge: XrayEdge, degree: Map<string, number>) {
  return (degree.get(edge.source) || 0) + (degree.get(edge.target) || 0)
}

function stableHash(input: string) {
  let hash = 2166136261
  for (let i = 0; i < input.length; i++) {
    hash ^= input.charCodeAt(i)
    hash = Math.imul(hash, 16777619)
  }
  return hash >>> 0
}

function zForNode(node: XrayNode, degree: number) {
  const hash = stableHash(node.id)
  const jitter = ((hash % 1000) / 1000 - 0.5) * 180
  const layerBase: Record<string, number> = {
    file: -260,
    ui: 180,
    api: 80,
    runtime: 0,
    protocol: -80,
    storage: -150,
    network: 220,
    crypto: 260,
    agent: 120,
  }
  const semantic = node.kind === "file" ? layerBase.file : layerBase[node.layer || "runtime"] ?? 0
  const connectivity = Math.min(260, Math.sqrt(Math.max(0, degree)) * 34)
  return semantic + connectivity + jitter
}

function withDepth(node: XrayNode, degree: number): XrayNode {
  return { ...node, z: node.z ?? zForNode(node, degree) }
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
    const fileNode: XrayNode = {
      id: `file:${file}`,
      kind: "file",
      label: file.split("/").slice(-1)[0] || file,
      tags: ["file", node.layer || "code", node.language || "unknown"].filter(Boolean) as string[],
      layer: node.layer,
      language: node.language,
      source: { file },
      x: 0,
      y: 0,
    }
    fileNodes.push(withDepth(fileNode, degree.get(node.id) || 0))
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
    .map((node) => withDepth(node, degree.get(node.id) || 0))

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
    nodeMap.set(node.id, { ...node, x: node.x ?? 0, y: node.y ?? 0, z: node.z ?? 0 })
  }

  const current = xrayState.get()
  xrayState.set({
    ...current,
    nodes: nodeMap,
    edges: projected.edges,
    runtimeStats: createMockRuntimeStats(nodeMap),
    selectedId: null,
    hoveredId: null,
    highlightedIds: new Set(),
    loading: false,
    error: null,
    zoom: 1,
    panX: 0,
    panY: 0,
    yaw: DEFAULT_YAW,
    pitch: DEFAULT_PITCH,
    rotation: DEFAULT_YAW,
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

export function toggleXrayFilter(key: XrayFilterKey) {
  const s = xrayState.get()
  const hidden = new Set(s.hiddenFilterKeys)
  if (hidden.has(key)) hidden.delete(key)
  else hidden.add(key)
  xrayState.set({ ...s, hiddenFilterKeys: hidden })
}

export function setHoveredNode(nodeId: string | null) {
  const s = xrayState.get()
  if (s.hoveredId === nodeId) return
  if (!nodeId) {
    xrayState.set({ ...s, hoveredId: null, highlightedIds: s.selectedId ? buildRelatedSet(s.selectedId, s.edges) : new Set() })
    return
  }
  xrayState.set({ ...s, hoveredId: nodeId, highlightedIds: buildRelatedSet(nodeId, s.edges) })
}

function buildRelatedSet(nodeId: string, edges: XrayEdge[]) {
  const related = new Set<string>([nodeId])
  for (const edge of edges) {
    if (edge.source === nodeId) related.add(edge.target)
    if (edge.target === nodeId) related.add(edge.source)
    if (related.size > 80) break
  }
  return related
}

export function setLayout(layout: LayoutType) {
  const s = xrayState.get()
  for (const node of s.nodes.values()) {
    node.prevX = node.x ?? 0
    node.prevY = node.y ?? 0
    node.prevZ = node.z ?? 0
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
  xrayState.set({ ...s, selectedId: nodeId, highlightedIds: buildRelatedSet(nodeId, s.edges), panX: -(node.x ?? 0), panY: -(node.y ?? 0), zoom: 1.5 })
}

export function highlightNodes(nodeIds: string[]) {
  const s = xrayState.get()
  xrayState.set({ ...s, highlightedIds: new Set(nodeIds) })
}

export function clearHighlight() {
  const s = xrayState.get()
  xrayState.set({ ...s, highlightedIds: s.selectedId ? buildRelatedSet(s.selectedId, s.edges) : new Set() })
}

export function selectNode(nodeId: string | null) {
  const s = xrayState.get()
  xrayState.set({ ...s, selectedId: nodeId, highlightedIds: nodeId ? buildRelatedSet(nodeId, s.edges) : new Set() })
}

export function setViewTransform(zoom: number, panX: number, panY: number, yaw: number, pitch = xrayState.get().pitch) {
  const clampedPitch = Math.max(-1.35, Math.min(1.35, pitch))
  const s = xrayState.get()
  xrayState.set({ ...s, zoom, panX, panY, yaw, pitch: clampedPitch, rotation: yaw })
}

export function resetView() {
  const s = xrayState.get()
  xrayState.set({ ...s, selectedId: null, hoveredId: null, highlightedIds: new Set(), zoom: 1, panX: 0, panY: 0, yaw: DEFAULT_YAW, pitch: DEFAULT_PITCH, rotation: DEFAULT_YAW })
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
    yaw: s.yaw,
    pitch: s.pitch,
  }
}
