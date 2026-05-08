type WasmExports = {
  memory: WebAssembly.Memory
  edgerun_xray_alloc(len: number): number
  edgerun_xray_free(ptr: number, len: number): void
  edgerun_xray_decode(ptr: number, len: number): bigint
  edgerun_xray_last_error(): bigint
}

const WASM_URL = "/xray-wire/edgerun_codelyzer.wasm"

let wasmPromise: Promise<WasmExports> | null = null

export async function decodeXrayWireMessage(bytes: Uint8Array): Promise<unknown> {
  const wasm = await loadWasm()
  const inputPtr = wasm.edgerun_xray_alloc(bytes.byteLength)

  try {
    new Uint8Array(wasm.memory.buffer, inputPtr, bytes.byteLength).set(bytes)
    const packed = wasm.edgerun_xray_decode(inputPtr, bytes.byteLength)
    if (packed === 0n) {
      throw new Error(readLastError(wasm) || "xray rkyv decode failed")
    }
    const json = readPackedString(wasm, packed)
    return JSON.parse(json)
  } finally {
    wasm.edgerun_xray_free(inputPtr, bytes.byteLength)
  }
}

async function loadWasm(): Promise<WasmExports> {
  if (!wasmPromise) {
    wasmPromise = WebAssembly.instantiateStreaming(fetch(WASM_URL), {}).then((result) => {
      const exports = result.instance.exports as Partial<WasmExports>
      if (
        !(exports.memory instanceof WebAssembly.Memory) ||
        typeof exports.edgerun_xray_alloc !== "function" ||
        typeof exports.edgerun_xray_free !== "function" ||
        typeof exports.edgerun_xray_decode !== "function" ||
        typeof exports.edgerun_xray_last_error !== "function"
      ) {
        throw new Error("xray wire wasm exports are incomplete")
      }
      return exports as WasmExports
    })
  }
  return wasmPromise
}

function readLastError(wasm: WasmExports): string {
  const packed = wasm.edgerun_xray_last_error()
  if (packed === 0n) return ""
  return readPackedString(wasm, packed)
}

function readPackedString(wasm: WasmExports, packed: bigint): string {
  const ptr = Number(packed >> 32n)
  const len = Number(packed & 0xffffffffn)
  const bytes = new Uint8Array(wasm.memory.buffer, ptr, len)
  const text = new TextDecoder().decode(bytes)
  wasm.edgerun_xray_free(ptr, len)
  return text
}
