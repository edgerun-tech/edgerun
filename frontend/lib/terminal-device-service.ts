// SPDX-License-Identifier: Apache-2.0
import type { DeviceStatus, TerminalDevice } from './terminal-drawer-store'
import { getRouteControlBase, parseRouteDeviceId, resolveDeviceRoute, routeRelayWsUrl } from './webrtc-route-client'
import { getWebRtcPeerSupervisor } from './webrtc-peer-supervisor'

type RouteResolveResponse = {
  ok?: boolean
  found?: boolean
  route?: {
    online?: boolean
  } | null
}

function toDirectWsUrl(baseUrl: string): string | null {
  const trimmed = baseUrl.trim()
  if (!trimmed) return null
  try {
    const url = new URL('/ws', `${trimmed.replace(/\/+$/, '')}/`)
    url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
    return url.toString()
  } catch {
    return null
  }
}

async function probeRouteDeviceOnlineViaWebRtc(routeDeviceId: string, timeoutMs = 1400): Promise<boolean> {
  const target = routeDeviceId.trim()
  if (!target) return false
  const supervisor = getWebRtcPeerSupervisor()
  try {
    await supervisor.connectToDevice(target)
  } catch {
    // keep probing via known sessions/routes
  }
  return supervisor.waitForRoutedPong(target, timeoutMs)
}

async function probeRouteDeviceOnlineViaDirectWs(routeDeviceId: string, timeoutMs = 1800): Promise<boolean> {
  const target = routeDeviceId.trim()
  if (!target) return false
  const controlBase = getRouteControlBase()
  const resolvedBase = await resolveDeviceRoute(controlBase, target).catch(() => null)
  const relayWs = routeRelayWsUrl(controlBase, target)
  const candidates = [
    resolvedBase ? toDirectWsUrl(resolvedBase) : null,
    relayWs,
  ].filter((url): url is string => Boolean(url?.trim()))

  for (const wsUrl of candidates) {
    const ok = await probeWsEndpoint(wsUrl, timeoutMs)
    if (ok) return true
  }
  return false
}

function probeWsEndpoint(wsUrl: string, timeoutMs: number): Promise<boolean> {
  return new Promise<boolean>((resolve) => {
    let settled = false
    const ws = new WebSocket(wsUrl)
    const timeoutId = window.setTimeout(() => {
      if (settled) return
      settled = true
      try { ws.close() } catch { /* ignore */ }
      resolve(false)
    }, timeoutMs)
    const finish = (ok: boolean) => {
      if (settled) return
      settled = true
      window.clearTimeout(timeoutId)
      try { ws.close() } catch { /* ignore */ }
      resolve(ok)
    }
    ws.addEventListener('open', () => finish(true))
    ws.addEventListener('error', () => finish(false))
    ws.addEventListener('close', () => finish(false))
  })
}

async function probeRoutePresence(routeDeviceId: string): Promise<boolean> {
  const controlBase = getRouteControlBase().trim().replace(/\/+$/, '')
  if (!controlBase) return false
  const target = routeDeviceId.trim()
  if (!target) return false
  try {
    const resp = await fetch(`${controlBase}/v1/route/resolve/${encodeURIComponent(target)}`)
    if (!resp.ok) return false
    const body = await resp.json() as RouteResolveResponse
    if (!body.ok || !body.found) return false
    return body.route?.online !== false
  } catch {
    return false
  }
}

export async function probeRouteDeviceOnline(routeDeviceId: string, timeoutMs = 2000): Promise<boolean> {
  const routePresent = await probeRoutePresence(routeDeviceId).catch(() => false)
  if (routePresent) return true
  const webRtcOk = await probeRouteDeviceOnlineViaWebRtc(routeDeviceId, Math.min(timeoutMs, 1400)).catch(() => false)
  if (webRtcOk) return true
  return probeRouteDeviceOnlineViaDirectWs(routeDeviceId, Math.min(Math.max(timeoutMs - 200, 1000), 2200))
}

export async function probeDeviceOnline(baseUrl: string, timeoutMs = 2500): Promise<boolean> {
  try {
    const routeDeviceId = parseRouteDeviceId(baseUrl)
    if (!routeDeviceId) return false
    return probeRouteDeviceOnline(routeDeviceId, Math.min(timeoutMs, 2400))
  } catch {
    return false
  }
}

export async function refreshTerminalDevices(
  devices: readonly Pick<TerminalDevice, 'id' | 'baseUrl'>[],
  mark: (id: string, status: DeviceStatus) => void
): Promise<void> {
  await Promise.all(devices.map(async (device) => {
      const online = await probeDeviceOnline(device.baseUrl)
      mark(device.id, online ? 'online' : 'offline')
  }))
}
