type BootPayload = {
  appId: string
  name: string
  description: string
  capabilities: string[]
  source: unknown
}

type HostMessage =
  | { type: "BOOT"; payload: BootPayload }
  | { type: "STOP" }

type ViewState = {
  title?: string
  lines: string[]
}

let stopped = false
let interval: ReturnType<typeof setInterval> | null = null

function postLog(message: string) {
  self.postMessage({ type: "LOG", payload: message })
}

function postView(view: ViewState) {
  self.postMessage({ type: "VIEW", payload: view })
}

function boot(payload: BootPayload) {
  stopped = false
  self.postMessage({ type: "READY" })
  postLog(`boot ${payload.appId}`)

  postView({
    title: payload.name,
    lines: [
      payload.description || "Sandboxed JavaScript app",
      `capabilities: ${payload.capabilities.join(", ") || "none"}`,
      "network: denied by default",
      "dom: unavailable inside worker",
      "storage: host-mediated only",
    ],
  })

  interval = setInterval(() => {
    if (stopped) return
    postLog(`heartbeat ${new Date().toISOString()}`)
  }, 5000)
}

function stop() {
  stopped = true
  if (interval) {
    clearInterval(interval)
    interval = null
  }
  postLog("stopped")
}

self.onmessage = (event: MessageEvent<HostMessage>) => {
  try {
    switch (event.data.type) {
      case "BOOT":
        boot(event.data.payload)
        break
      case "STOP":
        stop()
        break
    }
  } catch (error) {
    self.postMessage({
      type: "ERROR",
      payload: error instanceof Error ? error.message : String(error),
    })
  }
}
