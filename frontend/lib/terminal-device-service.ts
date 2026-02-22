import type { DeviceStatus, TerminalDevice } from './terminal-drawer-store'
import { resolveTerminalBaseUrl } from './webrtc-route-client'

type BridgeDevice = {
  name?: string
  base_url?: string
  baseUrl?: string
}

type BridgeResponse = {
  ok?: boolean
  devices?: BridgeDevice[]
}

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

export async function canUseCurrentOriginAsDevice(origin: string): Promise<boolean> {
  return probeDeviceOnline(origin, 1400)
}

export async function importTailscaleBridgeDevices(
  endpoints: readonly string[],
  existingUrls: Set<string>
): Promise<{ added: Array<{ name: string; baseUrl: string }>; error?: string }> {
  let payload: BridgeResponse | null = null
  for (const endpoint of endpoints) {
    try {
      const response = await fetch(endpoint, { method: 'GET' })
      if (!response.ok) continue
      payload = await response.json() as BridgeResponse
      if (payload?.ok) break
    } catch {
      // try next endpoint
    }
  }

  if (!payload?.ok || !Array.isArray(payload.devices)) {
    return {
      added: [],
      error: 'Tailscale bridge unavailable. Start: edgerun tailscale bridge'
    }
  }

  const added: Array<{ name: string; baseUrl: string }> = []
  for (const item of payload.devices) {
    const baseUrl = (item.base_url || item.baseUrl || '').trim()
    if (!baseUrl || existingUrls.has(baseUrl)) continue
    const name = (item.name || 'Tailscale Device').trim()
    added.push({ name, baseUrl })
    existingUrls.add(baseUrl)
  }
  return { added }
}
