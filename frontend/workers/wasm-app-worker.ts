type BootPayload = {
  wasmUrl: string | null
}

type HostMessage =
  | { type: "BOOT"; payload: BootPayload }
  | { type: "STOP" }

let stopped = false

function postLog(message: string) {
  self.postMessage({ type: "LOG", payload: message })
}

async function boot(payload: BootPayload) {
  stopped = false
  self.postMessage({ type: "STATUS", payload: "starting" })

  if (!payload.wasmUrl) {
    self.postMessage({
      type: "BLOCKED",
      payload: ["No wasmUrl configured yet. Object-ref loading must come through the node/object store."],
    })
    return
  }

  self.postMessage({ type: "STATUS", payload: "fetching" })
  const response = await fetch(payload.wasmUrl)
  if (!response.ok) throw new Error(`failed to fetch wasm: ${response.status}`)
  const bytes = new Uint8Array(await response.arrayBuffer())
  if (stopped) return

  self.postMessage({ type: "STATUS", payload: "instantiating" })
  const started = performance.now()
  const { instance } = await WebAssembly.instantiate(bytes, {
    env: {
      abort(message: number, fileName: number, line: number, column: number) {
        throw new Error(`abort at ${line}:${column} message=${message} file=${fileName}`)
      },
    },
  })
  if (stopped) return

  const exports = Object.keys(instance.exports)
  const instantiateMs = performance.now() - started
  self.postMessage({
    type: "READY",
    payload: {
      lines: [
        `loaded ${bytes.byteLength} bytes`,
        `instantiated in ${instantiateMs.toFixed(2)}ms`,
        `exports: ${exports.join(", ") || "none"}`,
      ],
    },
  })
}

function stop() {
  stopped = true
  postLog("stopped")
}

self.onmessage = (event: MessageEvent<HostMessage>) => {
  boot(event.data.payload).catch((error) => {
    self.postMessage({
      type: "ERROR",
      payload: error instanceof Error ? error.message : String(error),
    })
  })
}

self.addEventListener("message", (event: MessageEvent<HostMessage>) => {
  if (event.data.type === "STOP") stop()
})
