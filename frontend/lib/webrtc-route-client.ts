type RouteEntry = {
  device_id?: string
  owner_pubkey?: string
  reachable_urls?: string[]
  online?: boolean
}

type RouteResolveResponse = {
  ok?: boolean
  found?: boolean
  route?: RouteEntry | null
}

type OwnerRoutesResponse = {
  ok?: boolean
  owner_pubkey?: string
  devices?: RouteEntry[]
}

type SignalInboundMessage = {
  from_device_id: string
  kind: string
  sdp?: string
  candidate?: string
  sdp_mid?: string
  sdp_mline_index?: number
  metadata?: unknown
}

type SignalOutboundMessage = {
  to_device_id?: string
  to_owner_pubkey?: string
  kind: string
  sdp?: string
  candidate?: string
  sdp_mid?: string
  sdp_mline_index?: number
  metadata?: unknown
}

const CONTROL_BASE_STORAGE_KEY = 'edgerun.route.controlBase'
const LOCALHOST_NAMES = new Set(['127.0.0.1', 'localhost'])

function normalizeBase(value: string): string {
  return value.trim().replace(/\/+$/, '')
}

function localControlBaseCandidates(): string[] {
  if (typeof window === 'undefined') return ['http://127.0.0.1:8090', 'http://127.0.0.1:8080']
  const { protocol, hostname, port } = window.location
  if (!LOCALHOST_NAMES.has(hostname)) return []
  const scheme = protocol === 'https:' ? 'https:' : 'http:'
  const out: string[] = []
  if (port !== '8090') out.push(`${scheme}//${hostname}:8090`)
  if (port !== '8080') out.push(`${scheme}//${hostname}:8080`)
  if (port !== '8090') out.push(`${scheme}//127.0.0.1:8090`)
  return [...new Set(out.map(normalizeBase))]
}

function getControlBaseCandidates(): string[] {
  if (typeof window === 'undefined') return ['http://127.0.0.1:8090', 'http://127.0.0.1:8080']
  const fromStorage = normalizeBase(window.localStorage.getItem(CONTROL_BASE_STORAGE_KEY) || '')
  const origin = normalizeBase(window.location.origin)
  const local = localControlBaseCandidates()
  const candidates = [
    ...local,
    fromStorage,
    origin
  ].filter((value) => value.length > 0)
  return [...new Set(candidates)]
}

async function firstReachableControlBase(candidates: string[]): Promise<string | null> {
  for (const base of candidates) {
    const controller = new AbortController()
    const timeout = window.setTimeout(() => controller.abort(), 900)
    try {
      const url = new URL('/health', `${base}/`)
      const response = await fetch(url.toString(), { method: 'GET', signal: controller.signal })
      if (response.ok) {
        window.localStorage.setItem(CONTROL_BASE_STORAGE_KEY, base)
        return base
      }
    } catch {
      // try next
    } finally {
      window.clearTimeout(timeout)
    }
  }
  return null
}

export function parseRouteDeviceId(target: string): string | null {
  const value = target.trim()
  if (!value) return null
  const routePrefix = 'route://'
  const edgerunPrefix = 'edgerun://'
  if (value.startsWith(routePrefix)) return value.slice(routePrefix.length).trim() || null
  if (value.startsWith(edgerunPrefix)) return value.slice(edgerunPrefix.length).trim() || null
  return null
}

export function getRouteControlBase(): string {
  if (typeof window === 'undefined') return 'http://127.0.0.1:8090'
  const candidates = getControlBaseCandidates()
  return candidates[0] || normalizeBase(window.location.origin)
}

export async function resolveDeviceRoute(controlBase: string, deviceId: string): Promise<string | null> {
  const trimmedBase = controlBase.trim().replace(/\/+$/, '')
  const trimmedDeviceId = deviceId.trim()
  if (!trimmedBase || !trimmedDeviceId) return null
  try {
    const url = new URL(`/v1/route/resolve/${encodeURIComponent(trimmedDeviceId)}`, `${trimmedBase}/`)
    const response = await fetch(url.toString(), { method: 'GET' })
    if (!response.ok) return null
    const body = await response.json() as RouteResolveResponse
    if (!body.ok || !body.found) return null
    const reachable = Array.isArray(body.route?.reachable_urls) ? body.route?.reachable_urls : []
    const first = reachable.find((item) => typeof item === 'string' && item.trim().length > 0)
    return first?.trim() || null
  } catch {
    return null
  }
}

export async function resolveTerminalBaseUrl(input: string, controlBase?: string): Promise<string> {
  const target = input.trim()
  if (!target) return ''
  const routeDeviceId = parseRouteDeviceId(target)
  if (!routeDeviceId) return target
  const bases = controlBase ? [normalizeBase(controlBase)] : getControlBaseCandidates()
  for (const base of bases) {
    const resolved = await resolveDeviceRoute(base, routeDeviceId)
    if (resolved) {
      if (typeof window !== 'undefined') window.localStorage.setItem(CONTROL_BASE_STORAGE_KEY, base)
      return resolved
    }
  }
  const discovered = typeof window !== 'undefined'
    ? await firstReachableControlBase(getControlBaseCandidates())
    : null
  if (discovered) {
    const resolved = await resolveDeviceRoute(discovered, routeDeviceId)
    if (resolved) return resolved
  }
  return ''
}

export async function resolveOwnerRoutes(controlBase: string, ownerPubkey: string): Promise<RouteEntry[]> {
  const trimmedBase = normalizeBase(controlBase)
  const trimmedOwner = ownerPubkey.trim()
  if (!trimmedOwner) return []
  const bases = trimmedBase ? [trimmedBase, ...getControlBaseCandidates()] : getControlBaseCandidates()
  for (const base of [...new Set(bases)]) {
    try {
      const url = new URL(`/v1/route/owner/${encodeURIComponent(trimmedOwner)}`, `${base}/`)
      const response = await fetch(url.toString(), { method: 'GET' })
      if (!response.ok) continue
      const body = await response.json() as OwnerRoutesResponse
      if (!body.ok) continue
      if (typeof window !== 'undefined') window.localStorage.setItem(CONTROL_BASE_STORAGE_KEY, base)
      return Array.isArray(body.devices) ? body.devices : []
    } catch {
      // try next
    }
  }
  return []
}

export class WebRtcSignalClient {
  private readonly ws: WebSocket

  onMessage?: (message: SignalInboundMessage) => void
  onClose?: () => void
  onOpen?: () => void

  constructor(controlBase: string, deviceId: string) {
    const base = controlBase.trim().replace(/\/+$/, '')
    const url = new URL('/v1/webrtc/ws', `${base}/`)
    url.searchParams.set('device_id', deviceId)
    const wsProto = url.protocol === 'https:' ? 'wss:' : 'ws:'
    url.protocol = wsProto
    this.ws = new WebSocket(url.toString())
    this.ws.addEventListener('open', () => this.onOpen?.())
    this.ws.addEventListener('close', () => this.onClose?.())
    this.ws.addEventListener('message', (event) => {
      if (typeof event.data !== 'string') return
      try {
        const payload = JSON.parse(event.data) as SignalInboundMessage
        this.onMessage?.(payload)
      } catch {
        // ignore malformed signaling payloads
      }
    })
  }

  send(message: SignalOutboundMessage): void {
    if (this.ws.readyState !== WebSocket.OPEN) return
    try {
      this.ws.send(JSON.stringify(message))
    } catch {
      // ignore transient closed-state sends
    }
  }

  close(): void {
    this.ws.close()
  }
}
