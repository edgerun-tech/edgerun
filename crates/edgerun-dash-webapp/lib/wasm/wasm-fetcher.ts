export enum WasmSource {
  PublicUrl = "public_url",
  NodeFetch = "node_fetch",
  LocalUpload = "local_upload",
}

export interface WasmFetchOptions {
  source: WasmSource
  url?: string
  nodeId?: string
  objectId?: string
  isPublic: boolean
}

export interface WasmFetchResult {
  bytes: Uint8Array
  size: number
  name: string
  hash?: string
}

const WASM_MAGIC = new Uint8Array([0x00, 0x61, 0x73, 0x6d])

function isValidWasm(bytes: Uint8Array): boolean {
  return bytes.length >= 4 &&
    bytes[0] === WASM_MAGIC[0] &&
    bytes[1] === WASM_MAGIC[1] &&
    bytes[2] === WASM_MAGIC[2] &&
    bytes[3] === WASM_MAGIC[3]
}

export async function fetchWasmModule(options: WasmFetchOptions): Promise<WasmFetchResult> {
  let bytes: Uint8Array
  let name = options.url ? new URL(options.url).pathname.split("/").pop()?.replace(/\.wasm$/, "") || "unknown" : "unknown"
  let hash: string | undefined

  switch (options.source) {
    case WasmSource.PublicUrl: {
      if (!options.url) throw new Error("URL required for public fetch")
      const resp = await fetch(options.url)
      if (!resp.ok) throw new Error(`HTTP ${resp.status}: ${resp.statusText}`)
      bytes = new Uint8Array(await resp.arrayBuffer())
      break
    }

    case WasmSource.NodeFetch: {
      if (!options.nodeId || !options.objectId) {
        throw new Error("nodeId and objectId required for node fetch")
      }
      const resp = await fetch(`/api/nodes/${options.nodeId}/objects/${options.objectId}`)
      if (!resp.ok) throw new Error(`Node fetch failed: HTTP ${resp.status}`)

      if (options.isPublic) {
        bytes = new Uint8Array(await resp.arrayBuffer())
      } else {
        const envelope = await resp.arrayBuffer()
        bytes = new Uint8Array(envelope)
      }
      name = options.objectId.slice(0, 12)
      hash = options.objectId
      break
    }

    case WasmSource.LocalUpload: {
      throw new Error("Use WasmInstaller for local uploads")
    }

    default:
      throw new Error(`Unknown source: ${options.source}`)
  }

  if (!isValidWasm(bytes)) {
    throw new Error("Invalid WASM module: missing magic bytes (\\0asm)")
  }

  return { bytes, size: bytes.length, name, hash }
}

export function formatWasmSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
