// SPDX-License-Identifier: Apache-2.0
import { getRouteControlBase, normalizeControlBase, WebRtcSignalClient, parseRouteDeviceId } from './webrtc-route-client'

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

type RoutedPacket = {
  kind: 'relay'
  packet_id: string
  src: string
  dst: string
  ttl: number
  payload: string
  via?: string[]
}

type HelloPacket = {
  kind: 'hello'
  from: string
  at: number
  neighbors?: string[]
}

type RouteAdvertPacket = {
  kind: 'route-advert'
  from: string
  at: number
  routes: Array<{ target: string; hops: number }>
}

type SupervisorWirePacket = RoutedPacket | HelloPacket | RouteAdvertPacket

type RouteRecord = {
  nextHop: string
  hops: number
  updatedAt: number
}

type RoutedControlPayload =
  | { kind: 'ping'; nonce: string; from?: string }
  | { kind: 'pong'; nonce: string; from?: string }

const SUPERVISOR_STATE_KEY = 'edgerun.webrtc.peerSupervisor.v1'
const SUPERVISOR_DEVICE_ID_KEY = 'edgerun.webrtc.localDeviceId.v1'
export const ROUTED_MESSAGE_EVENT = 'edgerun:webrtc-routed-message'
export const ROUTE_SUPERVISOR_STATUS_EVENT = 'edgerun:route-supervisor-status'
const RTC_CONFIG: RTCConfiguration = {
  iceServers: [
    { urls: 'stun:stun.l.google.com:19302' },
    { urls: 'stun:stun.cloudflare.com:3478' }
  ]
}
const ROUTE_MAX_TTL = 8
const ROUTE_MAX_HOPS = 8
const ROUTE_ADVERT_INTERVAL_MS = 12_000
const ROUTE_EXPIRY_MS = 60_000
const PACKET_DEDUPE_TTL_MS = 30_000

export type RouteSupervisorStatus = {
  controlBase: string
  localDeviceId: string
  started: boolean
  controlSignalConnected: boolean
  controlSignalConnectedAt: number | null
  controlSignalDisconnectedAt: number | null
  directPeers: number
  routeEntries: number
  intents: number
  lastAdvertBroadcastAt: number
  lastRouteAdvertReceivedAt: number
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
  private readonly routes = new Map<string, RouteRecord>()
  private readonly seenPacketIds = new Map<string, number>()
  private intents: IntentState
  private started = false
  private routeAdvertTimer: number | null = null
  private controlSignalConnected = false
  private controlSignalConnectedAt: number | null = null
  private controlSignalDisconnectedAt: number | null = null
  private lastAdvertBroadcastAt = 0
  private lastRouteAdvertReceivedAt = 0

  constructor(controlBase?: string, localDeviceId?: string) {
    const resolvedBase = normalizeControlBase(controlBase || getRouteControlBase())
    if (!resolvedBase) throw new Error('invalid route control base')
    this.controlBase = resolvedBase
    this.localDeviceId = localDeviceId?.trim() || persistentLocalDeviceId()
    this.intents = parseIntentState(typeof window !== 'undefined' ? window.localStorage.getItem(SUPERVISOR_STATE_KEY) : null)
    this.signal = new WebRtcSignalClient(this.controlBase, this.localDeviceId)
    this.signal.onMessage = (message) => {
      void this.handleSignalMessage(message)
    }
    this.signal.onOpen = () => {
      this.controlSignalConnected = true
      this.controlSignalConnectedAt = Date.now()
      this.controlSignalDisconnectedAt = null
    }
    this.signal.onClose = () => {
      this.controlSignalConnected = false
      this.controlSignalDisconnectedAt = Date.now()
      this.scheduleReconnectAll(1200)
    }
  }

  start(): void {
    if (this.started) return
    this.started = true
    this.startRouteAdvertisements()
    for (const deviceId of this.intents.deviceIds) {
      void this.connectToDevice(deviceId)
    }
  }

  getLocalDeviceId(): string {
    return this.localDeviceId
  }

  getStatus(): RouteSupervisorStatus {
    this.pruneRoutes()
    return {
      controlBase: this.controlBase,
      localDeviceId: this.localDeviceId,
      started: this.started,
      controlSignalConnected: this.controlSignalConnected,
      controlSignalConnectedAt: this.controlSignalConnectedAt,
      controlSignalDisconnectedAt: this.controlSignalDisconnectedAt,
      directPeers: this.directOpenNeighbors().length,
      routeEntries: this.routes.size,
      intents: this.intents.deviceIds.length,
      lastAdvertBroadcastAt: this.lastAdvertBroadcastAt,
      lastRouteAdvertReceivedAt: this.lastRouteAdvertReceivedAt
    }
  }

  stop(): void {
    this.started = false
    this.controlSignalConnected = false
    this.controlSignalDisconnectedAt = Date.now()
    if (this.routeAdvertTimer !== null) {
      window.clearInterval(this.routeAdvertTimer)
      this.routeAdvertTimer = null
    }
    this.signal.close()
    for (const session of this.sessions.values()) {
      this.teardownSession(session, false)
    }
    this.sessions.clear()
    this.routes.clear()
    this.seenPacketIds.clear()
  }

  async connectToRouteTarget(target: string): Promise<void> {
    const deviceId = parseRouteDeviceId(target)
    if (!deviceId) return
    await this.connectToDevice(deviceId)
  }

  async connectToDevice(deviceId: string): Promise<void> {
    if (!this.started) {
      this.start()
    }
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

  sendRoutedText(targetDeviceId: string, text: string): boolean {
    const target = targetDeviceId.trim()
    if (!target || target === this.localDeviceId) return false
    const nextHop = this.resolveNextHop(target)
    if (!nextHop) return false
    const packet: RoutedPacket = {
      kind: 'relay',
      packet_id: this.packetId(),
      src: this.localDeviceId,
      dst: target,
      ttl: ROUTE_MAX_TTL,
      payload: text,
      via: [this.localDeviceId]
    }
    return this.sendPacketToPeer(nextHop, packet)
  }

  hasRouteTo(targetDeviceId: string): boolean {
    const target = targetDeviceId.trim()
    if (!target || target === this.localDeviceId) return false
    return Boolean(this.resolveNextHop(target))
  }

  async waitForRoutedPong(targetDeviceId: string, timeoutMs = 1400): Promise<boolean> {
    const target = targetDeviceId.trim()
    if (!target) return false
    const nonce = this.packetId()

    return new Promise<boolean>((resolve) => {
      let settled = false
      const onMessage = (event: Event) => {
        const custom = event as CustomEvent<{ from?: string; payload?: string }>
        const from = String(custom.detail?.from || '').trim()
        if (from !== target) return
        const payload = String(custom.detail?.payload || '')
        const control = this.parseControlPayload(payload)
        if (!control || control.kind !== 'pong' || control.nonce !== nonce) return
        if (settled) return
        settled = true
        window.clearTimeout(timeoutId)
        window.removeEventListener(ROUTED_MESSAGE_EVENT, onMessage as EventListener)
        resolve(true)
      }
      const timeoutId = window.setTimeout(() => {
        if (settled) return
        settled = true
        window.removeEventListener(ROUTED_MESSAGE_EVENT, onMessage as EventListener)
        resolve(false)
      }, timeoutMs)
      window.addEventListener(ROUTED_MESSAGE_EVENT, onMessage as EventListener)
      const sent = this.sendRoutedText(target, JSON.stringify({
        kind: 'ping',
        nonce,
        from: this.localDeviceId
      } satisfies RoutedControlPayload))
      if (!sent) {
        settled = true
        window.clearTimeout(timeoutId)
        window.removeEventListener(ROUTED_MESSAGE_EVENT, onMessage as EventListener)
        resolve(false)
      }
    })
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
        this.setRoute(deviceId, deviceId, 1)
        this.broadcastRouteAdvert()
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
      this.setRoute(session.deviceId, session.deviceId, 1)
      this.sendPacketToPeer(session.deviceId, {
        kind: 'hello',
        from: this.localDeviceId,
        at: Date.now(),
        neighbors: this.directOpenNeighbors().filter((id) => id !== session.deviceId)
      })
      this.broadcastRouteAdvert()
    }
    dc.onclose = () => {
      this.dropRoutesVia(session.deviceId)
      this.scheduleReconnect(session.deviceId)
    }
    dc.onerror = () => {
      this.dropRoutesVia(session.deviceId)
      this.scheduleReconnect(session.deviceId)
    }
    dc.onmessage = (event) => {
      if (typeof event.data !== 'string') return
      this.handleDataChannelMessage(session.deviceId, event.data)
    }
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
    this.dropRoutesVia(deviceId)
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
    this.dropRoutesVia(session.deviceId)
    if (removeIntent) this.removeIntent(session.deviceId)
  }

  private packetId(): string {
    const suffix = window.crypto?.randomUUID ? window.crypto.randomUUID() : `${Date.now()}-${Math.random().toString(36).slice(2)}`
    return `${this.localDeviceId}-${suffix}`
  }

  private isOpenPeer(deviceId: string): boolean {
    const session = this.sessions.get(deviceId)
    return Boolean(session?.dc && session.dc.readyState === 'open')
  }

  private directOpenNeighbors(): string[] {
    const out: string[] = []
    for (const [deviceId, session] of this.sessions) {
      if (session.dc && session.dc.readyState === 'open') out.push(deviceId)
    }
    return out
  }

  private setRoute(target: string, nextHop: string, hops: number): void {
    if (!target || !nextHop) return
    if (target === this.localDeviceId) return
    if (hops < 1 || hops > ROUTE_MAX_HOPS) return
    const existing = this.routes.get(target)
    const now = Date.now()
    if (!existing || hops < existing.hops || existing.nextHop === nextHop) {
      this.routes.set(target, { nextHop, hops, updatedAt: now })
    }
  }

  private dropRoutesVia(nextHop: string): void {
    for (const [target, route] of this.routes) {
      if (route.nextHop === nextHop || target === nextHop) {
        this.routes.delete(target)
      }
    }
  }

  private pruneRoutes(): void {
    const now = Date.now()
    for (const [target, route] of this.routes) {
      if (now - route.updatedAt > ROUTE_EXPIRY_MS) {
        this.routes.delete(target)
      }
    }
    for (const [packetId, seenAt] of this.seenPacketIds) {
      if (now - seenAt > PACKET_DEDUPE_TTL_MS) {
        this.seenPacketIds.delete(packetId)
      }
    }
  }

  private resolveNextHop(target: string): string | null {
    if (this.isOpenPeer(target)) return target
    this.pruneRoutes()
    const route = this.routes.get(target)
    if (!route) return null
    if (!this.isOpenPeer(route.nextHop)) {
      this.routes.delete(target)
      return null
    }
    return route.nextHop
  }

  private sendPacketToPeer(peerId: string, packet: SupervisorWirePacket): boolean {
    const session = this.sessions.get(peerId)
    if (!session?.dc || session.dc.readyState !== 'open') return false
    try {
      session.dc.send(JSON.stringify(packet))
      return true
    } catch {
      return false
    }
  }

  private handleDataChannelMessage(fromPeerId: string, raw: string): void {
    let packet: SupervisorWirePacket | null = null
    try {
      const parsed = JSON.parse(raw) as SupervisorWirePacket
      if (!parsed || typeof parsed !== 'object' || !('kind' in parsed)) return
      packet = parsed
    } catch {
      return
    }
    if (!packet) return

    if (packet.kind === 'hello') {
      this.setRoute(fromPeerId, fromPeerId, 1)
      const neighbors = Array.isArray(packet.neighbors) ? packet.neighbors : []
      for (const neighbor of neighbors) {
        const target = String(neighbor || '').trim()
        if (!target || target === this.localDeviceId || target === fromPeerId) continue
        this.setRoute(target, fromPeerId, 2)
      }
      this.broadcastRouteAdvert()
      return
    }

    if (packet.kind === 'route-advert') {
      this.lastRouteAdvertReceivedAt = Date.now()
      this.setRoute(fromPeerId, fromPeerId, 1)
      const routes = Array.isArray(packet.routes) ? packet.routes : []
      for (const route of routes) {
        const target = String(route.target || '').trim()
        const hops = Number(route.hops)
        if (!target || !Number.isFinite(hops)) continue
        if (target === this.localDeviceId || target === fromPeerId) continue
        const derivedHops = Math.max(2, Math.min(ROUTE_MAX_HOPS, Math.round(hops) + 1))
        this.setRoute(target, fromPeerId, derivedHops)
      }
      return
    }

    this.handleRelayPacket(fromPeerId, packet)
  }

  private handleRelayPacket(fromPeerId: string, packet: RoutedPacket): void {
    if (!packet.packet_id || !packet.src || !packet.dst) return
    if (this.seenPacketIds.has(packet.packet_id)) return
    this.seenPacketIds.set(packet.packet_id, Date.now())
    this.setRoute(fromPeerId, fromPeerId, 1)
    if (packet.src !== this.localDeviceId) this.setRoute(packet.src, fromPeerId, 2)

    if (packet.dst === this.localDeviceId) {
      this.handleRoutedControlPayload(packet.src, packet.payload)
      window.dispatchEvent(new CustomEvent(ROUTED_MESSAGE_EVENT, {
        detail: {
          from: packet.src,
          to: packet.dst,
          payload: packet.payload,
          via: packet.via || []
        }
      }))
      return
    }

    if (packet.ttl <= 1) return
    const nextHop = this.resolveNextHop(packet.dst)
    if (!nextHop) return
    const traversed = new Set([...(packet.via || []), this.localDeviceId])
    if (traversed.has(nextHop)) return
    const forwarded: RoutedPacket = {
      ...packet,
      ttl: packet.ttl - 1,
      via: [...traversed]
    }
    this.sendPacketToPeer(nextHop, forwarded)
  }

  private parseControlPayload(payload: string): RoutedControlPayload | null {
    try {
      const parsed = JSON.parse(payload) as RoutedControlPayload
      if (!parsed || typeof parsed !== 'object') return null
      if (parsed.kind !== 'ping' && parsed.kind !== 'pong') return null
      if (typeof parsed.nonce !== 'string' || !parsed.nonce.trim()) return null
      return {
        kind: parsed.kind,
        nonce: parsed.nonce.trim(),
        from: typeof parsed.from === 'string' && parsed.from.trim() ? parsed.from.trim() : undefined
      }
    } catch {
      return null
    }
  }

  private handleRoutedControlPayload(sourceDeviceId: string, payload: string): void {
    const control = this.parseControlPayload(payload)
    if (!control) return
    if (control.kind !== 'ping') return
    this.sendRoutedText(sourceDeviceId, JSON.stringify({
      kind: 'pong',
      nonce: control.nonce,
      from: this.localDeviceId
    } satisfies RoutedControlPayload))
  }

  private buildAdvertRoutesForPeer(peerId: string): Array<{ target: string; hops: number }> {
    this.pruneRoutes()
    const advertised: Array<{ target: string; hops: number }> = []
    for (const neighbor of this.directOpenNeighbors()) {
      if (neighbor === peerId) continue
      advertised.push({ target: neighbor, hops: 1 })
    }
    for (const [target, route] of this.routes) {
      if (target === peerId || target === this.localDeviceId) continue
      if (route.nextHop === peerId) continue
      advertised.push({ target, hops: Math.min(route.hops, ROUTE_MAX_HOPS) })
    }
    return advertised.slice(0, 64)
  }

  private broadcastRouteAdvert(): void {
    this.lastAdvertBroadcastAt = Date.now()
    for (const peerId of this.directOpenNeighbors()) {
      this.sendPacketToPeer(peerId, {
        kind: 'route-advert',
        from: this.localDeviceId,
        at: Date.now(),
        routes: this.buildAdvertRoutesForPeer(peerId)
      })
    }
  }

  private startRouteAdvertisements(): void {
    if (this.routeAdvertTimer !== null) return
    this.routeAdvertTimer = window.setInterval(() => {
      this.broadcastRouteAdvert()
      this.pruneRoutes()
    }, ROUTE_ADVERT_INTERVAL_MS)
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
