type BootPayload = {
  appId: string
  name: string
  description: string
  permissions: string[]
  source: string | null
}

type HostMessage =
  | { type: "BOOT"; payload: BootPayload }
  | { type: "STOP" }

type ViewState = {
  title?: string
  lines: string[]
}

type AppApi = {
  log: (message: unknown) => void
  view: (view: ViewState) => void
  permissions: readonly string[]
  metadata: Readonly<Pick<BootPayload, "appId" | "name" | "description">>
}

let stopped = false
let interval: ReturnType<typeof setInterval> | null = null

function postLog(message: unknown) {
  self.postMessage({ type: "LOG", payload: typeof message === "string" ? message : JSON.stringify(message) })
}

function postView(view: ViewState) {
  self.postMessage({ type: "VIEW", payload: view })
}

function deny(name: string): never {
  throw new Error(`${name} is not available to sandboxed worker apps; request a host capability instead`)
}

function hardenWorkerGlobals() {
  const deniedGlobals = [
    "fetch",
    "XMLHttpRequest",
    "WebSocket",
    "EventSource",
    "Worker",
    "SharedWorker",
    "importScripts",
    "indexedDB",
    "caches",
    "navigator",
  ]

  for (const name of deniedGlobals) {
    try {
      Object.defineProperty(globalThis, name, {
        configurable: false,
        enumerable: false,
        get() {
          return () => deny(name)
        },
        set() {
          deny(name)
        },
      })
    } catch {
      // Some browser-provided worker globals may be non-configurable already.
      // In that case the host must enforce the same policy with CSP / worker origin isolation.
    }
  }
}

function runAppSource(payload: BootPayload) {
  if (!payload.source) return

  const api: AppApi = Object.freeze({
    log: postLog,
    view: postView,
    permissions: Object.freeze([...payload.permissions]),
    metadata: Object.freeze({
      appId: payload.appId,
      name: payload.name,
      description: payload.description,
    }),
  })

  const source = `"use strict";\n${payload.source}\n//# sourceURL=edgerun-sandbox://${payload.appId}.js`

  const appFactory = new Function(
    "app",
    "fetch",
    "XMLHttpRequest",
    "WebSocket",
    "EventSource",
    "Worker",
    "SharedWorker",
    "importScripts",
    "indexedDB",
    "caches",
    "navigator",
    source,
  )

  appFactory(
    api,
    () => deny("fetch"),
    () => deny("XMLHttpRequest"),
    () => deny("WebSocket"),
    () => deny("EventSource"),
    () => deny("Worker"),
    () => deny("SharedWorker"),
    () => deny("importScripts"),
    () => deny("indexedDB"),
    () => deny("caches"),
    Object.freeze({}),
  )
}

function boot(payload: BootPayload) {
  stopped = false
  hardenWorkerGlobals()
  self.postMessage({ type: "READY" })
  postLog(`boot ${payload.appId}`)

  postView({
    title: payload.name,
    lines: [
      payload.description || "Sandboxed JavaScript app",
      `permissions: ${payload.permissions.join(", ") || "none"}`,
      "network: denied by default",
      "dom: unavailable inside worker",
      "storage: host-mediated only",
      payload.source ? "source: runtime manifest" : "source: none",
    ],
  })

  runAppSource(payload)

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
