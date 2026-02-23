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

export async function probeRouteControlStatus(
  controlBaseOverride?: string,
  sourceOverride?: RouteControlSource,
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

  const httpStarted = Date.now()
  const httpController = new AbortController()
  const httpTimer = window.setTimeout(() => httpController.abort(), CONTROL_PROBE_TIMEOUT_MS)
  try {
    const response = await fetch(`${base}/health`, {
      signal: httpController.signal,
      cache: 'no-store'
    })
    out.httpReachable = response.ok
    out.httpStatus = Number.isFinite(response.status) ? response.status : null
  } catch (error) {
    out.error = error instanceof Error ? error.message : 'http probe failed'
  } finally {
    window.clearTimeout(httpTimer)
    out.httpLatencyMs = Date.now() - httpStarted
  }

  const wsUrl = controlWsProbeUrl(base)
  if (wsUrl) {
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

  if (!out.error && !out.controlWsReachable) {
    out.error = 'control ws probe failed'
  }
  if (!out.error && !out.httpReachable) {
    out.error = 'scheduler probe failed'
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
    try {
      const resp = await fetch(`${trimmedBase}/v1/route/resolve/${encodeURIComponent(trimmedDeviceId)}`)
      if (!resp.ok) return null
      const body = await resp.json() as RouteResolveResponse
      if (!body.ok || !body.found) return null
      const reachable = Array.isArray(body.route?.reachable_urls) ? body.route?.reachable_urls : []
      const first = reachable.find((item) => typeof item === 'string' && item.trim().length > 0)
      return first?.trim() || null
    } catch {
      return null
    }
  }
}

export async function resolveTerminalBaseUrl(input: string, controlBase?: string): Promise<string> {
  const target = input.trim()
  if (!target) return ''
  const routeDeviceId = parseRouteDeviceId(target)
  if (!routeDeviceId) return target
  const base = normalizeControlBase(controlBase || getRouteControlBase())
  if (!base) return ''
  const resolved = await resolveDeviceRoute(base, routeDeviceId)
  if (resolved) {
    rememberControlBase(base)
    return resolved
  }
  return ''
}

export async function resolveOwnerRoutes(controlBase: string, ownerPubkey: string): Promise<RouteEntry[]> {
  const base = normalizeControlBase(controlBase || getRouteControlBase())
  const trimmedOwner = ownerPubkey.trim()
  if (!trimmedOwner || !base) return []
  try {
    const body = await getControlClient(base).request<OwnerRoutesResponse>(
      'route.owner',
      { owner_pubkey: trimmedOwner }
    )
    if (!body.ok) return []
    rememberControlBase(base)
    return Array.isArray(body.devices) ? body.devices : []
  } catch {
    try {
      const resp = await fetch(`${base}/v1/route/owner/${encodeURIComponent(trimmedOwner)}`)
      if (!resp.ok) return []
      const body = await resp.json() as OwnerRoutesResponse
      if (!body.ok) return []
      rememberControlBase(base)
      return Array.isArray(body.devices) ? body.devices : []
    } catch {
      // Explicit user action failed across ws + fallback HTTP route lookup.
    }
  }
  return []
}

export function routeRelayWsUrl(controlBase: string, deviceId: string): string | null {
  const base = normalizeControlBase(controlBase)
  const target = deviceId.trim()
  if (!base || !target) return null
  try {
    const url = new URL('/v1/route/ws', `${base}/`)
    url.searchParams.set('device_id', target)
    url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
    return url.toString()
  } catch {
    return null
  }
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
  private readonly ws: WebSocket

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
    this.ws = new WebSocket(url.toString())
    this.ws.addEventListener('open', () => this.onOpen?.())
    this.ws.addEventListener('close', () => this.onClose?.())
    this.ws.addEventListener('error', () => {
      // no-op: existing handlers manage retry and fallback behavior
    })
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
