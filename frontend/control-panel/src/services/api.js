// SPDX-License-Identifier: Apache-2.0
function buildWsUrl() {
  const url = new URL('/api/ws', window.location.origin)
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
  return url.toString()
}

let socket = null
let connectPromise = null
const pending = new Map()

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
    })

    ws.addEventListener('message', (event) => {
      if (typeof event.data !== 'string') return
      let payload = null
      try {
        payload = JSON.parse(event.data)
      } catch {
        return
      }
      const id = String(payload?.request_id || '').trim()
      if (!id) return
      const entry = pending.get(id)
      if (!entry) return
      pending.delete(id)
      clearTimeout(entry.timeoutId)
      if (payload.ok) {
        entry.resolve(payload.data || {})
        return
      }
      entry.reject(new Error(String(payload.error || 'request failed')))
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

