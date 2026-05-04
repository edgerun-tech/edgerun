/**
 * Client for connecting Xray to edgerun-codelyzer.
 *
 * Preferred bridge:
 *   cargo run -p edgerun-codelyzer --bin xray-server -- /path/to/repo
 *
 * The server exposes HTTP JSON at /graph. WebSocket is still supported for
 * future live updates, but the stable path is HTTP snapshot fetch + graph_update
 * envelope parsing.
 */

import type { XrayNode, XrayEdge } from "../graph/types"

export interface CodeAnalyzerConfig {
  wsUrl: string
  httpUrl?: string
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
  httpUrl: "http://localhost:13337/graph",
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

  async fetchGraph(path?: string): Promise<GraphData> {
    const url = new URL(this.config.httpUrl)
    if (path) url.searchParams.set("path", path)
    const response = await fetch(url.toString(), { cache: "no-store" })
    if (!response.ok) throw new Error(`codelyzer HTTP ${response.status}`)
    const json = await response.json()
    const data = json?.type === "graph_update" ? json.data : json
    return this.parseGraphData(data)
  }

  async loadGraph(path?: string): Promise<void> {
    try {
      const graphData = await this.fetchGraph(path)
      this.notifyHandlers(graphData)
    } catch (err) {
      this.notifyError(err instanceof Error ? err : new Error(String(err)))
    }
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

    this.ws.onerror = () => {
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
      } else if (json.nodes || json.edges) {
        this.notifyHandlers(this.parseGraphData(json))
      }
    } catch {
      // Ignore non-JSON messages.
    }
  }

  private handleBinaryMessage(data: Uint8Array): void {
    try {
      const graphData = this.decodeBinaryGraph(data)
      this.notifyHandlers(graphData)
    } catch (err) {
      console.warn("[xray:codelyzer] Failed to decode binary message:", err)
    }
  }

  private parseGraphData(data: any): GraphData {
    const nodes: XrayNode[] = (data?.nodes || []).map((n: any) => ({
      id: n.id || "",
      kind: this.mapNodeKind(n),
      label: n.name || n.label || n.id || "",
      tags: Array.isArray(n.tags) ? n.tags : [],
      layer: this.inferLayer(n),
      language: this.mapLanguage(n.language),
      source: n.file ? { file: n.file, symbol: n.name || n.label } : undefined,
      x: typeof n.x === "number" ? n.x : 0,
      y: typeof n.y === "number" ? n.y : 0,
    }))

    const edges: XrayEdge[] = (data?.edges || []).map((e: any, index: number) => ({
      id: e.id || `${e.source}->${e.target}:${e.kind || "calls"}:${index}`,
      source: e.source || "",
      target: e.target || "",
      kind: this.mapEdgeKind(e.kind),
      tags: Array.isArray(e.tags) ? e.tags : [],
    }))

    return { nodes, edges }
  }

  private decodeBinaryGraph(data: Uint8Array): GraphData {
    const reader = new BinaryReader(data)
    const nodeCount = reader.u32()
    const nodes: XrayNode[] = []

    for (let i = 0; i < nodeCount; i++) {
      const id = reader.string()
      const name = reader.string()
      const file = reader.string()
      const language = reader.string()
      const isStatic = reader.bool()
      const connections = reader.u32()
      const tags = reader.strings()
      const commit = reader.optionalString()

      nodes.push({
        id,
        kind: "function",
        label: name || id,
        tags: [...tags, ...(isStatic ? ["static"] : []), ...(commit ? [`commit:${commit}`] : []), `connections:${connections}`],
        layer: this.inferLayer({ file }),
        language: this.mapLanguage(language),
        source: file ? { file, symbol: name } : undefined,
        x: 0,
        y: 0,
      })
    }

    const edgeCount = reader.u32()
    const edges: XrayEdge[] = []
    for (let i = 0; i < edgeCount; i++) {
      const source = reader.string()
      const target = reader.string()
      const kind = reader.string()
      edges.push({
        id: `${source}->${target}:${kind}:${i}`,
        source,
        target,
        kind: this.mapEdgeKind(kind),
        tags: [],
      })
    }

    return { nodes, edges }
  }

  private mapLanguage(language: string | undefined): XrayNode["language"] {
    const normalized = (language || "unknown").toLowerCase()
    if (["rust", "typescript", "javascript", "python", "go", "java", "c", "unknown"].includes(normalized)) {
      return normalized as XrayNode["language"]
    }
    return "unknown"
  }

  private mapNodeKind(n: any): XrayNode["kind"] {
    const kind = n.kind || ""
    if (["function", "file", "process", "protocol", "event", "test", "storage", "network", "agent"].includes(kind)) {
      return kind as XrayNode["kind"]
    }
    return "function"
  }

  private mapEdgeKind(kind: string): XrayEdge["kind"] {
    const normalized = (kind || "calls").toLowerCase()
    if (normalized === "direct" || normalized === "indirect" || normalized === "macro" || normalized === "unknown") return "calls"
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
    if (validKinds.includes(normalized as any)) return normalized as XrayEdge["kind"]
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
      this.ws.send(JSON.stringify({ type: "request", action: "analyze", payload: { path } }))
    } else {
      void this.loadGraph(path)
    }
  }
}

class BinaryReader {
  private offset = 0

  constructor(private readonly data: Uint8Array) {}

  private checkBounds(bytes: number) {
    if (this.offset + bytes > this.data.length) {
      throw new Error(`BinaryReader: read past end (offset=${this.offset}, need=${bytes}, total=${this.data.length})`)
    }
  }

  bool(): boolean {
    this.checkBounds(1)
    return this.data[this.offset++] !== 0
  }

  u32(): number {
    this.checkBounds(4)
    const view = new DataView(this.data.buffer, this.data.byteOffset + this.offset, 4)
    const value = view.getUint32(0, true)
    this.offset += 4
    return value
  }

  string(): string {
    const len = this.u32()
    this.checkBounds(len)
    const bytes = this.data.slice(this.offset, this.offset + len)
    this.offset += len
    return new TextDecoder().decode(bytes)
  }

  strings(): string[] {
    const len = this.u32()
    const values: string[] = []
    for (let i = 0; i < len; i++) values.push(this.string())
    return values
  }

  optionalString(): string | undefined {
    return this.bool() ? this.string() : undefined
  }
}
