import {
  createRuntimeId,
  sha256Hex,
  utf8Decode,
  utf8Encode,
  type RuntimeIdentity,
  type RuntimeMessage,
  type RuntimeResponse,
} from "./browser-capability-types"
import { browserMessageRouter, BrowserMessageRouter } from "./message-router"
import { runtimeEventLog } from "./runtime-event-log"

export interface BrowserWasmAppDefinition {
  appId: string
  wasmBytes: Uint8Array
  packageHash?: string
  publicKey?: Uint8Array
  entry?: string
}

export interface BrowserWasmAppInstance {
  appId: string
  instanceId: string
  identity: RuntimeIdentity
  run(input?: Uint8Array | string): Promise<number>
  pollOutput(): Uint8Array
  drainMessages(): RuntimeMessage[]
}

interface PendingHostMessage {
  message: RuntimeMessage
  response?: RuntimeResponse
}

export class BrowserWasmHost {
  private readonly router: BrowserMessageRouter

  constructor(router = browserMessageRouter) {
    this.router = router
  }

  async instantiate(def: BrowserWasmAppDefinition): Promise<BrowserWasmAppInstance> {
    const instanceId = createRuntimeId("app_instance")
    const packageHash = def.packageHash ?? await sha256Hex(def.wasmBytes)
    const identity: RuntimeIdentity = {
      appId: def.appId,
      publicKey: def.publicKey,
      packageHash,
      instanceId,
      assuranceClass: "software",
    }

    const output: number[] = []
    const pendingEvents: Uint8Array[] = []
    const emittedMessages: RuntimeMessage[] = []
    const pendingMessages: PendingHostMessage[] = []

    const imports = {
      env: {
        write_output: (ptr: number, len: number) => {
          const bytes = readMemory(ptr, len)
          output.push(...bytes)
        },
        send_message: (targetPtr: number, targetLen: number, payloadPtr: number, payloadLen: number): number => {
          const target = utf8Decode(readMemory(targetPtr, targetLen))
          const payload = readMemory(payloadPtr, payloadLen)
          const message: RuntimeMessage = {
            id: createRuntimeId("msg"),
            from: def.appId,
            to: target,
            payload,
            evidence: {
              caller: identity,
              target: this.router.getIdentity(target),
              context: {
                wasmHost: "browser",
              },
            },
            createdAt: Date.now(),
          }
          emittedMessages.push(message)
          pendingMessages.push({ message })
          void this.router.send(message).then(response => {
            const pending = pendingMessages.find(item => item.message.id === message.id)
            if (pending) pending.response = response
            if (response.payload) pendingEvents.push(response.payload)
          })
          return 0
        },
        poll_event: (bufPtr: number, bufLen: number): number => {
          const event = pendingEvents.shift()
          if (!event) return -1
          if (event.byteLength > bufLen) {
            pendingEvents.unshift(event)
            return -2
          }
          writeMemory(bufPtr, event)
          return event.byteLength
        },
        random_bytes: (ptr: number, len: number): number => {
          const random = new Uint8Array(len)
          crypto.getRandomValues(random)
          writeMemory(ptr, random)
          return len
        },
        now_ms: (): number => Date.now(),
      },
    }

    let wasmInstance: WebAssembly.Instance | undefined
    let memory: WebAssembly.Memory | undefined

    const readMemory = (ptr: number, len: number): Uint8Array => {
      if (!memory) throw new Error("WASM memory is not initialized")
      return new Uint8Array(memory.buffer, ptr, len).slice()
    }

    const writeMemory = (ptr: number, bytes: Uint8Array): void => {
      if (!memory) throw new Error("WASM memory is not initialized")
      new Uint8Array(memory.buffer, ptr, bytes.byteLength).set(bytes)
    }

    const result = await WebAssembly.instantiate(def.wasmBytes, imports)
    wasmInstance = result.instance
    memory = wasmInstance.exports.memory as WebAssembly.Memory | undefined
    if (!memory) throw new Error("WASM module must export memory")

    const endpoint = {
      appId: def.appId,
      identity,
      post: async (message: RuntimeMessage): Promise<RuntimeResponse> => {
        pendingEvents.push(message.payload)
        const event = runtimeEventLog.append({
          kind: "router_message_delivered",
          actor: message.from,
          target: def.appId,
          requestId: message.id,
          metadata: {
            deliveredToWasmApp: true,
          },
        })
        return {
          id: createRuntimeId("rsp"),
          requestId: message.id,
          from: def.appId,
          to: message.from,
          decision: "allow",
          eventId: event.id,
        }
      },
    }
    this.router.registerApp(endpoint)

    runtimeEventLog.append({
      kind: "app_instance_created",
      actor: def.appId,
      metadata: {
        instanceId,
        packageHash,
      },
    })

    return {
      appId: def.appId,
      instanceId,
      identity,
      run: async (input?: Uint8Array | string): Promise<number> => {
        const run = wasmInstance!.exports[def.entry ?? "run"]
        if (typeof run !== "function") throw new Error(`WASM module must export ${def.entry ?? "run"}`)

        const bytes = typeof input === "string" ? utf8Encode(input) : input ?? new Uint8Array()
        let ptr = 0
        if (bytes.byteLength > 0) {
          const alloc = wasmInstance!.exports.alloc
          if (typeof alloc === "function") {
            ptr = Number(alloc(bytes.byteLength))
          } else {
            ptr = memory!.buffer.byteLength
            memory!.grow(Math.max(1, Math.ceil(bytes.byteLength / 65536)))
          }
          writeMemory(ptr, bytes)
        }

        const code = Number(run(ptr, bytes.byteLength))
        runtimeEventLog.append({
          kind: "capability_action_completed",
          actor: def.appId,
          reason: "wasm run completed",
          metadata: {
            returnCode: code,
            inputBytes: bytes.byteLength,
          },
        })
        return code
      },
      pollOutput: () => new Uint8Array(output),
      drainMessages: () => emittedMessages.splice(0, emittedMessages.length),
    }
  }
}

export const browserWasmHost = new BrowserWasmHost()
