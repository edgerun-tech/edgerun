import { EventQueue, encodeNetworkEvent, encodeDiskEvent, encodeTimerEvent, NetworkSubtype } from "./event-queue"
import { decodeUINode, type UINode } from "./protobuf-ui"
import { getAppStorage, type AppStorage } from "./app-storage"

export interface WasmHostCallbacks {
  onOutput: (data: Uint8Array) => void
  onMessage: (target: string, payload: Uint8Array) => void
  onUIRender: (node: UINode) => void
  onJSXRender: (jsx: string) => void
  onUIAction: (action: string) => void
  onLog?: (level: "info" | "warn" | "error", msg: string) => void
}

const UI_CONTENT_TYPE = "edgerun-ui"
const JSX_CONTENT_TYPE = "edgerun-jsx"

export class WasmAdapter {
  private eventQueue = new EventQueue()
  private memory: WebAssembly.Memory | null = null
  private instance: WebAssembly.Instance | null = null
  private callbacks: WasmHostCallbacks
  private storage: AppStorage
  private nextSockId = 1
  private nextTimerId = 1
  private timers = new Map<number, ReturnType<typeof setTimeout>>()
  private blobs = new Map<string, Uint8Array>()
  private running = false
  private verbose: boolean

  constructor(callbacks: WasmHostCallbacks, options?: { verbose?: boolean; appName?: string }) {
    this.callbacks = callbacks
    this.verbose = options?.verbose ?? false
    this.storage = getAppStorage(options?.appName ?? "default")
    this.loadPersistedBlobs()
  }

  async load(wasmBytes: Uint8Array): Promise<void> {
    const imports = this.getImportObject()
    const { instance } = await WebAssembly.instantiate(wasmBytes, imports)
    this.instance = instance
    this.memory = instance.exports.memory as WebAssembly.Memory
    this.running = true
  }

  run(inputPtr: number = 0, inputLen: number = 0): number {
    if (!this.instance) throw new Error("WASM not loaded")
    const runFn = this.instance.exports.run as (a: number, b: number) => number
    return runFn(inputPtr, inputLen)
  }

  pushNetworkConnected(sockId?: number) {
    this.eventQueue.push(encodeNetworkEvent(sockId ?? this.nextSockId++, NetworkSubtype.Connected))
  }

  pushNetworkDisconnected(sockId: number) {
    this.eventQueue.push(encodeNetworkEvent(sockId, NetworkSubtype.Disconnected))
  }

  pushNetworkReceived(sockId: number, data: Uint8Array) {
    this.eventQueue.push(encodeNetworkEvent(sockId, NetworkSubtype.Received, data))
  }

  pushNetworkError(sockId: number) {
    this.eventQueue.push(encodeNetworkEvent(sockId, NetworkSubtype.Error))
  }

  pushDiskReadDone(opId: number, hash: Uint8Array | null, data?: Uint8Array) {
    this.eventQueue.push(encodeDiskEvent(opId, NetworkSubtype.Received as number, hash ?? undefined, data))
  }

  pushTimerFired(timerId: number) {
    this.eventQueue.push(encodeTimerEvent(timerId))
  }

  setTimer(delayMs: number): number {
    const timerId = this.nextTimerId++
    const id = setTimeout(() => {
      this.pushTimerFired(timerId)
    }, delayMs)
    this.timers.set(timerId, id)
    return timerId
  }

  cancelTimer(timerId: number) {
    const id = this.timers.get(timerId)
    if (id !== undefined) {
      clearTimeout(id)
      this.timers.delete(timerId)
    }
  }

  storeBlob(hashHex: string, data: Uint8Array) {
    this.blobs.set(hashHex, data)
    this.persistBlob(hashHex)
  }

  private loadPersistedBlobs() {
    for (const key of this.storage.keys()) {
      const bytes = this.storage.getBytes(key)
      if (bytes) this.blobs.set(key, bytes)
    }
  }

  private persistBlob(hashHex: string) {
    const data = this.blobs.get(hashHex)
    if (data) this.storage.setBytes(hashHex, data)
  }

  sendUIAction(action: string) {
    const actionBytes = new TextEncoder().encode(`{"action":"${action}"}`)
    this.pushNetworkReceived(999, actionBytes)
    this.callbacks.onUIAction(action)
  }

  private getImportObject(): WebAssembly.Imports {
    const adapter = this
    return {
      env: {
        write_output(ptr: number, len: number) {
          adapter.writeOutput(ptr, len)
        },
        send_message(targetPtr: number, targetLen: number, payloadPtr: number, payloadLen: number): number {
          return adapter.sendMessage(targetPtr, targetLen, payloadPtr, payloadLen)
        },
        read_blob(hashPtr: number, hashLen: number, dstPtr: number): number {
          return adapter.readBlob(hashPtr, hashLen, dstPtr)
        },
        write_blob(ptr: number, len: number, hashOutPtr: number): number {
          return adapter.writeBlob(ptr, len, hashOutPtr)
        },
        poll_event(bufPtr: number, bufLen: number): number {
          return adapter.pollEvent(bufPtr, bufLen)
        },
      },
    }
  }

  private readMemU8(): Uint8Array {
    if (!this.memory) throw new Error("No memory")
    return new Uint8Array(this.memory.buffer)
  }

  private readString(ptr: number, len: number): string {
    return new TextDecoder().decode(this.readMemU8().slice(ptr, ptr + len))
  }

  private writeOutput(ptr: number, len: number) {
    const mem = this.readMemU8()
    const data = mem.slice(ptr, ptr + len)
    this.callbacks.onOutput(data)
    this.tryParseUI(data)
    if (this.verbose) {
      this.callbacks.onLog?.("info", `write_output(${len})`)
    }
  }

  private sendMessage(targetPtr: number, targetLen: number, payloadPtr: number, payloadLen: number): number {
    const mem = this.readMemU8()
    const target = this.readString(targetPtr, targetLen)
    const payload = mem.slice(payloadPtr, payloadPtr + payloadLen)
    this.callbacks.onMessage(target, payload)
    if (this.verbose) {
      this.callbacks.onLog?.("info", `send_message(target=${target}, len=${payloadLen})`)
    }
    return 0
  }

  private readBlob(hashPtr: number, hashLen: number, dstPtr: number): number {
    const mem = this.readMemU8()
    const hashHex = Array.from(mem.slice(hashPtr, hashPtr + hashLen))
      .map((b: number) => b.toString(16).padStart(2, "0"))
      .join("")
    const data = this.blobs.get(hashHex)
    if (data && dstPtr >= 0 && data.length <= mem.length - dstPtr) {
      mem.set(data, dstPtr)
      return data.length
    }
    return -1
  }

  private writeBlob(ptr: number, len: number, hashOutPtr: number): number {
    const mem = this.readMemU8()
    const data = mem.slice(ptr, ptr + len)
    const hash = new Uint8Array(32)
    hash.fill(0xab)
    if (hashOutPtr >= 0 && hashOutPtr + 32 <= mem.length) {
      mem.set(hash, hashOutPtr)
    }
    const hashHex = Array.from(hash).map((b) => b.toString(16).padStart(2, "0")).join("")
    this.blobs.set(hashHex, data)
    return 0
  }

  private pollEvent(bufPtr: number, bufLen: number): number {
    const event = this.eventQueue.pop()
    if (!event) return -1
    if (event.length > bufLen) {
      this.eventQueue.pushFront(event)
      return -2
    }
    const mem = this.readMemU8()
    mem.set(event, bufPtr)
    return event.length
  }

  private tryParseUI(data: Uint8Array) {
    // Output format: status(u16) + ct_len(u32) + content_type + body_len(u32) + body
    if (data.length < 10) return

    const view = new DataView(data.buffer, data.byteOffset, data.byteLength)
    const ctLen = view.getUint32(2, true)
    const ctStart = 6
    const ctEnd = ctStart + ctLen

    if (ctEnd > data.length) return

    const contentType = new TextDecoder().decode(data.slice(ctStart, ctEnd))

    const bodyOffset = 6 + ctLen
    if (bodyOffset + 4 > data.length) return

    const bodyLen = view.getUint32(bodyOffset, true)
    const bodyStart = bodyOffset + 4
    if (bodyStart + bodyLen > data.length) return

    if (contentType.includes(JSX_CONTENT_TYPE)) {
      const jsx = new TextDecoder().decode(data.slice(bodyStart, bodyStart + bodyLen))
      this.callbacks.onJSXRender(jsx)
      return
    }

    if (contentType.includes(UI_CONTENT_TYPE)) {
      const uiBytes = data.slice(bodyStart, bodyStart + bodyLen)
      try {
        const node = decodeUINode(uiBytes)
        this.callbacks.onUIRender(node)
      } catch (e) {
        if (this.verbose) {
          this.callbacks.onLog?.("error", `Failed to decode UINode: ${e}`)
        }
      }
    }
  }

  dispose() {
    this.running = false
    for (const id of this.timers.values()) {
      clearTimeout(id)
    }
    this.timers.clear()
    this.blobs.clear()
    this.instance = null
    this.memory = null
  }
}
