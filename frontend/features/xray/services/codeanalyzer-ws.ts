/**
 * WebSocket service for connecting to codeanalyzer.
 * Handles connection, message parsing, and graph data fetching.
 */

import type { XrayNode, XrayEdge } from "../graph/types"

export interface CodeAnalyzerConfig {
  wsUrl: string
  reconnectInterval?: number
  maxReconnectAttempts?: number
}

export interface GraphData {
  nodes: XrayNode[]
  edges: XrayEdge[]
}

type MessageHandler = (data: GraphData) => void
type ErrorHandler = (error: Error) => void

const DEFAULT_CONFIG: Required<CodeAnalyzerConfig> = {
  wsUrl: "ws://localhost:13337/ws",
  reconnectInterval: 3000,
  maxReconnectAttempts: 5,
}

export class CodeAnalyzerWsService {
  private ws: WebSocket | null = null
  private config: Required<CodeAnalyzerConfig>
  private reconnectAttempts = 0
  private messageHandlers: Set<MessageHandler> = new Set()
  private errorHandlers: Set<ErrorHandler> = new Set()
  private isIntentionalClose = false

  constructor(config?: Partial<CodeAnalyzerConfig>) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  connect(): void {
    if (this.ws?.readyState === WebSocket.OPEN) return

    this.isIntentionalClose = false
    this.ws = new WebSocket(this.config.wsUrl)

    this.ws.onopen = () => {
      console.log("[xray:ws] Connected to codeanalyzer")
      this.reconnectAttempts = 0
    }

    this.ws.onmessage = (event) => {
      try {
        const data = event.data
        if (typeof data === "string") {
          this.handleJsonMessage(data)
        } else if (data instanceof Blob) {
          data.arrayBuffer().then((buf) => this.handleBinaryMessage(new Uint8Array(buf)))
        } else if (data instanceof ArrayBuffer) {
          this.handleBinaryMessage(new Uint8Array(data))
        }
      } catch (err) {
        this.notifyError(err instanceof Error ? err : new Error(String(err)))
      }
    }

    this.ws.onerror = (event) => {
      this.notifyError(new Error("WebSocket error"))
    }

    this.ws.onclose = () => {
      if (!this.isIntentionalClose && this.reconnectAttempts < this.config.maxReconnectAttempts) {
        this.reconnectAttempts++
        setTimeout(() => this.connect(), this.config.reconnectInterval)
      }
    }
  }

  private handleJsonMessage(text: string): void {
    try {
      const json = JSON.parse(text)
      if (json.type === "graph_update" && json.data) {
        const graphData = this.parseGraphData(json.data)
        this.notifyHandlers(graphData)
      }
    } catch {
      // Ignore non-JSON messages
    }
  }

  private handleBinaryMessage(data: Uint8Array): void {
    try {
      const graphData = this.decodeProtobufGraph(data)
      this.notifyHandlers(graphData)
    } catch (err) {
      console.warn("[xray:ws] Failed to decode binary message:", err)
    }
  }

  private parseGraphData(data: any): GraphData {
    const nodes: XrayNode[] = (data.nodes || []).map((n: any) => ({
      id: n.id || "",
      kind: this.mapNodeKind(n),
      label: n.name || n.id || "",
      tags: Array.isArray(n.tags) ? n.tags : [],
      layer: this.inferLayer(n),
      language: n.language || "unknown",
      source: n.file ? { file: n.file } : undefined,
      x: 0,
      y: 0,
    }))

    const edges: XrayEdge[] = (data.edges || []).map((e: any) => ({
      id: `${e.source}->${e.target}`,
      source: e.source || "",
      target: e.target || "",
      kind: this.mapEdgeKind(e.kind),
      tags: Array.isArray(e.tags) ? e.tags : [],
    }))

    return { nodes, edges }
  }

  private decodeProtobufGraph(data: Uint8Array): GraphData {
    // Simple protobuf-like decode for GraphData
    // In production, use generated protobuf types
    const nodes: XrayNode[] = []
    const edges: XrayEdge[] = []

    let offset = 0
    while (offset < data.length) {
      const tag = data[offset] >> 3
      const wireType = data[offset] & 0x07
      offset++

      if (wireType === 2) {
        // Length-delimited
        let length = 0
        let shift = 0
        while (true) {
          const byte = data[offset++]
          length |= (byte & 0x7f) << shift
          if ((byte & 0x80) === 0) break
          shift += 7
        }

        if (tag === 1) {
          // nodes field
          const nodeData = data.slice(offset, offset + length)
          offset += length
          // Parse node (simplified)
          const node = this.parseProtobufNode(nodeData)
          if (node) nodes.push(node)
        } else if (tag === 2) {
          // edges field
          const edgeData = data.slice(offset, offset + length)
          offset += length
          const edge = this.parseProtobufEdge(edgeData)
          if (edge) edges.push(edge)
        } else {
          offset += length
        }
      } else {
        break
      }
    }

    return { nodes, edges }
  }

  private parseProtobufNode(data: Uint8Array): XrayNode | null {
    // Simplified protobuf node parsing
    return null
  }

  private parseProtobufEdge(data: Uint8Array): XrayEdge | null {
    // Simplified protobuf edge parsing
    return null
  }

  private mapNodeKind(n: any): XrayNode["kind"] {
    const kind = n.kind || ""
    if (["function", "file", "process", "protocol", "event", "test", "storage", "network", "agent"].includes(kind)) {
      return kind as XrayNode["kind"]
    }
    return "function"
  }

  private mapEdgeKind(kind: string): XrayEdge["kind"] {
    const validKinds: XrayEdge["kind"][] = [
      "calls",
      "imports",
      "owns",
      "implements",
      "sends",
      "receives",
      "stores",
      "signs",
      "verifies",
      "tests",
      "observed_flow",
    ]
    if (validKinds.includes(kind as any)) {
      return kind as XrayEdge["kind"]
    }
    return "calls"
  }

  private inferLayer(n: any): XrayNode["layer"] {
    const file = n.file || ""
    if (file.includes("/ui/") || file.includes("components")) return "ui"
    if (file.includes("/api/") || file.includes("routes")) return "api"
    if (file.includes("/runtime/") || file.includes("rt-")) return "runtime"
    if (file.includes("/proto/") || file.includes("protocol")) return "protocol"
    if (file.includes("/storage/") || file.includes("store")) return "storage"
    if (file.includes("/network/") || file.includes("net-")) return "network"
    if (file.includes("/crypto/") || file.includes("crypto")) return "crypto"
    if (file.includes("/agent/") || file.includes("agent")) return "agent"
    return undefined
  }

  onGraphUpdate(handler: MessageHandler): () => void {
    this.messageHandlers.add(handler)
    return () => this.messageHandlers.delete(handler)
  }

  onError(handler: ErrorHandler): () => void {
    this.errorHandlers.add(handler)
    return () => this.errorHandlers.delete(handler)
  }

  private notifyHandlers(data: GraphData): void {
    this.messageHandlers.forEach((h) => h(data))
  }

  private notifyError(error: Error): void {
    this.errorHandlers.forEach((h) => h(error))
  }

  disconnect(): void {
    this.isIntentionalClose = true
    this.ws?.close()
    this.ws = null
  }

  requestAnalyze(path: string): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(
        JSON.stringify({
          type: "request",
          action: "analyze",
          payload: { path },
        }),
      )
    }
  }
}
