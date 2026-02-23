// SPDX-License-Identifier: Apache-2.0
type RpcResponse = {
  id?: number
  result?: unknown
  error?: { code: number; message: string }
  method?: string
  params?: { result?: unknown; subscription?: number }
}

type PendingRequest = {
  resolve: (value: unknown) => void
  reject: (reason?: unknown) => void
  timeoutId: number
}

type ActiveSubscription = {
  method: string
  params: unknown[]
  notificationMethod: string
  callback: (value: unknown) => void
  remoteId: number | null
}

type SubscriptionAck = {
  localId: string
}

export type SolanaRpcWsLease = {
  client: SolanaRpcWsClient
  release: () => void
}

const REQUEST_TIMEOUT_MS = 12_000
const RECONNECT_MS = 1_200
const WS_CLOSE_NORMAL = 1000
const DEFAULT_WS_URL = 'wss://api.devnet.solana.com'
const DEFAULT_HTTP_URL = 'https://api.devnet.solana.com'

function toWsUrl(rpcUrl: string): string {
  try {
    const parsed = new URL(rpcUrl)
    if (parsed.protocol === 'ws:' || parsed.protocol === 'wss:') {
      return parsed.toString()
    }
    if (parsed.protocol === 'http:' || parsed.protocol === 'https:') {
      const ws = new URL(parsed.toString())
      ws.protocol = parsed.protocol === 'https:' ? 'wss:' : 'ws:'
      if (parsed.port) {
        const value = Number(parsed.port)
        if (Number.isFinite(value) && value > 0) ws.port = String(value + 1)
      }
      return ws.toString()
    }
  } catch {
    // ignore and fallback
  }
  return DEFAULT_WS_URL
}

function toHttpUrl(rpcUrl: string): string {
  try {
    const parsed = new URL(rpcUrl)
    if (parsed.protocol === 'http:' || parsed.protocol === 'https:') {
      return parsed.toString()
    }
    if (parsed.protocol === 'ws:' || parsed.protocol === 'wss:') {
      const http = new URL(parsed.toString())
      http.protocol = parsed.protocol === 'wss:' ? 'https:' : 'http:'
      if (parsed.port) {
        const value = Number(parsed.port)
        if (Number.isFinite(value) && value > 1) http.port = String(value - 1)
      }
      return http.toString()
    }
  } catch {
    // ignore and fallback
  }
  return DEFAULT_HTTP_URL
}

function unsubscribeMethod(subscribeMethod: string): string {
  return subscribeMethod.replace(/Subscribe$/, 'Unsubscribe')
}

export class SolanaRpcWsClient {
  private wsUrl: string
  private httpUrl: string
  private socket: WebSocket | null = null
  private connecting: Promise<void> | null = null
  private nextRequestId = 1
  private nextLocalSubId = 1
  private closed = false
  private pending = new Map<number, PendingRequest>()
  private subscriptions = new Map<string, ActiveSubscription>()
  private remoteToLocal = new Map<number, string>()
  private subscriptionAck = new Map<number, SubscriptionAck>()
  private reconnectTimer: number | null = null

  constructor(rpcUrl: string) {
    this.wsUrl = toWsUrl(rpcUrl)
    this.httpUrl = toHttpUrl(rpcUrl)
  }

  private clearPending(reason: string): void {
    const error = new Error(reason)
    for (const [id, entry] of this.pending) {
      window.clearTimeout(entry.timeoutId)
      entry.reject(error)
      this.pending.delete(id)
    }
    for (const [id, ack] of this.subscriptionAck) {
      const sub = this.subscriptions.get(ack.localId)
      if (sub) sub.remoteId = null
      this.subscriptionAck.delete(id)
    }
    this.remoteToLocal.clear()
  }

  private scheduleReconnect(): void {
    if (this.closed || this.reconnectTimer !== null) return
    this.reconnectTimer = window.setTimeout(() => {
      this.reconnectTimer = null
      void this.ensureConnected().catch(() => {
        this.scheduleReconnect()
      })
    }, RECONNECT_MS)
  }

  private onMessage(raw: MessageEvent): void {
    let payload: RpcResponse | null = null
    try {
      payload = JSON.parse(String(raw.data)) as RpcResponse
    } catch {
      return
    }
    if (!payload) return

    if (typeof payload.id === 'number') {
      const pending = this.pending.get(payload.id)
      if (pending) {
        window.clearTimeout(pending.timeoutId)
        this.pending.delete(payload.id)
        if (payload.error) pending.reject(new Error(payload.error.message || 'rpc_error'))
        else pending.resolve(payload.result)
        return
      }

      const ack = this.subscriptionAck.get(payload.id)
      if (ack) {
        this.subscriptionAck.delete(payload.id)
        const remoteId = typeof payload.result === 'number' ? payload.result : null
        const sub = this.subscriptions.get(ack.localId)
        if (sub) sub.remoteId = remoteId
        if (remoteId !== null) this.remoteToLocal.set(remoteId, ack.localId)
      }
      return
    }

    const remoteId = payload.params?.subscription
    if (typeof remoteId !== 'number') return
    const localId = this.remoteToLocal.get(remoteId)
    if (!localId) return
    const sub = this.subscriptions.get(localId)
    if (!sub) return
    if (payload.method && payload.method !== sub.notificationMethod) return
    sub.callback(payload.params?.result)
  }

  private async openSocket(): Promise<void> {
    if (this.closed) throw new Error('rpc_ws_closed')
    if (this.socket && this.socket.readyState === WebSocket.OPEN) return
    if (this.connecting) return this.connecting

    this.connecting = new Promise<void>((resolve, reject) => {
      try {
        const ws = new WebSocket(this.wsUrl)
        this.socket = ws

        ws.onopen = () => {
          this.connecting = null
          resolve()
          for (const [localId, sub] of this.subscriptions) {
            sub.remoteId = null
            this.sendSubscription(localId, sub)
          }
        }

        ws.onmessage = (event: MessageEvent) => {
          this.onMessage(event)
        }

        ws.onerror = () => {
          if (this.connecting) {
            this.connecting = null
            reject(new Error('rpc_ws_connect_error'))
          }
        }

        ws.onclose = () => {
          this.socket = null
          this.clearPending('rpc_ws_disconnected')
          this.connecting = null
          this.scheduleReconnect()
        }
      } catch (error) {
        this.connecting = null
        reject(error)
      }
    })

    return this.connecting
  }

  async ensureConnected(): Promise<void> {
    return this.openSocket()
  }

  private sendRaw(message: string): void {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
      throw new Error('rpc_ws_not_open')
    }
    this.socket.send(message)
  }

  private async requestOverWs<T>(method: string, params: unknown[] = []): Promise<T> {
    await this.ensureConnected()
    const id = this.nextRequestId++
    const payload = JSON.stringify({
      jsonrpc: '2.0',
      id,
      method,
      params
    })
    return new Promise<T>((resolve, reject) => {
      const timeoutId = window.setTimeout(() => {
        this.pending.delete(id)
        reject(new Error(`rpc_timeout_${method}`))
      }, REQUEST_TIMEOUT_MS)
      this.pending.set(id, { resolve: (value) => resolve(value as T), reject, timeoutId })
      try {
        this.sendRaw(payload)
      } catch (error) {
        window.clearTimeout(timeoutId)
        this.pending.delete(id)
        reject(error)
      }
    })
  }

  private async requestOverHttp<T>(method: string, params: unknown[] = []): Promise<T> {
    const controller = new AbortController()
    const timeoutId = window.setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS)
    try {
      const response = await fetch(this.httpUrl, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: this.nextRequestId++,
          method,
          params
        }),
        signal: controller.signal
      })
      if (!response.ok) {
        throw new Error(`rpc_http_status_${response.status}`)
      }
      const payload = await response.json() as RpcResponse
      if (payload.error) throw new Error(payload.error.message || 'rpc_error')
      return payload.result as T
    } catch (error) {
      if (error instanceof Error && error.name === 'AbortError') {
        throw new Error(`rpc_timeout_${method}`)
      }
      throw error
    } finally {
      window.clearTimeout(timeoutId)
    }
  }

  async request<T>(method: string, params: unknown[] = []): Promise<T> {
    if (/Subscribe$|Unsubscribe$/.test(method)) {
      return this.requestOverWs<T>(method, params)
    }
    return this.requestOverHttp<T>(method, params)
  }

  private sendSubscription(localId: string, sub: ActiveSubscription): void {
    const id = this.nextRequestId++
    this.subscriptionAck.set(id, { localId })
    const payload = JSON.stringify({
      jsonrpc: '2.0',
      id,
      method: sub.method,
      params: sub.params
    })
    this.sendRaw(payload)
  }

  async subscribe(
    method: string,
    params: unknown[],
    notificationMethod: string,
    callback: (value: unknown) => void
  ): Promise<() => void> {
    await this.ensureConnected()
    const localId = `sub-${this.nextLocalSubId++}`
    const sub: ActiveSubscription = {
      method,
      params,
      notificationMethod,
      callback,
      remoteId: null
    }
    this.subscriptions.set(localId, sub)
    this.sendSubscription(localId, sub)

    return () => {
      const current = this.subscriptions.get(localId)
      if (!current) return
      this.subscriptions.delete(localId)
      if (current.remoteId !== null) {
        this.remoteToLocal.delete(current.remoteId)
        const id = this.nextRequestId++
        try {
          this.sendRaw(JSON.stringify({
            jsonrpc: '2.0',
            id,
            method: unsubscribeMethod(current.method),
            params: [current.remoteId]
          }))
        } catch {
          // ignore unsub failures
        }
      }
    }
  }

  close(): void {
    this.closed = true
    if (this.reconnectTimer !== null) {
      window.clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
    this.clearPending('rpc_ws_closed')
    for (const key of [...this.subscriptions.keys()]) this.subscriptions.delete(key)
    if (this.socket && this.socket.readyState < WebSocket.CLOSING) {
      this.socket.close(WS_CLOSE_NORMAL, 'client_closed')
    }
    this.socket = null
  }
}

const cache = new Map<string, { client: SolanaRpcWsClient; refs: number }>()

export function acquireSolanaRpcWsClient(rpcUrl: string): SolanaRpcWsLease {
  const key = toWsUrl(rpcUrl)
  const found = cache.get(key)
  if (found) {
    found.refs += 1
    return {
      client: found.client,
      release: () => {
        const entry = cache.get(key)
        if (!entry) return
        entry.refs -= 1
        if (entry.refs <= 0) {
          entry.client.close()
          cache.delete(key)
        }
      }
    }
  }

  const client = new SolanaRpcWsClient(rpcUrl)
  cache.set(key, { client, refs: 1 })
  return {
    client,
    release: () => {
      const entry = cache.get(key)
      if (!entry) return
      entry.refs -= 1
      if (entry.refs <= 0) {
        entry.client.close()
        cache.delete(key)
      }
    }
  }
}
