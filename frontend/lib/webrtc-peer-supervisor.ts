import { getRouteControlBase, WebRtcSignalClient, parseRouteDeviceId } from './webrtc-route-client'

type IntentState = {
  deviceIds: string[]
}

type PeerSession = {
  deviceId: string
  pc: RTCPeerConnection
  dc: RTCDataChannel | null
  reconnectTimer: number | null
  reconnectAttempts: number
  closed: boolean
}

const SUPERVISOR_STATE_KEY = 'edgerun.webrtc.peerSupervisor.v1'
const SUPERVISOR_DEVICE_ID_KEY = 'edgerun.webrtc.localDeviceId.v1'
const RTC_CONFIG: RTCConfiguration = {
  iceServers: [
    { urls: 'stun:stun.l.google.com:19302' },
    { urls: 'stun:stun.cloudflare.com:3478' }
  ]
}

function parseIntentState(raw: string | null): IntentState {
  if (!raw) return { deviceIds: [] }
  try {
    const parsed = JSON.parse(raw) as IntentState
    if (!Array.isArray(parsed.deviceIds)) return { deviceIds: [] }
    const deviceIds = parsed.deviceIds
      .map((id) => String(id || '').trim())
      .filter((id) => id.length > 0)
    return { deviceIds: [...new Set(deviceIds)] }
  } catch {
    return { deviceIds: [] }
  }
}

function serializeIntentState(state: IntentState): string {
  return JSON.stringify({ deviceIds: [...new Set(state.deviceIds)] })
}

function persistentLocalDeviceId(): string {
  if (typeof window === 'undefined') return 'web-local'
  const existing = window.localStorage.getItem(SUPERVISOR_DEVICE_ID_KEY)?.trim() || ''
  if (existing) return existing
  const generated = window.crypto?.randomUUID ? `web-${window.crypto.randomUUID()}` : `web-${Date.now()}`
  window.localStorage.setItem(SUPERVISOR_DEVICE_ID_KEY, generated)
  return generated
}

export class WebRtcPeerSupervisor {
  private readonly controlBase: string
  private readonly localDeviceId: string
  private readonly signal: WebRtcSignalClient
  private readonly sessions = new Map<string, PeerSession>()
  private intents: IntentState
  private started = false

  constructor(controlBase?: string, localDeviceId?: string) {
    this.controlBase = (controlBase || getRouteControlBase()).replace(/\/+$/, '')
    this.localDeviceId = localDeviceId?.trim() || persistentLocalDeviceId()
    this.intents = parseIntentState(typeof window !== 'undefined' ? window.localStorage.getItem(SUPERVISOR_STATE_KEY) : null)
    this.signal = new WebRtcSignalClient(this.controlBase, this.localDeviceId)
    this.signal.onMessage = (message) => {
      void this.handleSignalMessage(message)
    }
    this.signal.onClose = () => {
      this.scheduleReconnectAll(1200)
    }
  }

  start(): void {
    if (this.started) return
    this.started = true
    for (const deviceId of this.intents.deviceIds) {
      void this.connectToDevice(deviceId)
    }
  }

  stop(): void {
    this.started = false
    this.signal.close()
    for (const session of this.sessions.values()) {
      this.teardownSession(session, false)
    }
    this.sessions.clear()
  }

  async connectToRouteTarget(target: string): Promise<void> {
    const deviceId = parseRouteDeviceId(target)
    if (!deviceId) return
    await this.connectToDevice(deviceId)
  }

  async connectToDevice(deviceId: string): Promise<void> {
    const target = deviceId.trim()
    if (!target) return
    const existing = this.sessions.get(target)
    if (existing && !existing.closed) return
    const session = this.createSession(target)
    this.sessions.set(target, session)
    this.addIntent(target)
    const dc = session.pc.createDataChannel('edgerun-control', { ordered: true })
    this.bindDataChannel(session, dc)
    await this.negotiateOffer(session)
  }

  sendText(deviceId: string, text: string): boolean {
    const session = this.sessions.get(deviceId)
    if (!session?.dc || session.dc.readyState !== 'open') return false
    try {
      session.dc.send(text)
      return true
    } catch {
      return false
    }
  }

  private createSession(deviceId: string): PeerSession {
    const pc = new RTCPeerConnection(RTC_CONFIG)
    const session: PeerSession = {
      deviceId,
      pc,
      dc: null,
      reconnectTimer: null,
      reconnectAttempts: 0,
      closed: false
    }
    pc.onicecandidate = (event) => {
      if (!event.candidate) return
      this.signal.send({
        to_device_id: deviceId,
        kind: 'ice',
        candidate: event.candidate.candidate,
        sdp_mid: event.candidate.sdpMid || undefined,
        sdp_mline_index: event.candidate.sdpMLineIndex ?? undefined
      })
    }
    pc.ondatachannel = (event) => {
      this.bindDataChannel(session, event.channel)
    }
    pc.onconnectionstatechange = () => {
      const state = pc.connectionState
      if (state === 'connected') {
        session.reconnectAttempts = 0
        return
      }
      if (state === 'failed' || state === 'disconnected' || state === 'closed') {
        this.scheduleReconnect(deviceId)
      }
    }
    return session
  }

  private bindDataChannel(session: PeerSession, dc: RTCDataChannel): void {
    session.dc = dc
    dc.onopen = () => {
      session.reconnectAttempts = 0
      try {
        dc.send(JSON.stringify({ kind: 'hello', from: this.localDeviceId, at: Date.now() }))
      } catch {
        // ignore transient send errors on open
      }
    }
    dc.onclose = () => this.scheduleReconnect(session.deviceId)
    dc.onerror = () => this.scheduleReconnect(session.deviceId)
  }

  private async negotiateOffer(session: PeerSession): Promise<void> {
    const offer = await session.pc.createOffer()
    await session.pc.setLocalDescription(offer)
    this.signal.send({
      to_device_id: session.deviceId,
      kind: 'offer',
      sdp: offer.sdp || ''
    })
  }

  private async handleSignalMessage(message: {
    from_device_id: string
    kind: string
    sdp?: string
    candidate?: string
    sdp_mid?: string
    sdp_mline_index?: number
  }): Promise<void> {
    const from = message.from_device_id?.trim() || ''
    if (!from) return

    let session = this.sessions.get(from)
    if (!session) {
      session = this.createSession(from)
      this.sessions.set(from, session)
      this.addIntent(from)
    }

    if (message.kind === 'offer' && message.sdp) {
      await session.pc.setRemoteDescription({ type: 'offer', sdp: message.sdp })
      const answer = await session.pc.createAnswer()
      await session.pc.setLocalDescription(answer)
      this.signal.send({
        to_device_id: from,
        kind: 'answer',
        sdp: answer.sdp || ''
      })
      return
    }
    if (message.kind === 'answer' && message.sdp) {
      await session.pc.setRemoteDescription({ type: 'answer', sdp: message.sdp })
      return
    }
    if (message.kind === 'ice' && message.candidate) {
      await session.pc.addIceCandidate({
        candidate: message.candidate,
        sdpMid: message.sdp_mid || null,
        sdpMLineIndex: message.sdp_mline_index ?? null
      })
    }
  }

  private scheduleReconnect(deviceId: string): void {
    const session = this.sessions.get(deviceId)
    if (!session || session.closed) return
    if (session.reconnectTimer !== null) return
    const delay = Math.min(15000, 800 * (1 << Math.min(session.reconnectAttempts, 5)))
    session.reconnectAttempts = session.reconnectAttempts + 1
    session.reconnectTimer = window.setTimeout(() => {
      session.reconnectTimer = null
      this.reconnect(deviceId)
    }, delay)
  }

  private scheduleReconnectAll(delayMs: number): void {
    for (const deviceId of this.intents.deviceIds) {
      const session = this.sessions.get(deviceId)
      if (session && !session.closed) continue
      window.setTimeout(() => {
        void this.connectToDevice(deviceId)
      }, delayMs)
    }
  }

  private reconnect(deviceId: string): void {
    const prev = this.sessions.get(deviceId)
    if (prev) this.teardownSession(prev, false)
    this.sessions.delete(deviceId)
    void this.connectToDevice(deviceId)
  }

  private teardownSession(session: PeerSession, removeIntent: boolean): void {
    session.closed = true
    if (session.reconnectTimer !== null) {
      window.clearTimeout(session.reconnectTimer)
      session.reconnectTimer = null
    }
    try { session.dc?.close() } catch { /* ignore */ }
    try { session.pc.close() } catch { /* ignore */ }
    if (removeIntent) this.removeIntent(session.deviceId)
  }

  private addIntent(deviceId: string): void {
    if (this.intents.deviceIds.includes(deviceId)) return
    this.intents = {
      deviceIds: [...this.intents.deviceIds, deviceId]
    }
    this.persistIntents()
  }

  private removeIntent(deviceId: string): void {
    this.intents = {
      deviceIds: this.intents.deviceIds.filter((id) => id !== deviceId)
    }
    this.persistIntents()
  }

  private persistIntents(): void {
    if (typeof window === 'undefined') return
    window.localStorage.setItem(SUPERVISOR_STATE_KEY, serializeIntentState(this.intents))
  }
}

let singleton: WebRtcPeerSupervisor | null = null

export function getWebRtcPeerSupervisor(): WebRtcPeerSupervisor {
  if (!singleton) singleton = new WebRtcPeerSupervisor()
  return singleton
}

export function initWebRtcPeerSupervisor(): void {
  if (typeof window === 'undefined') return
  const supervisor = getWebRtcPeerSupervisor()
  supervisor.start()
}
