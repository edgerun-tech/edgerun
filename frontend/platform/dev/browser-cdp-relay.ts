"use client"

type CdpTarget = {
  id: string
  type: string
  title?: string
  url?: string
  webSocketDebuggerUrl?: string
}

type CdpResponse = {
  id?: number
  result?: unknown
  error?: { message?: string; code?: number; data?: unknown }
  method?: string
  params?: unknown
}

type RelayDestination = "frontend" | "backend" | "chatgpt"
type BackendBridge = (message: string, options?: Record<string, unknown>) => unknown | Promise<unknown>
type BrowserTimer = ReturnType<typeof setTimeout>

export type BrowserCdpRelayState = {
  destination: RelayDestination
  frontend: "ready" | "offline"
  backend: "ready" | "offline"
  backendBridgeRegistered: boolean
  endpoint: string
  updatedAtIso: string
}

export type BrowserCdpRelay = {
  endpoint: string
  eventName: typeof FRONTEND_EVENT
  destination: RelayDestination
  setEndpoint(endpoint: string): string
  setDestination(destination: RelayDestination): RelayDestination
  setBackendBridge(bridge: BackendBridge): void
  status(): BrowserCdpRelayState
  subscribeStatus(handler: (state: BrowserCdpRelayState) => void): () => void
  targets(): Promise<CdpTarget[]>
  connect(query?: string): Promise<BrowserCdpPage>
  connectWebSocket(webSocketUrl: string, target?: Partial<CdpTarget>): Promise<BrowserCdpPage>
  sendToChatGpt(message: string, options?: { target?: string; webSocketUrl?: string; waitMs?: number }): Promise<unknown>
  sendToFrontend(message: string, detail?: Record<string, unknown>): Record<string, unknown>
  subscribe(handler: (message: Record<string, unknown>) => void): () => void
  relay(message: string, destination?: RelayDestination, options?: Record<string, unknown>): Promise<unknown>
}

const DEFAULT_ENDPOINT = "http://127.0.0.1:9222"
const DEFAULT_TIMEOUT_MS = 30_000
const FRONTEND_EVENT = "edgerun:cdp-relay-message"
const FRONTEND_CHANNEL = "edgerun:frontend-relay"
const DESTINATION_STORAGE_KEY = "edgerun:cdp-relay-destination"

function normalizeEndpoint(endpoint?: string): string {
  const raw = (endpoint || DEFAULT_ENDPOINT).trim()
  if (/^https?:\/\//i.test(raw)) return raw.replace(/\/+$/, "")
  if (/^\d+$/.test(raw)) return `http://127.0.0.1:${raw}`
  if (/^[\w.-]+:\d+$/.test(raw)) return `http://${raw}`
  return raw.replace(/\/+$/, "")
}

function storedDestination(): RelayDestination {
  try {
    const value = window.localStorage.getItem(DESTINATION_STORAGE_KEY)
    return value === "backend" || value === "frontend" || value === "chatgpt" ? value : "backend"
  } catch {
    return "backend"
  }
}

function scoreTarget(target: CdpTarget, query?: string): number {
  if (!target.webSocketDebuggerUrl) return -1000
  if (target.type !== "page") return -100
  if (!query) return 10
  const q = query.toLowerCase()
  const fields = [target.id, target.title, target.url, target.type].filter(Boolean).map((field) => String(field).toLowerCase())
  let score = 40
  for (const field of fields) {
    if (field === q) score += 100
    if (field.includes(q)) score += field.startsWith(q) ? 50 : 25
  }
  return score
}

async function fetchTargets(endpoint?: string): Promise<CdpTarget[]> {
  const response = await fetch(`${normalizeEndpoint(endpoint)}/json/list`, { cache: "no-store" })
  if (!response.ok) throw new Error(`CDP target list failed: ${response.status}`)
  return response.json() as Promise<CdpTarget[]>
}

async function findTarget(endpoint: string | undefined, query?: string): Promise<CdpTarget> {
  const targets = await fetchTargets(endpoint)
  const ranked = targets
    .map((target) => ({ target, score: scoreTarget(target, query) }))
    .filter((item) => item.score > 0)
    .sort((a, b) => b.score - a.score)
  const target = ranked[0]?.target
  if (!target?.webSocketDebuggerUrl) throw new Error(`No CDP page matched ${query || "default page"}`)
  return target
}

class BrowserCdpConnection {
  private ws: WebSocket | null = null
  private nextId = 1
  private openPromise: Promise<void> | null = null
  private readonly pending = new Map<number, {
    method: string
    resolve: (value: unknown) => void
    reject: (error: Error) => void
    timer: BrowserTimer
  }>()

  constructor(
    private readonly webSocketUrl: string,
    private readonly timeoutMs = DEFAULT_TIMEOUT_MS,
  ) {}

  open(): Promise<void> {
    if (this.openPromise) return this.openPromise
    this.openPromise = new Promise((resolve, reject) => {
      const ws = new WebSocket(this.webSocketUrl)
      this.ws = ws
      ws.addEventListener("open", () => resolve(), { once: true })
      ws.addEventListener("error", () => reject(new Error(`CDP websocket failed: ${this.webSocketUrl}`)), { once: true })
      ws.addEventListener("message", (event) => this.handleMessage(event.data))
      ws.addEventListener("close", () => this.closePending())
    })
    return this.openPromise
  }

  async send<T = unknown>(method: string, params: Record<string, unknown> = {}, timeoutMs = this.timeoutMs): Promise<T> {
    await this.open()
    const ws = this.ws
    if (!ws || ws.readyState !== WebSocket.OPEN) throw new Error("CDP websocket is not open")
    const id = this.nextId++
    return await new Promise<T>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id)
        reject(new Error(`CDP command timed out: ${method}`))
      }, timeoutMs)
      this.pending.set(id, { method, resolve: resolve as (value: unknown) => void, reject, timer })
      ws.send(JSON.stringify({ id, method, params }))
    })
  }

  close(): void {
    this.ws?.close()
    this.closePending()
  }

  private handleMessage(raw: unknown): void {
    const text = typeof raw === "string" ? raw : String(raw)
    const payload = JSON.parse(text) as CdpResponse
    if (!payload.id) return
    const pending = this.pending.get(payload.id)
    if (!pending) return
    clearTimeout(pending.timer)
    this.pending.delete(payload.id)
    if (payload.error) pending.reject(new Error(`CDP ${pending.method} failed: ${payload.error.message || payload.error.code}`))
    else pending.resolve(payload.result)
  }

  private closePending(): void {
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer)
      pending.reject(new Error(`CDP websocket closed while waiting for ${pending.method}`))
    }
    this.pending.clear()
  }
}

class BrowserCdpPage {
  constructor(
    private readonly connection: BrowserCdpConnection,
    readonly target: CdpTarget,
  ) {}

  async evaluate<T = unknown>(expression: string): Promise<T> {
    const result = await this.connection.send<{ result?: { value?: T }, exceptionDetails?: unknown }>("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
      userGesture: true,
      includeCommandLineAPI: true,
    })
    if (result.exceptionDetails) throw new Error(`Runtime.evaluate threw: ${JSON.stringify(result.exceptionDetails)}`)
    return result.result?.value as T
  }

  async focus(locator = "role=textbox"): Promise<unknown> {
    return await this.evaluate(`(${focusElementInPage.toString()})(${JSON.stringify(locator)})`)
  }

  async type(text: string, locator = "role=textbox"): Promise<void> {
    await this.focus(locator)
    await this.connection.send("Input.insertText", { text })
  }

  async press(key = "Enter"): Promise<void> {
    const code = key.length === 1 ? `Key${key.toUpperCase()}` : key
    const keyCode = key.length === 1 ? key.toUpperCase().charCodeAt(0) : 0
    await this.connection.send("Input.dispatchKeyEvent", { type: "keyDown", key, code, windowsVirtualKeyCode: keyCode, nativeVirtualKeyCode: keyCode })
    await this.connection.send("Input.dispatchKeyEvent", { type: "keyUp", key, code, windowsVirtualKeyCode: keyCode, nativeVirtualKeyCode: keyCode })
  }

  close(): void {
    this.connection.close()
  }
}

function focusElementInPage(locator: string) {
  const textOf = (el: Element) => [
    el.getAttribute("aria-label"),
    el.getAttribute("placeholder"),
    el.getAttribute("data-placeholder"),
    el.textContent,
  ].filter(Boolean).join(" ").trim()
  const visible = (el: Element) => {
    const rect = el.getBoundingClientRect()
    const style = getComputedStyle(el)
    return rect.width > 0 && rect.height > 0 && style.display !== "none" && style.visibility !== "hidden"
  }
  let candidates: Element[]
  if (locator.startsWith("role=")) candidates = Array.from(document.querySelectorAll(`[role="${CSS.escape(locator.slice(5))}"]`))
  else if (locator.startsWith("text=")) candidates = Array.from(document.querySelectorAll("button,a,input,textarea,[role=textbox],[contenteditable=true],div,span")).filter((el) => textOf(el).toLowerCase().includes(locator.slice(5).toLowerCase()))
  else if (locator.startsWith("placeholder=")) candidates = Array.from(document.querySelectorAll("input,textarea,[role=textbox],[contenteditable=true]")).filter((el) => textOf(el).toLowerCase().includes(locator.slice(12).toLowerCase()))
  else candidates = Array.from(document.querySelectorAll(locator))
  const el = candidates.find(visible) as HTMLElement | undefined
  if (!el) return { found: false }
  el.scrollIntoView({ block: "center", inline: "center" })
  el.focus({ preventScroll: true })
  if (el.isContentEditable) {
    const range = document.createRange()
    range.selectNodeContents(el)
    range.collapse(false)
    const selection = getSelection()
    selection?.removeAllRanges()
    selection?.addRange(range)
  }
  const rect = el.getBoundingClientRect()
  return { found: true, tag: el.tagName, role: el.getAttribute("role"), text: textOf(el).slice(0, 200), rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height } }
}

async function connectPage(endpoint: string | undefined, query?: string): Promise<BrowserCdpPage> {
  const target = await findTarget(endpoint, query)
  const connection = new BrowserCdpConnection(target.webSocketDebuggerUrl!)
  await connection.open()
  return new BrowserCdpPage(connection, target)
}

async function connectWebSocket(webSocketUrl: string, target: Partial<CdpTarget> = {}): Promise<BrowserCdpPage> {
  const connection = new BrowserCdpConnection(webSocketUrl)
  await connection.open()
  return new BrowserCdpPage(connection, {
    id: target.id || webSocketUrl,
    type: target.type || "page",
    title: target.title,
    url: target.url,
    webSocketDebuggerUrl: webSocketUrl,
  })
}

async function sendToChatGpt(message: string, options: { endpoint?: string; target?: string; webSocketUrl?: string; waitMs?: number } = {}) {
  const page = options.webSocketUrl
    ? await connectWebSocket(options.webSocketUrl, { url: options.target || "chatgpt.com" })
    : await connectPage(options.endpoint, options.target || "chatgpt.com")
  try {
    const before = await page.evaluate<Array<{ role: string | null; text: string }>>(`Array.from(document.querySelectorAll('[data-message-author-role]')).map((el) => ({ role: el.getAttribute('data-message-author-role'), text: el.innerText }))`)
    await page.type(message, "role=textbox")
    await page.press("Enter")
    await new Promise((resolve) => setTimeout(resolve, options.waitMs ?? 6000))
    const after = await page.evaluate<Array<{ role: string | null; text: string }>>(`Array.from(document.querySelectorAll('[data-message-author-role]')).map((el) => ({ role: el.getAttribute('data-message-author-role'), text: el.innerText }))`)
    const newMessages = after.slice(before.length)
    const response = [...newMessages].reverse().find((item) => item.role === "assistant") ?? [...after].reverse().find((item) => item.role === "assistant")
    return { ok: true, sent: message, response: response?.text ?? "", messages: newMessages }
  } finally {
    page.close()
  }
}

export function bootstrapBrowserCdpRelay(): BrowserCdpRelay | undefined {
  if (typeof window === "undefined") return
  const existing: BrowserCdpRelay | undefined = window.edgerunCdpRelay
  if (existing) return existing

  let backendBridge: BackendBridge | null = null
  const statusSubscribers = new Set<(state: BrowserCdpRelayState) => void>()
  const channel = "BroadcastChannel" in window ? new BroadcastChannel(FRONTEND_CHANNEL) : null
  channel?.addEventListener("message", (event) => {
    window.dispatchEvent(new CustomEvent(FRONTEND_EVENT, { detail: event.data }))
  })

  const getState = (): BrowserCdpRelayState => ({
    destination: relay.destination,
    frontend: "ready",
    backend: backendBridge ? "ready" : "offline",
    backendBridgeRegistered: Boolean(backendBridge),
    endpoint: relay.endpoint,
    updatedAtIso: new Date().toISOString(),
  })

  const publishStatus = () => {
    const state = getState()
    for (const handler of statusSubscribers) handler(state)
    window.dispatchEvent(new CustomEvent("edgerun:cdp-relay-status", { detail: state }))
  }

  const relay: BrowserCdpRelay = {
    endpoint: DEFAULT_ENDPOINT,
    eventName: FRONTEND_EVENT,
    destination: storedDestination(),
    setEndpoint(endpoint: string) {
      relay.endpoint = normalizeEndpoint(endpoint)
      publishStatus()
      return relay.endpoint
    },
    setDestination(destination: RelayDestination) {
      relay.destination = destination
      try {
        window.localStorage.setItem(DESTINATION_STORAGE_KEY, destination)
      } catch {
        // Ignore storage failures; the in-memory route still updates.
      }
      publishStatus()
      return relay.destination
    },
    setBackendBridge(bridge: BackendBridge) {
      backendBridge = bridge
      publishStatus()
    },
    status() {
      return getState()
    },
    subscribeStatus(handler: (state: BrowserCdpRelayState) => void) {
      statusSubscribers.add(handler)
      handler(getState())
      return () => {
        statusSubscribers.delete(handler)
      }
    },
    targets() {
      return fetchTargets(relay.endpoint)
    },
    connect(query?: string) {
      return connectPage(relay.endpoint, query)
    },
    connectWebSocket(webSocketUrl: string, target?: Partial<CdpTarget>) {
      return connectWebSocket(webSocketUrl, target)
    },
    sendToChatGpt(message: string, options?: { target?: string; webSocketUrl?: string; waitMs?: number }) {
      return sendToChatGpt(message, { endpoint: relay.endpoint, ...options })
    },
    sendToFrontend(message: string, detail: Record<string, unknown> = {}) {
      const payload = { message, createdAtIso: new Date().toISOString(), ...detail }
      window.dispatchEvent(new CustomEvent(FRONTEND_EVENT, { detail: payload }))
      channel?.postMessage(payload)
      return payload
    },
    subscribe(handler: (message: Record<string, unknown>) => void) {
      const listener = (event: Event) => handler((event as CustomEvent<Record<string, unknown>>).detail)
      window.addEventListener(FRONTEND_EVENT, listener)
      return () => window.removeEventListener(FRONTEND_EVENT, listener)
    },
    async relay(message: string, destination: RelayDestination = relay.destination, options: Record<string, unknown> = {}) {
      if (destination === "frontend") return relay.sendToFrontend(message, options)
      if (destination === "backend") {
        if (!backendBridge) throw new Error("No backend bridge registered. Call window.edgerunCdpRelay.setBackendBridge(fn).")
        return await backendBridge(message, options)
      }
      return await relay.sendToChatGpt(message, options as { target?: string; webSocketUrl?: string; waitMs?: number })
    },
  }

  window.edgerunCdpRelay = relay
  return relay
}

declare global {
  interface Window {
    edgerunCdpRelay?: BrowserCdpRelay
  }
}
