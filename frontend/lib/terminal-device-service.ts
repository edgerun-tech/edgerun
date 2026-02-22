// SPDX-License-Identifier: Apache-2.0
import type { DeviceStatus, TerminalDevice } from './terminal-drawer-store'
import { parseRouteDeviceId, resolveTerminalBaseUrl } from './webrtc-route-client'
import { getWebRtcPeerSupervisor } from './webrtc-peer-supervisor'

async function probeRouteDeviceOnline(routeDeviceId: string, timeoutMs = 1400): Promise<boolean> {
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

async function probeTerminalWsOnline(baseUrl: string, timeoutMs: number): Promise<boolean> {
  const wsUrl = new URL('/ws', baseUrl)
  wsUrl.protocol = wsUrl.protocol === 'https:' ? 'wss:' : 'ws:'
  return new Promise<boolean>((resolve) => {
    const ws = new WebSocket(wsUrl.toString())
    let settled = false
    const done = (ok: boolean): void => {
      if (settled) return
      settled = true
      window.clearTimeout(timeoutId)
      try {
        ws.close()
      } catch {
        // ignore close errors
      }
      resolve(ok)
    }
    const timeoutId = window.setTimeout(() => done(false), timeoutMs)
    ws.addEventListener('open', () => done(true), { once: true })
    ws.addEventListener('error', () => done(false), { once: true })
    ws.addEventListener('close', () => {
      if (!settled) done(false)
    }, { once: true })
  })
}

export async function probeDeviceOnline(baseUrl: string, timeoutMs = 2500): Promise<boolean> {
  try {
    const routeDeviceId = parseRouteDeviceId(baseUrl)
    if (routeDeviceId) {
      const routedOnline = await probeRouteDeviceOnline(routeDeviceId, Math.min(timeoutMs, 1600))
      if (routedOnline) return true
    }
    const resolved = await resolveTerminalBaseUrl(baseUrl)
    if (!resolved) return false
    return probeTerminalWsOnline(resolved, timeoutMs)
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
