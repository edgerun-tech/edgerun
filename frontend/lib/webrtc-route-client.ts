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
  to_device_id: string
  kind: string
  sdp?: string
  candidate?: string
  sdp_mid?: string
  sdp_mline_index?: number
  metadata?: unknown
}

const CONTROL_BASE_STORAGE_KEY = 'edgerun.route.controlBase'

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
  if (typeof window === 'undefined') return 'http://127.0.0.1:8080'
  const fromStorage = window.localStorage.getItem(CONTROL_BASE_STORAGE_KEY)?.trim() || ''
  if (fromStorage.length > 0) return fromStorage.replace(/\/+$/, '')
  return window.location.origin.replace(/\/+$/, '')
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
  const base = controlBase || getRouteControlBase()
  const resolved = await resolveDeviceRoute(base, routeDeviceId)
  return resolved || ''
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
    this.ws.send(JSON.stringify(message))
  }

  close(): void {
    this.ws.close()
  }
}
