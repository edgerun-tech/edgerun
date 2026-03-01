// SPDX-License-Identifier: Apache-2.0
import type { DeviceStatus, TerminalDevice } from './terminal-drawer-store'
import { parseRouteDeviceId } from './webrtc-route-client'
import { getWebRtcPeerSupervisor } from './webrtc-peer-supervisor'

function withHttpScheme(value: string): string {
  const raw = String(value || '').trim()
  if (!raw) return ''
  return /^[a-zA-Z][a-zA-Z\d+\-.]*:\/\//.test(raw) ? raw : `http://${raw}`
}

function identityUrlForBase(baseUrl: string): string {
  const withScheme = withHttpScheme(baseUrl)
  if (!withScheme) return ''
  let url: URL
  try {
    url = new URL(withScheme)
  } catch {
    return ''
  }
  if (url.protocol !== 'http:' && url.protocol !== 'https:') return ''
  const path = url.pathname.replace(/\/+$/, '')
  if (!path || path === '/') {
    url.pathname = '/v1/device/identity'
  } else if (/\/term$/i.test(path)) {
    url.pathname = path.replace(/\/term$/i, '/v1/device/identity')
  } else {
    url.pathname = `${path}/v1/device/identity`
  }
  return url.toString()
}

async function fetchWithTimeout(input: string, timeoutMs: number): Promise<Response> {
  const controller = typeof AbortController !== 'undefined' ? new AbortController() : null
  const timer = controller ? window.setTimeout(() => controller.abort(), timeoutMs) : null
  try {
    return await fetch(input, {
      method: 'GET',
      cache: 'no-store',
      signal: controller?.signal
    })
  } finally {
    if (timer !== null) window.clearTimeout(timer)
  }
}

async function probeHttpDeviceOnline(baseUrl: string, timeoutMs = 1800): Promise<boolean> {
  const identityUrl = identityUrlForBase(baseUrl)
  if (!identityUrl) return false
  try {
    const response = await fetchWithTimeout(identityUrl, timeoutMs)
    if (!response.ok) return false
    const payload = await response.json().catch(() => ({}))
    const id = String(payload?.device_pubkey_b64url || '').trim()
    return id.length > 0
  } catch {
    return false
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

export async function probeRouteDeviceOnline(routeDeviceId: string, timeoutMs = 2000): Promise<boolean> {
  return probeRouteDeviceOnlineViaWebRtc(routeDeviceId, Math.min(timeoutMs, 1400)).catch(() => false)
}

export async function probeDeviceOnline(baseUrl: string, timeoutMs = 2500): Promise<boolean> {
  try {
    const routeDeviceId = parseRouteDeviceId(baseUrl)
    if (routeDeviceId) {
      return probeRouteDeviceOnline(routeDeviceId, Math.min(timeoutMs, 2400))
    }
    return probeHttpDeviceOnline(baseUrl, Math.min(timeoutMs, 2200))
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
