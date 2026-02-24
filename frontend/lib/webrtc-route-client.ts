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
  metadata?: string
  error?: string
}

type SignalOutboundMessage = {
  to_device_id?: string
  to_owner_pubkey?: string
  kind: string
  sdp?: string
  candidate?: string
  sdp_mid?: string
  sdp_mline_index?: number
  metadata?: string
}

export const DEFAULT_ROUTE_CONTROL_BASE = 'https://api.edgerun.tech'
const CONTROL_BASE_STORAGE_KEY = 'edgerun.route.controlBase'
const CONTROL_PROBE_TIMEOUT_MS = 2000
const CONTROL_STATUS_CACHE_TTL_MS = 5000
const LOCALHOST_NAMES = new Set(['127.0.0.1', 'localhost', '::1'])
const controlClients = new Map<string, SchedulerControlWsClient>()

export type RouteControlSource = 'configured' | 'storage' | 'local' | 'origin' | 'default'

type RouteControlCandidate = {
  base: string
  source: RouteControlSource
}

type RouteControlCachedStatus = {
  base: string
  source: RouteControlSource
  selectedAt: number
  httpReachable: boolean
  httpStatus: number | null
  httpLatencyMs: number | null
  controlWsReachable: boolean
  controlWsLatencyMs: number | null
}

export type RouteControlSelection = {
  candidates: RouteControlCandidate[]
  selected: string
  source: RouteControlSource
  selectedFromStorage: boolean
}

export type RouteControlProbeStatus = {
  base: string
  source: RouteControlSource
  checkedAt: number
  httpReachable: boolean
  httpStatus: number | null
  httpLatencyMs: number | null
  controlWsReachable: boolean
  controlWsLatencyMs: number | null
  error: string
}

function normalizeBase(value: string): string {
  return value.trim().replace(/\/+$/, '')
}

function withScheme(raw: string): string {
  const trimmed = normalizeBase(raw)
  if (!trimmed) return ''
  if (/^[a-z][a-z\d+\-.]*:\/\//i.test(trimmed)) return trimmed
  return `https://${trimmed}`
}

export function normalizeControlBase(value: string): string {
  const raw = withScheme(value).trim()
  if (!raw) return ''
  try {
    const parsed = new URL(raw)
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') return ''
    if (!parsed.hostname) return ''
    return `${parsed.protocol}//${parsed.host}`
  } catch {
    return ''
  }
}

function isLocalHostHost(host: string): boolean {
  return LOCALHOST_NAMES.has(host.toLowerCase())
}

function isBrowserLocalContext(): boolean {
  if (typeof window === 'undefined') return false
  return isLocalHostHost(window.location.hostname)
}

function toRouteControlCandidate(value: string, source: RouteControlSource): RouteControlCandidate | null {
  const normalized = normalizeControlBase(value)
  if (!normalized) return null
  try {
    const parsed = new URL(`${normalized}/`)
    const isLocal = isLocalHostHost(parsed.hostname)
    if ((source === 'configured' || source === 'local') && isLocal && !isBrowserLocalContext()) return null
    if (source === 'storage' && !isBrowserLocalContext()) return null
    return { base: normalized, source }
  } catch {
    return null
  }
}

function mergeCandidates(target: RouteControlCandidate[], next: RouteControlCandidate | null): void {
  if (!next) return
  if (target.some((candidate) => candidate.base === next.base)) return
  target.push(next)
}

function configuredApiBase(): string {
  if (typeof window === 'undefined') return ''
  const explicit = normalizeBase(String((window as any).__EDGERUN_API_BASE || ''))
  if (!explicit) return ''
  return normalizeControlBase(explicit)
}

function localControlBaseCandidates(): string[] {
  if (typeof window === 'undefined' || !isBrowserLocalContext()) return []
  const { protocol, hostname, port } = window.location
  const scheme = protocol === 'https:' ? 'https:' : 'http:'
  const out: string[] = []
  if (port !== '8090') out.push(`${scheme}//${hostname}:8090`)
  if (hostname !== '127.0.0.1') out.push(`${scheme}//127.0.0.1:8090`)
  return [...new Set(out.map(normalizeBase))]
}

export function getRouteControlBaseSelection(): RouteControlSelection {
  if (typeof window === 'undefined') {
    return {
      selected: DEFAULT_ROUTE_CONTROL_BASE,
      source: 'default',
      selectedFromStorage: false,
      candidates: [{ base: DEFAULT_ROUTE_CONTROL_BASE, source: 'default' }]
    }
  }

  const candidates: RouteControlCandidate[] = []
  const configured = configuredApiBase()
  mergeCandidates(candidates, toRouteControlCandidate(configured, 'configured'))
  for (const candidate of localControlBaseCandidates().map((value) => toRouteControlCandidate(value, 'local'))) {
    mergeCandidates(candidates, candidate)
  }
  mergeCandidates(candidates, toRouteControlCandidate(window.localStorage.getItem(CONTROL_BASE_STORAGE_KEY) || '', 'storage'))
  if (isBrowserLocalContext()) {
    mergeCandidates(candidates, toRouteControlCandidate(window.location.origin, 'origin'))
  }

  if (candidates.length === 0) {
    candidates.push({ base: DEFAULT_ROUTE_CONTROL_BASE, source: 'default' })
  }

  const selected = candidates[0]!
  return {
    candidates,
    selected: selected.base,
    source: selected.source,
    selectedFromStorage: selected.source === 'storage'
  }
}

function rememberControlBase(controlBase: string): void {
  if (typeof window === 'undefined') return
  const normalized = normalizeControlBase(controlBase)
  if (!normalized) return
  window.localStorage.setItem(CONTROL_BASE_STORAGE_KEY, normalized)
}

function encodeB64Url(input: string): string {
  if (!input) return ''
  const bytes = new TextEncoder().encode(input)
  let binary = ''
  for (let i = 0; i < bytes.length; i += 1) binary += String.fromCharCode(bytes[i]!)
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '')
}

function decodeB64Url(input: string): string {
  if (!input) return ''
  const base64 = input.replace(/-/g, '+').replace(/_/g, '/')
  const padded = base64 + '='.repeat((4 - (base64.length % 4 || 4)) % 4)
  const binary = atob(padded)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i)
  return new TextDecoder().decode(bytes)
}

function encodeSignalOutbound(message: SignalOutboundMessage): string {
  const mline = typeof message.sdp_mline_index === 'number' ? String(message.sdp_mline_index) : ''
  return [
    encodeB64Url(message.to_device_id || ''),
    encodeB64Url(message.to_owner_pubkey || ''),
    encodeB64Url(message.kind || ''),
    encodeB64Url(message.sdp || ''),
    encodeB64Url(message.candidate || ''),
    encodeB64Url(message.sdp_mid || ''),
    encodeB64Url(mline),
    encodeB64Url(message.metadata || '')
  ].join('|')
}

function decodeSignalInbound(frame: string): SignalInboundMessage | null {
  const parts = frame.split('|')
  if (parts.length !== 8) return null
  try {
    const from_device_id = decodeB64Url(parts[0] || '')
    const kind = decodeB64Url(parts[1] || '')
    const sdp = decodeB64Url(parts[2] || '')
    const candidate = decodeB64Url(parts[3] || '')
    const sdp_mid = decodeB64Url(parts[4] || '')
    const mlineRaw = decodeB64Url(parts[5] || '')
    const metadata = decodeB64Url(parts[6] || '')
    const error = decodeB64Url(parts[7] || '')
    const parsedMline = mlineRaw ? Number.parseInt(mlineRaw, 10) : NaN
    return {
      from_device_id,
      kind,
      sdp: sdp || undefined,
      candidate: candidate || undefined,
      sdp_mid: sdp_mid || undefined,
      sdp_mline_index: Number.isFinite(parsedMline) ? parsedMline : undefined,
      metadata: metadata || undefined,
      error: error || undefined
    }
  } catch {
    return null
  }
}

let cachedControlStatus: RouteControlCachedStatus | null = null

function controlWsProbeUrl(controlBase: string): string {
  const normalized = normalizeControlBase(controlBase)
  if (!normalized) return ''
  const url = new URL('/v1/control/ws', `${normalized}/`)
  url.searchParams.set('client_id', 'route-control-probe')
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
  return url.toString()
}

export function getRouteControlBase(): string {
  const selection = getRouteControlBaseSelection()
  return selection.selected || DEFAULT_ROUTE_CONTROL_BASE
}

export function getCachedRouteControlStatus(): RouteControlProbeStatus | null {
  if (!cachedControlStatus) return null
  if (Date.now() - cachedControlStatus.selectedAt > CONTROL_STATUS_CACHE_TTL_MS) return null
  return {
    base: cachedControlStatus.base,
    source: cachedControlStatus.source,
    checkedAt: cachedControlStatus.selectedAt,
    httpReachable: cachedControlStatus.httpReachable,
    httpStatus: cachedControlStatus.httpStatus,
    httpLatencyMs: cachedControlStatus.httpLatencyMs,
    controlWsReachable: cachedControlStatus.controlWsReachable,
    controlWsLatencyMs: cachedControlStatus.controlWsLatencyMs,
    error: ''
  }
}

export type RouteControlProbeOptions = {
  skipWsProbe?: boolean
}

export async function probeRouteControlStatus(
  controlBaseOverride?: string,
  sourceOverride?: RouteControlSource,
  options?: RouteControlProbeOptions,
): Promise<RouteControlProbeStatus> {
  const selection = getRouteControlBaseSelection()
  const overrideRequested = typeof controlBaseOverride === 'string' && controlBaseOverride.trim().length > 0
  const overridden = normalizeControlBase(controlBaseOverride || '')
  const base = overridden || selection.selected || DEFAULT_ROUTE_CONTROL_BASE
  const source = sourceOverride ?? (overridden ? 'configured' : selection.source)
  const out: RouteControlProbeStatus = {
    base,
    source,
    checkedAt: Date.now(),
    httpReachable: false,
    httpStatus: null,
    httpLatencyMs: null,
    controlWsReachable: false,
    controlWsLatencyMs: null,
    error: ''
  }
  if (overrideRequested && !overridden) {
    out.error = 'invalid control base override'
  }

  const wsUrl = controlWsProbeUrl(base)
  if (!options?.skipWsProbe && wsUrl) {
    out.controlWsReachable = await new Promise<boolean>((resolve) => {
      let settled = false
      let ws: WebSocket | null = null
      const started = Date.now()
      const timer = window.setTimeout(() => {
        if (settled) return
        settled = true
        if (ws && ws.readyState === WebSocket.OPEN) {
          try { ws.close() } catch { /* ignore */ }
        }
        out.controlWsLatencyMs = Date.now() - started
        resolve(false)
      }, CONTROL_PROBE_TIMEOUT_MS)
      try {
        ws = new WebSocket(wsUrl)
      } catch {
        settled = true
        window.clearTimeout(timer)
        out.controlWsLatencyMs = Date.now() - started
        resolve(false)
        return
      }

      const finish = (ok: boolean) => {
        if (settled) return
        settled = true
        window.clearTimeout(timer)
        if (ws && ws.readyState === WebSocket.OPEN) {
          try { ws.close() } catch { /* ignore */ }
        }
        out.controlWsLatencyMs = Date.now() - started
        resolve(ok)
      }
      ws.addEventListener('open', () => finish(true), { once: true })
      ws.addEventListener('error', () => finish(false), { once: true })
      ws.addEventListener('close', () => finish(false), { once: true })
    })
  }

  if (!out.error && !options?.skipWsProbe && !out.controlWsReachable) {
    out.error = 'control ws probe failed'
  }
  out.checkedAt = Date.now()
  cachedControlStatus = {
    base: out.base,
    source: out.source,
    selectedAt: out.checkedAt,
    httpReachable: out.httpReachable,
    httpStatus: out.httpStatus,
    httpLatencyMs: out.httpLatencyMs,
    controlWsReachable: out.controlWsReachable,
    controlWsLatencyMs: out.controlWsLatencyMs
  }
  return out
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

export async function resolveDeviceRoute(controlBase: string, deviceId: string): Promise<string | null> {
  const trimmedBase = normalizeControlBase(controlBase)
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

function routeControlBasesForResolution(preferredControlBase?: string): string[] {
  const selection = getRouteControlBaseSelection()
  const out: string[] = []
  const preferred = normalizeControlBase(preferredControlBase || '')
  if (preferred) out.push(preferred)
  for (const candidate of selection.candidates) {
    const normalized = normalizeControlBase(candidate.base || '')
    if (normalized) out.push(normalized)
  }
  out.push(DEFAULT_ROUTE_CONTROL_BASE)
  return [...new Set(out.map((base) => normalizeControlBase(base)).filter(Boolean))]
}

export async function resolveTerminalBaseUrl(input: string, controlBase?: string): Promise<string> {
  const target = input.trim()
  if (!target) return ''
  const routeDeviceId = parseRouteDeviceId(target)
  if (!routeDeviceId) return target
  const bases = routeControlBasesForResolution(controlBase || getRouteControlBase())
  for (const base of bases) {
    const resolved = await resolveDeviceRoute(base, routeDeviceId)
    if (resolved) {
      rememberControlBase(base)
      return resolved
    }
  }
  return ''
}

export async function resolveOwnerRoutes(controlBase: string, ownerPubkey: string): Promise<RouteEntry[]> {
  const trimmedOwner = ownerPubkey.trim()
  if (!trimmedOwner) return []
  const bases = routeControlBasesForResolution(controlBase || getRouteControlBase())
  for (const base of bases) {
    try {
      const body = await getControlClient(base).request<OwnerRoutesResponse>(
        'route.owner',
        { owner_pubkey: trimmedOwner }
      )
      if (!body.ok) continue
      rememberControlBase(base)
      return Array.isArray(body.devices) ? body.devices : []
    } catch {
      // Explicit user action failed on this WS control channel; try next candidate.
    }
  }
  return []
}

function getControlClient(controlBase: string): SchedulerControlWsClient {
  const normalized = normalizeControlBase(controlBase)
  if (!normalized) throw new Error('invalid route control base')
  const existing = controlClients.get(normalized)
  if (existing) return existing
  const created = new SchedulerControlWsClient(normalized, 'route-client')
  controlClients.set(normalized, created)
  return created
}

export class WebRtcSignalClient {
  private readonly wsUrl: string
  private ws: WebSocket | null = null
  private closed = false
  private reconnectTimer: number | null = null
  private reconnectAttempts = 0

  onMessage?: (message: SignalInboundMessage) => void
  onClose?: () => void
  onOpen?: () => void

  constructor(controlBase: string, deviceId: string) {
    const base = normalizeControlBase(controlBase)
    if (!base) throw new Error('invalid route control base')
    const url = new URL('/v1/webrtc/ws', `${base}/`)
    url.searchParams.set('device_id', deviceId)
    const wsProto = url.protocol === 'https:' ? 'wss:' : 'ws:'
    url.protocol = wsProto
    this.wsUrl = url.toString()
    this.connect()
  }

  private connect(): void {
    if (this.closed) return
    if (this.ws && (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING)) return
    const ws = new WebSocket(this.wsUrl)
    this.ws = ws
    ws.addEventListener('open', () => {
      this.reconnectAttempts = 0
      this.onOpen?.()
    })
    ws.addEventListener('close', () => {
      if (this.ws === ws) this.ws = null
      this.onClose?.()
      this.scheduleReconnect()
    })
    ws.addEventListener('error', () => {
      // no-op: close handler drives reconnect behavior
    })
    ws.addEventListener('message', (event) => {
      if (typeof event.data !== 'string') return
      const payload = decodeSignalInbound(event.data)
      if (!payload) return
      this.onMessage?.(payload)
    })
  }

  private scheduleReconnect(): void {
    if (this.closed) return
    if (this.reconnectTimer !== null) return
    const delay = Math.min(10_000, 600 * (1 << Math.min(this.reconnectAttempts, 4)))
    this.reconnectAttempts += 1
    this.reconnectTimer = window.setTimeout(() => {
      this.reconnectTimer = null
      this.connect()
    }, delay)
  }

  send(message: SignalOutboundMessage): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return
    try {
      this.ws.send(encodeSignalOutbound(message))
    } catch {
      // ignore transient closed-state sends
    }
  }

  close(): void {
    this.closed = true
    if (this.reconnectTimer !== null) {
      window.clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
    if (this.ws) {
      try {
        this.ws.close()
      } catch {
        // ignore close errors
      }
      this.ws = null
    }
  }
}
