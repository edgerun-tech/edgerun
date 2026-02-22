import { ROUTED_MESSAGE_EVENT, getWebRtcPeerSupervisor } from './webrtc-peer-supervisor'
import {
  encodeRoutedTerminalFrame,
  parseRoutedTerminalFrame,
  type RoutedTerminalFrame,
  type RoutedTerminalFramePayload
} from './routed-terminal-protocol'

type RoutedMessageEvent = CustomEvent<{
  from?: string
  payload?: string
}>

type ShellSession = {
  remoteDeviceId: string
  sessionId: string
  startedAt: number
}

const sessions = new Map<string, ShellSession>()
let initialized = false

function keyOf(remoteDeviceId: string, sessionId: string): string {
  return `${remoteDeviceId}::${sessionId}`
}

function nowIso(): string {
  return new Date().toISOString()
}

function promptFor(deviceId: string): string {
  return `edgerun@${deviceId.slice(0, 10)}$ `
}

function sendFrame(targetDeviceId: string, frame: RoutedTerminalFramePayload): void {
  getWebRtcPeerSupervisor().sendRoutedText(targetDeviceId, encodeRoutedTerminalFrame(frame))
}

function lines(text: string): string[] {
  return text.replace(/\r/g, '').split('\n')
}

function executeCommand(input: string): string {
  const trimmed = input.trim()
  if (!trimmed) return ''
  if (trimmed === 'help') {
    return [
      'Routed shell commands:',
      '  help        show this help',
      '  whoami      show routed device id',
      '  date        show current ISO timestamp',
      '  peers       show known direct peers',
      '  clear       clear terminal output (client-side)',
      '  echo <txt>  print text',
      '  exit        close session'
    ].join('\n')
  }
  if (trimmed === 'whoami') {
    return getWebRtcPeerSupervisor().getLocalDeviceId()
  }
  if (trimmed === 'date') {
    return nowIso()
  }
  if (trimmed === 'peers') {
    return 'Peer visibility depends on active routed links.'
  }
  if (trimmed === 'clear') {
    return '__EDGERUN_RTERM_CLEAR__'
  }
  if (trimmed === 'exit') {
    return '__EDGERUN_RTERM_EXIT__'
  }
  if (trimmed.startsWith('echo ')) {
    return trimmed.slice(5)
  }
  return `unknown command: ${trimmed}`
}

function onOpen(remoteDeviceId: string, frame: Extract<RoutedTerminalFrame, { type: 'open' }>): void {
  sessions.set(keyOf(remoteDeviceId, frame.sessionId), {
    remoteDeviceId,
    sessionId: frame.sessionId,
    startedAt: Date.now()
  })
  sendFrame(remoteDeviceId, {
    type: 'ack',
    sessionId: frame.sessionId,
    ok: true,
    message: `routed shell ready (${frame.cols}x${frame.rows})`
  })
  sendFrame(remoteDeviceId, {
    type: 'output',
    sessionId: frame.sessionId,
    stream: 'stdout',
    encoding: 'utf8',
    data: `Connected to routed shell at ${nowIso()}\n${promptFor(getWebRtcPeerSupervisor().getLocalDeviceId())}`
  })
}

function onInput(remoteDeviceId: string, frame: Extract<RoutedTerminalFrame, { type: 'input' }>): void {
  const session = sessions.get(keyOf(remoteDeviceId, frame.sessionId))
  if (!session) {
    sendFrame(remoteDeviceId, {
      type: 'error',
      sessionId: frame.sessionId,
      code: 'session_not_found',
      message: 'session-not-found',
      retriable: true
    })
    return
  }
  const input = frame.data
  for (const line of lines(input)) {
    const output = executeCommand(line)
    if (!output) continue
    if (output === '__EDGERUN_RTERM_CLEAR__') {
      sendFrame(remoteDeviceId, {
        type: 'output',
        sessionId: frame.sessionId,
        stream: 'stdout',
        encoding: 'utf8',
        data: '\x1bc'
      })
      sendFrame(remoteDeviceId, {
        type: 'output',
        sessionId: frame.sessionId,
        stream: 'stdout',
        encoding: 'utf8',
        data: promptFor(getWebRtcPeerSupervisor().getLocalDeviceId())
      })
      continue
    }
    if (output === '__EDGERUN_RTERM_EXIT__') {
      sendFrame(remoteDeviceId, {
        type: 'exit',
        sessionId: frame.sessionId,
        code: 0,
        reason: 'client_requested_exit'
      })
      sessions.delete(keyOf(remoteDeviceId, frame.sessionId))
      return
    }
    sendFrame(remoteDeviceId, {
      type: 'output',
      sessionId: frame.sessionId,
      stream: 'stdout',
      encoding: 'utf8',
      data: `${output}\n${promptFor(getWebRtcPeerSupervisor().getLocalDeviceId())}`
    })
  }
}

function onClose(remoteDeviceId: string, frame: Extract<RoutedTerminalFrame, { type: 'close' }>): void {
  sessions.delete(keyOf(remoteDeviceId, frame.sessionId))
}

function handleFrame(remoteDeviceId: string, frame: RoutedTerminalFrame): void {
  if (frame.type === 'open') return onOpen(remoteDeviceId, frame)
  if (frame.type === 'input') return onInput(remoteDeviceId, frame)
  if (frame.type === 'close') return onClose(remoteDeviceId, frame)
}

export function initRoutedTerminalShell(): void {
  if (initialized || typeof window === 'undefined') return
  initialized = true
  window.addEventListener(ROUTED_MESSAGE_EVENT, (event: Event) => {
    const custom = event as RoutedMessageEvent
    const from = String(custom.detail?.from || '').trim()
    const payload = String(custom.detail?.payload || '')
    if (!from || !payload) return
    const frame = parseRoutedTerminalFrame(payload)
    if (!frame) return
    handleFrame(from, frame)
  })
}
