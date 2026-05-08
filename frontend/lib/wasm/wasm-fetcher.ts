export enum WasmSource {
  PublicUrl = "public-url",
  NodeFetch = "node-fetch",
}

export type WasmFetchOptions =
  | {
      source: WasmSource.PublicUrl
      url?: string
      isPublic: boolean
    }
  | {
      source: WasmSource.NodeFetch
      objectId: string
      nodeId?: string
      isPublic: boolean
    }

export interface WasmFetchResult {
  name: string
  bytes: Uint8Array
  hash: string
  isPublic: boolean
}

function hex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("")
}

async function sha256(bytes: Uint8Array): Promise<string> {
  return hex(new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)))
}

function nameFromUrl(url: string): string {
  const path = new URL(url, window.location.origin).pathname
  return path.split("/").pop()?.replace(/\.wasm$/, "") || "wasm-app"
}

export async function fetchWasmModule(options: WasmFetchOptions): Promise<WasmFetchResult> {
  const url = options.source === WasmSource.PublicUrl
    ? options.url
    : `/protocol/object/${encodeURIComponent(options.objectId)}`

  if (!url) throw new Error("wasm URL is required")

  const response = await fetch(url, { cache: "no-store" })
  if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`)
  const bytes = new Uint8Array(await response.arrayBuffer())
  return {
    name: options.source === WasmSource.PublicUrl ? nameFromUrl(url) : options.objectId,
    bytes,
    hash: await sha256(bytes),
    isPublic: options.isPublic,
  }
}
