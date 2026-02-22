export type RoutedTerminalFrame =
  | {
      proto: 'edgerun.rterm.v2'
      v: 2
      type: 'open'
      sessionId: string
      cols: number
      rows: number
      term: string
    }
  | {
      proto: 'edgerun.rterm.v2'
      v: 2
      type: 'ack'
      sessionId: string
      ok: boolean
      message?: string
      capabilities?: string[]
    }
  | {
      proto: 'edgerun.rterm.v2'
      v: 2
      type: 'input'
      sessionId: string
      data: string
      encoding: 'utf8'
    }
  | {
      proto: 'edgerun.rterm.v2'
      v: 2
      type: 'output'
      sessionId: string
      data: string
      stream: 'stdout' | 'stderr'
      encoding: 'utf8'
    }
  | { proto: 'edgerun.rterm.v2'; v: 2; type: 'resize'; sessionId: string; cols: number; rows: number }
  | { proto: 'edgerun.rterm.v2'; v: 2; type: 'close'; sessionId: string }
  | { proto: 'edgerun.rterm.v2'; v: 2; type: 'exit'; sessionId: string; code?: number; reason?: string }
  | {
      proto: 'edgerun.rterm.v2'
      v: 2
      type: 'error'
      sessionId: string
      code: string
      message: string
      retriable?: boolean
    }

export type RoutedTerminalFramePayload =
  | { type: 'open'; sessionId: string; cols: number; rows: number; term: string }
  | { type: 'ack'; sessionId: string; ok: boolean; message?: string; capabilities?: string[] }
  | { type: 'input'; sessionId: string; data: string; encoding: 'utf8' }
  | { type: 'output'; sessionId: string; data: string; stream: 'stdout' | 'stderr'; encoding: 'utf8' }
  | { type: 'resize'; sessionId: string; cols: number; rows: number }
  | { type: 'close'; sessionId: string }
  | { type: 'exit'; sessionId: string; code?: number; reason?: string }
  | { type: 'error'; sessionId: string; code: string; message: string; retriable?: boolean }

const PROTO = 'edgerun.rterm.v2'

export function encodeRoutedTerminalFrame(frame: RoutedTerminalFramePayload): string {
  return JSON.stringify({
    proto: PROTO,
    v: 2,
    ...frame
  })
}

export function parseRoutedTerminalFrame(payload: string): RoutedTerminalFrame | null {
  try {
    const parsed = JSON.parse(payload) as Partial<RoutedTerminalFrame>
    if (!parsed || parsed.proto !== PROTO || parsed.v !== 2 || typeof parsed.type !== 'string') return null
    if (typeof parsed.sessionId !== 'string' || !parsed.sessionId.trim()) return null
    if (parsed.type === 'open' || parsed.type === 'resize') {
      if (typeof (parsed as any).cols !== 'number' || typeof (parsed as any).rows !== 'number') return null
    }
    if (parsed.type === 'open' && typeof (parsed as any).term !== 'string') return null
    if (parsed.type === 'input' || parsed.type === 'output') {
      if (typeof (parsed as any).data !== 'string') return null
      if ((parsed as any).encoding !== 'utf8') return null
    }
    if (parsed.type === 'output' && (parsed as any).stream !== 'stdout' && (parsed as any).stream !== 'stderr') return null
    if (parsed.type === 'ack' && typeof (parsed as any).ok !== 'boolean') return null
    if (parsed.type === 'error') {
      if (typeof (parsed as any).code !== 'string') return null
      if (typeof (parsed as any).message !== 'string') return null
    }
    return parsed as RoutedTerminalFrame
  } catch {
    return null
  }
}
