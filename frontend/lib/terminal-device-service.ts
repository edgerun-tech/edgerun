// SPDX-License-Identifier: Apache-2.0
import type { DeviceStatus, TerminalDevice } from './terminal-drawer-store'
import { parseRouteDeviceId } from './webrtc-route-client'
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

export async function probeDeviceOnline(baseUrl: string, timeoutMs = 2500): Promise<boolean> {
  try {
    const routeDeviceId = parseRouteDeviceId(baseUrl)
    if (!routeDeviceId) return false
    return probeRouteDeviceOnline(routeDeviceId, Math.min(timeoutMs, 1600))
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
