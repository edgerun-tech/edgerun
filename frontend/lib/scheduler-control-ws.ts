// SPDX-License-Identifier: Apache-2.0
type ControlWsResponse = {
  request_id?: string
  ok?: boolean
  data?: unknown
  error?: string
  status?: number
}

type PendingRequest = {
  resolve: (value: unknown) => void
  reject: (error: Error) => void
  timeoutId: number
}

type ControlWsMockRequest = {
  controlBase: string
  clientId: string
  requestId: string
  op: string
  payload: unknown
}

type ControlWsMockResponder = (request: ControlWsMockRequest) => Promise<unknown> | unknown

declare global {
  var __EDGERUN_CONTROL_WS_MOCK__: ControlWsMockResponder | undefined
  var __EDGERUN_CONTROL_WS_MOCK_ENABLED__: boolean | undefined
}

function toControlWsUrl(controlBase: string, clientId: string): string {
  const base = controlBase.trim().replace(/\/+$/, '')
  const url = new URL('/v1/control/ws', `${base}/`)
  url.searchParams.set('client_id', clientId)
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
  return url.toString()
}

function randomClientSuffix(): string {
  const globalCrypto = (globalThis as any).crypto as Crypto | undefined
  if (typeof globalCrypto?.randomUUID === 'function') return globalCrypto.randomUUID()
  return `${Date.now()}-${Math.floor(Math.random() * 1_000_000)}`
}

export class SchedulerControlWsClient {
  private readonly controlBase: string
  private readonly clientId: string
  private socket: WebSocket | null = null
  private connectPromise: Promise<WebSocket> | null = null
  private readonly pending = new Map<string, PendingRequest>()

  constructor(controlBase: string, clientIdPrefix = 'web-control') {
    this.controlBase = controlBase.trim().replace(/\/+$/, '')
    this.clientId = `${clientIdPrefix}-${randomClientSuffix()}`
  }

  async request<T>(op: string, payload: unknown, timeoutMs = 6000): Promise<T> {
    if (!this.controlBase) throw new Error('control base is required')
    const mock = this.resolveMock()
    if (mock) {
      const requestId = `${Date.now()}-${Math.floor(Math.random() * 1_000_000)}`
      const mocked = await Promise.resolve(
        mock({
          controlBase: this.controlBase,
          clientId: this.clientId,
          requestId,
          op,
          payload
        })
      )
      return mocked as T
    }
    const socket = await this.ensureSocket()
    const requestId = `${Date.now()}-${Math.floor(Math.random() * 1_000_000)}`
    const body = JSON.stringify({
      request_id: requestId,
      op,
      payload
    })
    return new Promise<T>((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        this.pending.delete(requestId)
        reject(new Error(`${op} timed out`))
      }, timeoutMs) as unknown as number

      this.pending.set(requestId, { resolve: resolve as (value: unknown) => void, reject, timeoutId })
      try {
        socket.send(body)
      } catch (err) {
        clearTimeout(timeoutId)
        this.pending.delete(requestId)
        reject(err instanceof Error ? err : new Error(String(err)))
      }
    })
  }

  close(): void {
    if (this.socket) {
      try {
        this.socket.close()
      } catch {
        // ignore close errors
      }
    }
    this.socket = null
    this.connectPromise = null
    for (const [requestId, pending] of this.pending) {
      clearTimeout(pending.timeoutId)
      pending.reject(new Error(`request closed: ${requestId}`))
    }
    this.pending.clear()
  }

  private async ensureSocket(): Promise<WebSocket> {
    if (this.socket && this.socket.readyState === WebSocket.OPEN) return this.socket
    if (this.connectPromise) return this.connectPromise

    this.connectPromise = new Promise<WebSocket>((resolve, reject) => {
      const ws = new WebSocket(toControlWsUrl(this.controlBase, this.clientId))
      let settled = false

      ws.addEventListener('open', () => {
        settled = true
        this.socket = ws
        resolve(ws)
      }, { once: true })

      ws.addEventListener('error', () => {
        if (settled) return
        settled = true
        this.connectPromise = null
        reject(new Error('control ws connection failed'))
      }, { once: true })

      ws.addEventListener('close', () => {
        this.socket = null
        this.connectPromise = null
        for (const [requestId, pending] of this.pending) {
          clearTimeout(pending.timeoutId)
          pending.reject(new Error(`control ws closed: ${requestId}`))
        }
        this.pending.clear()
      })

      ws.addEventListener('message', (event) => {
        if (typeof event.data !== 'string') return
        let payload: ControlWsResponse | null = null
        try {
          payload = JSON.parse(event.data) as ControlWsResponse
        } catch {
          return
        }
        if (!payload) return
        const requestId = String(payload.request_id || '').trim()
        if (!requestId) return
        const pending = this.pending.get(requestId)
        if (!pending) return
        this.pending.delete(requestId)
        clearTimeout(pending.timeoutId)
        if (payload.ok) {
          pending.resolve(payload.data)
          return
        }
        const status = typeof payload.status === 'number' ? ` (${payload.status})` : ''
        const message = String(payload.error || `control request failed${status}`)
        pending.reject(new Error(message))
      })
    })

    try {
      return await this.connectPromise
    } finally {
      this.connectPromise = null
    }
  }

  private resolveMock(): ControlWsMockResponder | null {
    const scope = globalThis as {
      __EDGERUN_CONTROL_WS_MOCK__?: unknown
      __EDGERUN_CONTROL_WS_MOCK_ENABLED__?: unknown
    }
    if (scope.__EDGERUN_CONTROL_WS_MOCK_ENABLED__ !== true) return null
    const candidate = scope.__EDGERUN_CONTROL_WS_MOCK__
    if (typeof candidate !== 'function') return null
    return candidate as ControlWsMockResponder
  }
}
