const DEFAULT_ENDPOINT = "http://127.0.0.1:9222"
const DEFAULT_TIMEOUT_MS = 10_000

export class CdpError extends Error {
  constructor(message, details = {}) {
    super(message)
    this.name = "CdpError"
    this.details = details
  }
}

export function normalizeEndpoint(endpoint = DEFAULT_ENDPOINT) {
  const raw = String(endpoint || DEFAULT_ENDPOINT).trim()
  if (!raw) return DEFAULT_ENDPOINT
  if (/^https?:\/\//i.test(raw)) return raw.replace(/\/+$/, "")
  if (/^\d+$/.test(raw)) return `http://127.0.0.1:${raw}`
  if (/^[\w.-]+:\d+$/.test(raw)) return `http://${raw}`
  return raw.replace(/\/+$/, "")
}

export async function fetchJson(endpoint, path, { signal } = {}) {
  const base = normalizeEndpoint(endpoint)
  const response = await fetch(`${base}${path}`, { signal })
  if (!response.ok) {
    throw new CdpError(`CDP HTTP ${response.status} for ${path}`, {
      endpoint: base,
      path,
      status: response.status,
      body: await response.text().catch(() => ""),
    })
  }
  return response.json()
}

export async function getBrowserVersion(endpoint = DEFAULT_ENDPOINT, options = {}) {
  return fetchJson(endpoint, "/json/version", options)
}

export async function listTargets(endpoint = DEFAULT_ENDPOINT, options = {}) {
  return fetchJson(endpoint, "/json/list", options)
}

export async function newTarget(endpoint = DEFAULT_ENDPOINT, url = "about:blank", options = {}) {
  const encoded = encodeURIComponent(url)
  return fetchJson(endpoint, `/json/new?${encoded}`, options)
}

export async function closeTarget(endpoint = DEFAULT_ENDPOINT, targetId, options = {}) {
  return fetchJson(endpoint, `/json/close/${encodeURIComponent(targetId)}`, options)
}

export async function activateTarget(endpoint = DEFAULT_ENDPOINT, targetId, options = {}) {
  return fetchJson(endpoint, `/json/activate/${encodeURIComponent(targetId)}`, options)
}

export function targetLabel(target) {
  const title = target?.title || "(untitled)"
  const url = target?.url || ""
  return `${target?.type || "target"} ${target?.id || ""} ${title} ${url}`.trim()
}

export function scoreTarget(target, query) {
  if (!query) return target.type === "page" ? 10 : 0
  const q = query.toLowerCase()
  const fields = [
    target.id,
    target.title,
    target.url,
    target.type,
    target.description,
  ].filter(Boolean).map((value) => String(value).toLowerCase())
  let score = 0
  for (const field of fields) {
    if (field === q) score += 100
    if (field.includes(q)) score += field.startsWith(q) ? 50 : 25
  }
  if (target.type === "page") score += 5
  return score
}

export async function findTarget(endpoint = DEFAULT_ENDPOINT, query, options = {}) {
  const targets = await listTargets(endpoint, options)
  const pages = targets.filter((target) => target.webSocketDebuggerUrl)
  const ranked = pages
    .map((target) => ({ target, score: scoreTarget(target, query) }))
    .filter((item) => item.score > 0 || !query)
    .sort((a, b) => b.score - a.score)
  return ranked[0]?.target ?? null
}

export class CdpConnection {
  constructor(webSocketUrl, options = {}) {
    this.webSocketUrl = webSocketUrl
    this.timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS
    this.nextId = 1
    this.pending = new Map()
    this.eventHandlers = new Map()
    this.openPromise = null
    this.closed = false
  }

  static async connect(webSocketUrl, options = {}) {
    const connection = new CdpConnection(webSocketUrl, options)
    await connection.open()
    return connection
  }

  async open() {
    if (this.openPromise) return this.openPromise
    this.openPromise = new Promise((resolve, reject) => {
      const WebSocketCtor = globalThis.WebSocket
      if (!WebSocketCtor) {
        reject(new CdpError("This Node runtime does not expose global WebSocket. Use Node 22+ or Bun."))
        return
      }
      const ws = new WebSocketCtor(this.webSocketUrl)
      this.ws = ws
      ws.addEventListener("open", () => resolve(this))
      ws.addEventListener("error", (event) => reject(new CdpError("CDP websocket failed", { event })), { once: true })
      ws.addEventListener("message", (event) => this.#onMessage(event.data))
      ws.addEventListener("close", () => this.#onClose())
    })
    return this.openPromise
  }

  async send(method, params = {}, options = {}) {
    await this.open()
    if (this.closed) throw new CdpError("CDP websocket is closed")
    const id = this.nextId++
    const timeoutMs = options.timeoutMs ?? this.timeoutMs
    const message = JSON.stringify({ id, method, params })
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id)
        reject(new CdpError(`CDP command timed out: ${method}`, { method, params, timeoutMs }))
      }, timeoutMs)
      this.pending.set(id, { method, resolve, reject, timer })
      this.ws.send(message)
    })
  }

  on(method, handler) {
    const handlers = this.eventHandlers.get(method) ?? new Set()
    handlers.add(handler)
    this.eventHandlers.set(method, handlers)
    return () => handlers.delete(handler)
  }

  close() {
    this.closed = true
    this.ws?.close()
  }

  #onMessage(raw) {
    const text = typeof raw === "string" ? raw : String(raw)
    let payload
    try {
      payload = JSON.parse(text)
    } catch (error) {
      this.#emit("CdpConnection.parseError", { raw: text, error })
      return
    }
    if (payload.id) {
      const pending = this.pending.get(payload.id)
      if (!pending) return
      clearTimeout(pending.timer)
      this.pending.delete(payload.id)
      if (payload.error) {
        pending.reject(new CdpError(`CDP ${pending.method} failed: ${payload.error.message}`, payload.error))
      } else {
        pending.resolve(payload.result)
      }
      return
    }
    if (payload.method) this.#emit(payload.method, payload.params ?? {})
  }

  #emit(method, params) {
    const handlers = this.eventHandlers.get(method)
    if (!handlers) return
    for (const handler of handlers) handler(params)
  }

  #onClose() {
    this.closed = true
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer)
      pending.reject(new CdpError(`CDP websocket closed while waiting for ${pending.method}`))
    }
    this.pending.clear()
  }
}

export async function connectToTarget(endpoint = DEFAULT_ENDPOINT, query, options = {}) {
  const target = await findTarget(endpoint, query, options)
  if (!target?.webSocketDebuggerUrl) {
    throw new CdpError(`No CDP target matched ${query ? JSON.stringify(query) : "the default page"}`, {
      endpoint: normalizeEndpoint(endpoint),
      query,
    })
  }
  const connection = await CdpConnection.connect(target.webSocketDebuggerUrl, options)
  return { target, connection }
}
