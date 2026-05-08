"use client"

export interface NodeInstallResult {
  ok: boolean
  appId?: string
  releaseId?: string
  manifestSha256?: string
  developerId?: string
  error?: string
}

type EdgerunNodeExports = WebAssembly.Exports & {
  memory: WebAssembly.Memory
  edgerun_node_alloc(len: number): number
  edgerun_node_dealloc(ptr: number, len: number): void
  edgerun_node_install_eapp(ptr: number, len: number, timeMs: bigint): bigint
  edgerun_node_decode_app_store_catalog(ptr: number, len: number): bigint
  edgerun_node_decode_app_manifest(ptr: number, len: number): bigint
  edgerun_node_protocol_request(ptr: number, len: number, timeMs: bigint): bigint
  edgerun_node_add_locator(ptr: number, len: number, timeMs: bigint): bigint
  edgerun_node_ingest_transport_bytes(ptr: number, len: number, timeMs: bigint): bigint
  edgerun_node_next_transport_frame(): bigint
  edgerun_node_outbound_frame_ptr(): number
  edgerun_node_outbound_frame_len(): number
  edgerun_node_last_result_ptr(): number
  edgerun_node_last_result_len(): number
}

let exportsPromise: Promise<EdgerunNodeExports> | null = null
const encoder = new TextEncoder()
const decoder = new TextDecoder()
const EDGERUN_NODE_WASM_PATH = "/runtime/edgerun_node.wasm"

async function loadEdgerunNode(): Promise<EdgerunNodeExports> {
  if (!exportsPromise) {
    exportsPromise = WebAssembly.instantiateStreaming(fetch(EDGERUN_NODE_WASM_PATH), {})
      .then((result) => result.instance.exports as EdgerunNodeExports)
      .catch(async () => {
        const bytes = await fetch(EDGERUN_NODE_WASM_PATH).then((response) => response.arrayBuffer())
        const result = await WebAssembly.instantiate(bytes, {})
        return result.instance.exports as EdgerunNodeExports
      })
  }
  return exportsPromise
}

function readLastResult<T>(node: EdgerunNodeExports): T {
  const ptr = node.edgerun_node_last_result_ptr()
  const len = node.edgerun_node_last_result_len()
  const bytes = new Uint8Array(node.memory.buffer, ptr, len).slice()
  return JSON.parse(decoder.decode(bytes)) as T
}

function readLastResultBytes(node: EdgerunNodeExports): Uint8Array {
  const ptr = node.edgerun_node_last_result_ptr()
  const len = node.edgerun_node_last_result_len()
  return new Uint8Array(node.memory.buffer, ptr, len).slice()
}

function withNodeBytes<T>(
  node: EdgerunNodeExports,
  bytes: Uint8Array,
  fn: (ptr: number, len: number) => T,
): T {
  const ptr = node.edgerun_node_alloc(bytes.byteLength)
  try {
    new Uint8Array(node.memory.buffer, ptr, bytes.byteLength).set(bytes)
    return fn(ptr, bytes.byteLength)
  } finally {
    node.edgerun_node_dealloc(ptr, bytes.byteLength)
  }
}

export async function installEappWithEdgerunNode(eappBytes: Uint8Array): Promise<NodeInstallResult> {
  const node = await loadEdgerunNode()
  return withNodeBytes(node, eappBytes, (ptr) => {
    const packed = node.edgerun_node_install_eapp(ptr, eappBytes.byteLength, BigInt(Date.now()))
    const status = Number(packed >> 32n)
    const result = readLastResult<NodeInstallResult>(node)
    if (status !== 200 || !result.ok) {
      throw new Error(result.error || `edgerun-node rejected eapp: ${status}`)
    }
    return result
  })
}

export interface NodeCatalogDecodeResult {
  ok: boolean
  format: "edgerun-app-store-catalog-rkyv-v1"
  generatedAt: number
  sequence: number
  storeId: string
  signatureVerified: boolean
  apps: Array<{
    appId: string
    runtimeAppId: string
    releaseId: string
    developerId: string
    slug: string
    name: string
    version: string
    description: string
    runtime: "browser-iframe"
    packageUrl: string
    manifestUrl: string
    launchUrl: string
    eappSha256: string
    manifestSha256: string
    packageBytes: number
    developer: string
    requiredCapabilityIds: string[]
    optionalCapabilityIds: string[]
    assets: Array<{ path: string; sha256: string; bytes: number }>
    status: number
  }>
  error?: string
}

export interface NodeManifestDecodeResult {
  ok: boolean
  appId?: string
  developerId?: string
  slug?: string
  name?: string
  version?: string
  summary?: string
  codeSha256?: string
  error?: string
}

export async function decodeAppStoreCatalogWithEdgerunNode(catalogBytes: Uint8Array): Promise<NodeCatalogDecodeResult> {
  const node = await loadEdgerunNode()
  return withNodeBytes(node, catalogBytes, (ptr) => {
    const packed = node.edgerun_node_decode_app_store_catalog(ptr, catalogBytes.byteLength)
    const status = Number(packed >> 32n)
    const result = readLastResult<NodeCatalogDecodeResult>(node)
    if (status !== 200 || !result.ok) {
      throw new Error(result.error || `edgerun-node rejected app store catalog: ${status}`)
    }
    return result
  })
}

export async function decodeAppManifestWithEdgerunNode(manifestBytes: Uint8Array): Promise<NodeManifestDecodeResult> {
  const node = await loadEdgerunNode()
  return withNodeBytes(node, manifestBytes, (ptr) => {
    const packed = node.edgerun_node_decode_app_manifest(ptr, manifestBytes.byteLength)
    const status = Number(packed >> 32n)
    const result = readLastResult<NodeManifestDecodeResult>(node)
    if (status !== 200 || !result.ok) {
      throw new Error(result.error || `edgerun-node rejected app manifest: ${status}`)
    }
    return result
  })
}

export interface NodeProtocolRequest {
  method: string
  path: string
  body?: Uint8Array
}

export interface NodeProtocolResponse {
  status: number
  body: Uint8Array
  headers: Record<string, string>
}

async function flushNodeTransportFrames(ws: WebSocket): Promise<void> {
  while (ws.readyState === WebSocket.OPEN) {
    const frame = await nextNodeTransportFrame()
    if (!frame) return
    ws.send(frame)
  }
}

export async function sendEdgerunNodeProtocol(request: NodeProtocolRequest): Promise<NodeProtocolResponse> {
  const node = await loadEdgerunNode()
  const head = `${request.method}\n${request.path}\n\n`
  const headBytes = encoder.encode(head)
  const body = request.body ?? new Uint8Array(0)
  const bytes = new Uint8Array(headBytes.byteLength + body.byteLength)
  bytes.set(headBytes, 0)
  bytes.set(body, headBytes.byteLength)

  return withNodeBytes(node, bytes, (ptr, len) => {
    const packed = node.edgerun_node_protocol_request(ptr, len, BigInt(Date.now()))
    return {
      status: Number(packed >> 32n),
      body: readLastResultBytes(node),
      headers: {
        "content-type": "application/json",
        "x-edgerun-runtime": "edgerun-node",
      },
    }
  })
}

export interface NodeLocatorRegistration {
  nodeId: string
  nodeTarget: string
  username: string
  registeredAtIso: string
}

export async function registerNodeLocator(reg: NodeLocatorRegistration): Promise<void> {
  const node = await loadEdgerunNode()
  const bytes = encoder.encode(`${reg.nodeId}\nmanual\n${reg.nodeTarget}\n`)
  withNodeBytes(node, bytes, (ptr, len) => {
    const packed = node.edgerun_node_add_locator(ptr, len, BigInt(Date.now()))
    const status = Number(packed >> 32n)
    const result = readLastResult<{ ok?: boolean; error?: string }>(node)
    if (status !== 200 || !result.ok) {
      throw new Error(result.error || `edgerun-node rejected locator: ${status}`)
    }
  })
}

export async function ingestNodeTransportBytes(bytes: Uint8Array): Promise<void> {
  const node = await loadEdgerunNode()
  withNodeBytes(node, bytes, (ptr, len) => {
    const packed = node.edgerun_node_ingest_transport_bytes(ptr, len, BigInt(Date.now()))
    const status = Number(packed >> 32n)
    if (status !== 200) {
      const result = readLastResult<{ error?: string }>(node)
      throw new Error(result.error || `edgerun-node rejected transport frame: ${status}`)
    }
  })
}

export async function nextNodeTransportFrame(): Promise<Uint8Array | null> {
  const node = await loadEdgerunNode()
  const packed = node.edgerun_node_next_transport_frame()
  const status = Number(packed >> 32n)
  const len = Number(packed & 0xffffffffn)
  if (status !== 200 || len === 0) return null
  const ptr = node.edgerun_node_outbound_frame_ptr()
  return new Uint8Array(node.memory.buffer, ptr, len).slice()
}

export interface NodeWebSocketBridgeOptions {
  reconnectMs?: number
}

export class NodeWebSocketBridge {
  private ws: WebSocket | null = null
  private stopped = false
  private reconnectTimer: number | null = null

  constructor(
    private readonly url: string,
    private readonly options: NodeWebSocketBridgeOptions = {},
  ) {}

  start(): void {
    if (this.ws || this.stopped) return
    this.connect()
  }

  stop(): void {
    this.stopped = true
    if (this.reconnectTimer) {
      window.clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
    this.ws?.close()
    this.ws = null
  }

  async flush(): Promise<void> {
    if (this.ws?.readyState === WebSocket.OPEN) {
      await flushNodeTransportFrames(this.ws)
    }
  }

  private connect(): void {
    const ws = new WebSocket(this.url)
    ws.binaryType = "arraybuffer"
    this.ws = ws

    ws.onopen = () => {
      void this.flush().catch(() => ws.close())
    }

    ws.onmessage = (event) => {
      void this.handleFrame(event.data).catch(() => ws.close())
    }

    ws.onclose = () => {
      if (this.ws === ws) this.ws = null
      this.scheduleReconnect()
    }

    ws.onerror = () => {
      ws.close()
    }
  }

  private scheduleReconnect(): void {
    if (this.stopped || this.reconnectTimer) return
    const reconnectMs = this.options.reconnectMs ?? 1000
    this.reconnectTimer = window.setTimeout(() => {
      this.reconnectTimer = null
      if (!this.stopped) this.connect()
    }, reconnectMs)
  }

  private async handleFrame(data: unknown): Promise<void> {
    if (data instanceof ArrayBuffer) {
      await ingestNodeTransportBytes(new Uint8Array(data))
    } else if (data instanceof Blob) {
      await ingestNodeTransportBytes(new Uint8Array(await data.arrayBuffer()))
    } else {
      throw new Error("edgerun-node WebSocket bridge received non-binary frame")
    }
    await this.flush()
  }
}
