/**
 * Client for connecting Xray to edgerun-codelyzer.
 *
 * Preferred bridge:
 *   cargo run -p edgerun-node --features xray -- xray-server /path/to/repo
 *
 * The bridge is rkyv-only. Browser code forwards HTTP and WebSocket bytes into
 * the protocol WASM decoder and never implements a parallel byte shape.
 */

import type { XrayNode, XrayEdge } from "../graph/types"
import { decodeXrayWireMessage } from "./xray-wire-wasm"

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
    const bytes = new Uint8Array(await response.arrayBuffer())
    const message = await decodeXrayWireMessage(bytes)
    return this.graphPayloadFromWireMessage(message)
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
    this.ws.binaryType = "arraybuffer"

    this.ws.onopen = () => {
      console.log("[xray:ws] Connected to codeanalyzer")
      this.reconnectAttempts = 0
    }

    this.ws.onmessage = (event) => {
      try {
        const data = event.data
        if (typeof data === "string") {
          this.notifyError(new Error("Received non-rkyv WebSocket text frame"))
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

  private graphPayloadFromWireMessage(message: any): GraphData {
    if (message?.type === "graph_data") return message.data
    if (message?.type === "graph_update") return message.data
    throw new Error(`Unsupported codelyzer rkyv payload: ${message?.type || typeof message}`)
  }

  private handleBinaryMessage(data: Uint8Array): void {
    void decodeXrayWireMessage(data)
      .then((message) => {
        this.notifyHandlers(this.graphPayloadFromWireMessage(message))
      })
      .catch((err) => {
        this.notifyError(err instanceof Error ? err : new Error(String(err)))
      })
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
      this.notifyError(new Error("Analyze requests require rkyv browser encoding"))
    } else {
      void this.loadGraph(path)
    }
  }
}
