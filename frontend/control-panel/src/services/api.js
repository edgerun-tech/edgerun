// SPDX-License-Identifier: Apache-2.0
function buildWsUrl() {
  const url = new URL('/api/ws', window.location.origin)
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
  return url.toString()
}

const CONTROL_PANEL_WS_PROTOCOL_VERSION = 'edgerun.control_panel.ws.v1'

let socket = null
let connectPromise = null
const pending = new Map()
const statusSubscribers = new Set()
let statusFeedStarted = false
let statusFeedActive = false
let statusFeedTimer = null
let statusFeedBusy = false
let statusFeedToken = ''
let statusFeedPollMs = 2000
let statusPushSeenAt = 0

function isRecord(value) {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function toSafeInt(value, fallback = 0) {
  const n = Number(value)
  if (!Number.isFinite(n)) return fallback
  return Math.max(0, Math.floor(n))
}

function normalizeTask(raw) {
  if (!isRecord(raw)) return null
  const task = String(raw.task || '').trim()
  if (!task) return null
  return {
    task,
    state: String(raw.state || 'idle'),
    runs: toSafeInt(raw.runs, 0),
    last_exit: raw.last_exit ?? null,
    last_output: String(raw.last_output || '')
  }
}

function normalizeStatusBody(raw) {
  if (!isRecord(raw) || !Array.isArray(raw.tasks)) return null
  const tasks = raw.tasks.map(normalizeTask).filter(Boolean)
  return { tasks }
}

function parseServerMessage(raw) {
  if (!isRecord(raw)) return { kind: 'unknown' }
  const requestId = String(raw.request_id || '').trim()
  if (requestId) {
    if (typeof raw.ok !== 'boolean') {
      return { kind: 'response', requestId, ok: false, error: 'invalid response: missing ok boolean' }
    }
    if (raw.ok) {
      return { kind: 'response', requestId, ok: true, data: raw.data }
    }
    return { kind: 'response', requestId, ok: false, error: String(raw.error || 'request failed') }
  }

  // Canonical push event: {"event":"status","data":{"tasks":[...]}}
  if (raw.event === 'status') {
    const body = normalizeStatusBody(raw.data)
    if (!body) return { kind: 'unknown' }
    return { kind: 'status_push', body }
  }

  // Backward-compatible forms accepted by existing integrations.
  const direct = normalizeStatusBody(raw)
  if (direct) return { kind: 'status_push', body: direct }
  if (raw.type === 'status' || raw.op === 'status') {
    const body = normalizeStatusBody(raw.data)
    if (body) return { kind: 'status_push', body }
  }
  return { kind: 'unknown' }
}

async function ensureSocket() {
  if (socket && socket.readyState === WebSocket.OPEN) return socket
  if (connectPromise) return connectPromise
  connectPromise = new Promise((resolve, reject) => {
    const ws = new WebSocket(buildWsUrl())
    let settled = false

    ws.addEventListener('open', () => {
      settled = true
      socket = ws
      resolve(ws)
    }, { once: true })

    ws.addEventListener('error', () => {
      if (settled) return
      settled = true
      connectPromise = null
      reject(new Error('control panel websocket unavailable'))
    }, { once: true })

    ws.addEventListener('close', () => {
      socket = null
      connectPromise = null
      for (const [id, entry] of pending) {
        clearTimeout(entry.timeoutId)
        entry.reject(new Error(`request closed: ${id}`))
      }
      pending.clear()
      clearStatusFeedTimer()
      if (statusFeedStarted) {
        statusFeedActive = true
        scheduleStatusFeed(statusFeedPollMs)
      }
    })

    ws.addEventListener('message', (event) => {
      if (typeof event.data !== 'string') return
      let payload = null
      try {
        payload = JSON.parse(event.data)
      } catch {
        return
      }
      const parsed = parseServerMessage(payload)
      if (parsed.kind === 'response') {
        const entry = pending.get(parsed.requestId)
        if (!entry) return
        pending.delete(parsed.requestId)
        clearTimeout(entry.timeoutId)
        if (parsed.ok) {
          const data = parsed.data || {}
          maybeEmitStatus(data)
          entry.resolve(data)
          return
        }
        entry.reject(new Error(parsed.error))
        return
      }
      const pushed = parsed.kind === 'status_push' ? maybeEmitStatus(parsed.body) : false
      if (pushed) statusPushSeenAt = Date.now()
    })
  })
  try {
    return await connectPromise
  } finally {
    connectPromise = null
  }
}

async function wsCall(op, payload, token = '') {
  const ws = await ensureSocket()
  const requestId = `${Date.now()}-${Math.floor(Math.random() * 100000)}`
  return new Promise((resolve, reject) => {
    const timeoutId = window.setTimeout(() => {
      pending.delete(requestId)
      reject(new Error(`${op} timed out`))
    }, 5000)
    pending.set(requestId, { resolve, reject, timeoutId })
    ws.send(JSON.stringify({
      request_id: requestId,
      op,
      protocol: CONTROL_PANEL_WS_PROTOCOL_VERSION,
      token,
      payload
    }))
  })
}

export async function fetchStatus(token = '') {
  return wsCall('status', {}, token)
}

export async function runTask(task, token = '') {
  await wsCall('run', { task }, token)
}

function maybeEmitStatus(payload) {
  const body = normalizeStatusBody(payload)
  if (!body) return false
  for (const listener of statusSubscribers) {
    try {
      listener(body)
    } catch {
      // isolate subscriber errors
    }
  }
  return true
}

function clearStatusFeedTimer() {
  if (statusFeedTimer !== null) {
    window.clearTimeout(statusFeedTimer)
    statusFeedTimer = null
  }
}

function scheduleStatusFeed(delayMs) {
  clearStatusFeedTimer()
  if (!statusFeedActive) return
  statusFeedTimer = window.setTimeout(runStatusFeedTick, delayMs)
}

async function runStatusFeedTick() {
  if (!statusFeedActive || statusFeedBusy) return
  statusFeedBusy = true
  try {
    await ensureSocket()
    const now = Date.now()
    const pushFresh = now - statusPushSeenAt < Math.max(3000, statusFeedPollMs * 2)
    if (!pushFresh) {
      await fetchStatus(statusFeedToken)
    }
  } catch {
    // retry with backoff if socket is unavailable
  } finally {
    statusFeedBusy = false
    scheduleStatusFeed(statusFeedPollMs)
  }
}

export function subscribeStatus(listener) {
  statusSubscribers.add(listener)
  return () => statusSubscribers.delete(listener)
}

export function startStatusFeed({ token = '', pollMs = 2000 } = {}) {
  statusFeedToken = token
  statusFeedPollMs = Number.isFinite(pollMs) && pollMs > 0 ? Math.floor(pollMs) : 2000
  if (statusFeedStarted) return
  statusFeedStarted = true
  statusFeedActive = true
  runStatusFeedTick()
}

export function stopStatusFeed() {
  statusFeedActive = false
  statusFeedStarted = false
  statusFeedBusy = false
  clearStatusFeedTimer()
}
