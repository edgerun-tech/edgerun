// SPDX-License-Identifier: Apache-2.0
import type { DeviceStatus, TerminalDevice } from './terminal-drawer-store'
import { getRouteControlBase, parseRouteDeviceId } from './webrtc-route-client'
import { getWebRtcPeerSupervisor } from './webrtc-peer-supervisor'

type RouteResolveResponse = {
  ok?: boolean
  found?: boolean
  route?: {
    online?: boolean
  } | null
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
  return probeRouteDeviceOnlineViaWebRtc(routeDeviceId, Math.min(timeoutMs, 1400)).catch(() => false)
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
