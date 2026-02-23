// SPDX-License-Identifier: Apache-2.0
import { For, createSignal, onCleanup, onMount, untrack } from 'solid-js'
import { terminalDrawerActions } from '../../lib/terminal-drawer-store'
import { ROUTED_MESSAGE_EVENT, getWebRtcPeerSupervisor } from '../../lib/webrtc-peer-supervisor'
import { getRouteControlBase, resolveDeviceRoute, routeRelayWsUrl } from '../../lib/webrtc-route-client'
import { encodeRoutedTerminalFrame, parseRoutedTerminalFrame } from '../../lib/routed-terminal-protocol'

type Props = {
  paneId: string
  routeDeviceId: string
}

type RoutedMessageEvent = CustomEvent<{
  from?: string
  payload?: string
}>

const LOCALHOST_NAMES = new Set(['127.0.0.1', 'localhost', '::1'])
const REFRESH_INTERVAL_MS = 8000
const REFRESH_COOLDOWN_MS = 1500

function stripAnsi(value: string): string {
  let out = ''
  let i = 0
  while (i < value.length) {
    const ch = value.charCodeAt(i)
    if (ch === 0x1b && i + 1 < value.length && value[i + 1] === '[') {
      i += 2
      while (i < value.length) {
        const next = value.charCodeAt(i)
        const isEnd = (next >= 0x40 && next <= 0x7e)
        i += 1
        if (isEnd) break
      }
      continue
    }
    out += value[i] || ''
    i += 1
  }
  return out
}

function sessionIdForPane(paneId: string): string {
  return `pane-${paneId}`
}

function nowLabel(): string {
  return new Date().toLocaleTimeString('en-US', { hour12: false })
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

function localDirectWsCandidate(): string | null {
  if (typeof window === 'undefined') return null
  const host = window.location.hostname.toLowerCase()
  if (!LOCALHOST_NAMES.has(host)) return null
  return 'ws://127.0.0.1:5577/ws'
}

export function RoutedTerminalPane(props: Props) {
  const routeDeviceId = untrack(() => props.routeDeviceId)
  const paneId = untrack(() => props.paneId)
  const sessionId = sessionIdForPane(paneId)
  const [connected, setConnected] = createSignal(false)
  const [acknowledged, setAcknowledged] = createSignal(false)
  const [draft, setDraft] = createSignal('')
  const [lines, setLines] = createSignal<string[]>([])
  let directSocket: WebSocket | null = null
  let directEndpoint = ''
  let refreshInFlight = false
  let nextRefreshAllowedAt = 0

  function append(text: string): void {
    const normalized = stripAnsi(text).split('\u001bc').join('')
    if (!normalized.trim()) return
    setLines((prev) => {
      const chunks = normalized.split('\n').filter((line) => line.length > 0)
      const next = [...prev, ...chunks]
      return next.slice(-300)
    })
  }

  function setClearBuffer(): void {
    setLines([])
  }

  function closeDirectSocket(): void {
    if (!directSocket) return
    try {
      directSocket.close()
    } catch {
      // ignore socket close errors
    }
    directSocket = null
    directEndpoint = ''
  }

  function appendDirectData(data: unknown): void {
    if (typeof data === 'string') {
      append(data)
      return
    }
    if (data instanceof ArrayBuffer) {
      append(new TextDecoder().decode(new Uint8Array(data)))
      return
    }
    if (data instanceof Blob) {
      void data.arrayBuffer().then((buf) => append(new TextDecoder().decode(new Uint8Array(buf)))).catch(() => {})
      return
    }
    if (data instanceof Uint8Array) {
      append(new TextDecoder().decode(data))
    }
  }

  function ensureDirectSocket(endpoint: string): void {
    const target = endpoint.trim()
    if (!target) return
    if (directSocket && directEndpoint === target) {
      if (directSocket.readyState === WebSocket.OPEN || directSocket.readyState === WebSocket.CONNECTING) {
        const open = directSocket.readyState === WebSocket.OPEN
        setConnected(open)
        setAcknowledged(open)
        terminalDrawerActions.setPaneTransport(paneId, open ? 'raw' : 'unknown')
        return
      }
      closeDirectSocket()
    }

    closeDirectSocket()
    directEndpoint = target
    const ws = new WebSocket(target)
    ws.binaryType = 'arraybuffer'
    ws.addEventListener('open', () => {
      setConnected(true)
      setAcknowledged(true)
      terminalDrawerActions.setPaneTransport(paneId, 'raw')
      append(`[${nowLabel()}] direct ws connected`)
    })
    ws.addEventListener('close', () => {
      if (directSocket !== ws) return
      setConnected(false)
      setAcknowledged(false)
      terminalDrawerActions.setPaneTransport(paneId, 'unknown')
      directSocket = null
    })
    ws.addEventListener('error', () => {
      if (directSocket !== ws) return
      setConnected(false)
      setAcknowledged(false)
      terminalDrawerActions.setPaneTransport(paneId, 'unknown')
      directSocket = null
    })
    ws.addEventListener('message', (event) => {
      appendDirectData(event.data)
    })
    directSocket = ws
  }

  function sendOpen(cols = 120, rows = 36): boolean {
    return getWebRtcPeerSupervisor().sendRoutedText(routeDeviceId, encodeRoutedTerminalFrame({
      type: 'open',
      sessionId,
      cols,
      rows,
      term: 'xterm-256color'
    }))
  }

  function sendInput(input: string): boolean {
    return getWebRtcPeerSupervisor().sendRoutedText(routeDeviceId, encodeRoutedTerminalFrame({
      type: 'input',
      sessionId,
      data: input,
      encoding: 'utf8'
    }))
  }

  function sendClose(): void {
    getWebRtcPeerSupervisor().sendRoutedText(routeDeviceId, encodeRoutedTerminalFrame({
      type: 'close',
      sessionId
    }))
  }

  async function refreshState(force = false): Promise<void> {
    if (refreshInFlight) return
    const now = Date.now()
    if (!force && now < nextRefreshAllowedAt) return
    refreshInFlight = true
    nextRefreshAllowedAt = now + REFRESH_COOLDOWN_MS
    try {
    if (directSocket && (directSocket.readyState === WebSocket.OPEN || directSocket.readyState === WebSocket.CONNECTING)) {
      const open = directSocket.readyState === WebSocket.OPEN
      setConnected(open)
      setAcknowledged(open)
      terminalDrawerActions.setPaneTransport(paneId, open ? 'raw' : 'unknown')
      return
    }

    const controlBase = getRouteControlBase()
    const resolvedBase = await resolveDeviceRoute(controlBase, routeDeviceId).catch(() => null)
    const directCandidates = [
      resolvedBase ? toDirectWsUrl(resolvedBase) : null,
      localDirectWsCandidate(),
      routeRelayWsUrl(controlBase, routeDeviceId)
    ]
      .filter((value): value is string => Boolean(value && value.trim()))
      .map((value) => value.trim())
    const uniqueCandidates = [...new Set(directCandidates)]
    const directCandidate = uniqueCandidates[0]
    if (directCandidate) {
      ensureDirectSocket(directCandidate)
      return
    }

    closeDirectSocket()
    const supervisor = getWebRtcPeerSupervisor()
    await supervisor.connectToDevice(routeDeviceId).catch(() => {
      // keep probing with existing route table
    })
    const ok = await supervisor.waitForRoutedPong(routeDeviceId, 1200).catch(() => false)
    setConnected(ok)
    if (!ok) setAcknowledged(false)
    terminalDrawerActions.setPaneTransport(paneId, ok ? 'raw' : 'unknown')
    if (ok && !untrack(() => acknowledged())) {
      const opened = sendOpen()
      if (!opened) {
        append(`[${nowLabel()}] open failed`)
      }
    }
    } finally {
      refreshInFlight = false
    }
  }

  function submitDraft(): void {
    const text = draft()
    if (!text.trim()) return
    let sent = false
    if (directSocket && directSocket.readyState === WebSocket.OPEN) {
      try {
        directSocket.send(`${text}\n`)
        sent = true
      } catch {
        sent = false
      }
    } else {
      sent = sendInput(`${text}\n`)
    }
    append(`[${nowLabel()}] > ${text}`)
    if (!sent) append(`[${nowLabel()}] transport send failed`)
    setDraft('')
  }

  onMount(() => {
    append(`[${nowLabel()}] route://${routeDeviceId} attached`)
    void refreshState(true)
    const intervalId = window.setInterval(() => {
      void refreshState()
    }, REFRESH_INTERVAL_MS)

    const onRoutedMessage = (event: Event) => {
      const custom = event as RoutedMessageEvent
      const from = String(custom.detail?.from || '').trim()
      if (from !== routeDeviceId) return
      const payload = String(custom.detail?.payload || '')
      const frame = parseRoutedTerminalFrame(payload)
      if (!frame || frame.sessionId !== sessionId) return
      if (frame.type === 'ack') {
        if (!frame.ok) {
          append(`[${nowLabel()}] open rejected: ${frame.message || 'unknown'}`)
          return
        }
        setAcknowledged(true)
        append(`[${nowLabel()}] ${frame.message || 'session acknowledged'}`)
        return
      }
      if (frame.type === 'output') {
        if (frame.data.includes('\u001bc')) setClearBuffer()
        append(frame.data)
        return
      }
      if (frame.type === 'error') {
        append(`[${nowLabel()}] error[${frame.code}]: ${frame.message}`)
        return
      }
      if (frame.type === 'exit') {
        append(`[${nowLabel()}] remote exited${typeof frame.code === 'number' ? ` (${frame.code})` : ''}`)
        setAcknowledged(false)
        return
      }
      if (frame.type === 'close') {
        append(`[${nowLabel()}] remote requested close`)
        setAcknowledged(false)
      }
    }

    window.addEventListener(ROUTED_MESSAGE_EVENT, onRoutedMessage as EventListener)
    onCleanup(() => {
      window.clearInterval(intervalId)
      window.removeEventListener(ROUTED_MESSAGE_EVENT, onRoutedMessage as EventListener)
      if (directSocket) {
        closeDirectSocket()
      } else {
        sendClose()
      }
      terminalDrawerActions.setPaneTransport(paneId, 'unknown')
    })
  })

  return (
    <div class="flex h-full min-h-0 flex-col rounded-md border border-border/70 bg-background/40">
      <div class="flex items-center justify-between border-b border-border/70 px-3 py-2 text-[11px]">
        <span class="font-mono text-muted-foreground">route://{routeDeviceId}</span>
        <span class={connected() ? 'text-emerald-400' : 'text-rose-400'}>
          {connected() ? (acknowledged() ? 'session-open' : 'route-online') : 'route-offline'}
        </span>
      </div>
      <div class="min-h-0 flex-1 overflow-auto bg-black/60 px-3 py-2 font-mono text-xs text-emerald-200" data-testid="routed-terminal-log">
        <For each={lines()}>{(line) => <p>{line}</p>}</For>
      </div>
      <div class="flex items-center gap-2 border-t border-border/70 p-2">
        <input
          class="h-8 min-w-0 flex-1 rounded-md border border-border bg-background/80 px-2 font-mono text-xs text-foreground"
          placeholder={connected() ? 'Type command (help, date, whoami, echo ...)' : 'Waiting for route...'}
          value={draft()}
          onInput={(event) => setDraft(event.currentTarget.value)}
          onKeyDown={(event) => {
            if (event.key !== 'Enter') return
            event.preventDefault()
            submitDraft()
          }}
        />
        <button
          type="button"
          class="h-8 rounded-md border border-border/70 bg-card/60 px-3 text-xs text-muted-foreground hover:text-foreground disabled:opacity-50"
          disabled={!connected() || !acknowledged()}
          onClick={submitDraft}
        >
          Send
        </button>
      </div>
    </div>
  )
}
