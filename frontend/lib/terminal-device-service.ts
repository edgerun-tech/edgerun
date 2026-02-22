import type { DeviceStatus, TerminalDevice } from './terminal-drawer-store'
import { resolveTerminalBaseUrl } from './webrtc-route-client'

export async function probeDeviceOnline(baseUrl: string, timeoutMs = 2500): Promise<boolean> {
  const controller = new AbortController()
  const timeoutId = window.setTimeout(() => controller.abort(), timeoutMs)
  try {
    const resolved = await resolveTerminalBaseUrl(baseUrl)
    if (!resolved) return false
    const url = new URL('/v1/device/identity', resolved)
    const response = await fetch(url.toString(), { method: 'GET', signal: controller.signal })
    return response.ok
  } catch {
    return false
  } finally {
    window.clearTimeout(timeoutId)
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
