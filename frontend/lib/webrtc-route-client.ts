// SPDX-License-Identifier: Apache-2.0
import { SchedulerControlWsClient } from './scheduler-control-ws'

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
const controlClients = new Map<string, SchedulerControlWsClient>()

function normalizeBase(value: string): string {
  return value.trim().replace(/\/+$/, '')
}

function configuredApiBase(): string {
  if (typeof window === 'undefined') return ''
  const explicit = normalizeBase(String((window as any).__EDGERUN_API_BASE || ''))
  if (explicit) return explicit
  if (window.location.hostname === 'www.edgerun.tech') return 'https://api.edgerun.tech'
  return ''
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
  const configured = configuredApiBase()
  const origin = normalizeBase(window.location.origin)
  const local = localControlBaseCandidates()
  const candidates = [
    configured,
    ...local,
    fromStorage,
    origin
  ].filter((value) => value.length > 0)
  return [...new Set(candidates)]
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
    const body = await getControlClient(trimmedBase).request<RouteResolveResponse>(
      'route.resolve',
      { device_id: trimmedDeviceId }
    )
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
  const base = normalizeBase(controlBase || getRouteControlBase())
  if (!base) return ''
  const resolved = await resolveDeviceRoute(base, routeDeviceId)
  if (resolved) {
    if (typeof window !== 'undefined') window.localStorage.setItem(CONTROL_BASE_STORAGE_KEY, base)
    return resolved
  }
  return ''
}

export async function resolveOwnerRoutes(controlBase: string, ownerPubkey: string): Promise<RouteEntry[]> {
  const base = normalizeBase(controlBase || getRouteControlBase())
  const trimmedOwner = ownerPubkey.trim()
  if (!trimmedOwner || !base) return []
  try {
    const body = await getControlClient(base).request<OwnerRoutesResponse>(
      'route.owner',
      { owner_pubkey: trimmedOwner }
    )
    if (!body.ok) return []
    if (typeof window !== 'undefined') window.localStorage.setItem(CONTROL_BASE_STORAGE_KEY, base)
    return Array.isArray(body.devices) ? body.devices : []
  } catch {
    // Explicit user action failed on current control channel.
  }
  return []
}

function getControlClient(controlBase: string): SchedulerControlWsClient {
  const normalized = normalizeBase(controlBase)
  const existing = controlClients.get(normalized)
  if (existing) return existing
  const created = new SchedulerControlWsClient(normalized, 'route-client')
  controlClients.set(normalized, created)
  return created
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
