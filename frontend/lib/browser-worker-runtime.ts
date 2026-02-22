// SPDX-License-Identifier: Apache-2.0
import { readWalletSession } from './wallet-session'

type WorkerCapacity = {
  max_concurrent: number
  mem_bytes: number
}

export type BrowserWorkerConfig = {
  schedulerBaseUrl: string
  workerPubkey: string
  runtimeIds: string[]
  version: string
  capacity: WorkerCapacity
}

export type BrowserWorkerStats = {
  running: boolean
  heartbeats: number
  assignmentsSeen: number
  resultsSubmitted: number
  failuresSubmitted: number
  lastError: string
  lastEvent: string
}

export type BrowserWorkerEvents = {
  onStats?: (stats: BrowserWorkerStats) => void
  onLog?: (line: string) => void
}

export type BrowserWorkerController = {
  stop: () => void
  stats: () => BrowserWorkerStats
}

type MainToWorkerMessage =
  | { type: 'start'; config: BrowserWorkerConfig }
  | { type: 'stop' }

type WorkerToMainMessage =
  | { type: 'stats'; stats: BrowserWorkerStats }
  | { type: 'log'; line: string }

type RuntimeControllerState = {
  worker: Worker
  latestStats: BrowserWorkerStats
}

const EMPTY_STATS: BrowserWorkerStats = {
  running: false,
  heartbeats: 0,
  assignmentsSeen: 0,
  resultsSubmitted: 0,
  failuresSubmitted: 0,
  lastError: '',
  lastEvent: 'idle'
}

const BROWSER_RUNTIME_WORKER_ASSET = '/assets/browser-worker-runtime.worker.js'

function defaultSchedulerUrl(): string {
  if (typeof window === 'undefined') return 'http://127.0.0.1:8090'
  const injected = String((window as any).__EDGERUN_API_BASE || '').trim()
  if (injected) return injected
  const host = window.location.hostname
  if (host === '127.0.0.1' || host === 'localhost') return 'http://127.0.0.1:8090'
  return 'https://api.edgerun.tech'
}

export function defaultBrowserWorkerConfig(): BrowserWorkerConfig {
  const wallet = readWalletSession()
  return {
    schedulerBaseUrl: defaultSchedulerUrl(),
    workerPubkey: wallet.address.trim() || fallbackBrowserWorkerPubkey(),
    runtimeIds: ['0000000000000000000000000000000000000000000000000000000000000000'],
    version: 'browser-0.1.0',
    capacity: {
      max_concurrent: 1,
      mem_bytes: 268_435_456
    }
  }
}

export function startBrowserWorker(
  inputConfig: BrowserWorkerConfig,
  events: BrowserWorkerEvents = {}
): BrowserWorkerController {
  const config = normalizeConfig(inputConfig)
  const state: RuntimeControllerState = {
    worker: new Worker(resolveWorkerAssetUrl(BROWSER_RUNTIME_WORKER_ASSET), { type: 'module' }),
    latestStats: {
      ...EMPTY_STATS,
      running: true,
      lastEvent: 'starting'
    }
  }
  let stopped = false

  state.worker.addEventListener('message', (event: MessageEvent<WorkerToMainMessage>) => {
    const message = event.data
    if (!message || typeof message !== 'object') return
    if (message.type === 'stats') {
      state.latestStats = message.stats
      events.onStats?.({ ...state.latestStats })
      return
    }
    if (message.type === 'log') {
      events.onLog?.(message.line)
    }
  })

  state.worker.addEventListener('error', (event: ErrorEvent) => {
    state.latestStats = {
      ...state.latestStats,
      running: false,
      lastEvent: 'stopped-with-error',
      lastError: event.message || 'runtime worker crashed'
    }
    events.onStats?.({ ...state.latestStats })
    events.onLog?.(`runtime worker error: ${state.latestStats.lastError}`)
  })

  state.worker.postMessage({
    type: 'start',
    config
  } satisfies MainToWorkerMessage)

  events.onStats?.({ ...state.latestStats })

  const stop = () => {
    if (stopped) return
    stopped = true
    state.worker.postMessage({ type: 'stop' } satisfies MainToWorkerMessage)
    state.worker.terminate()
    state.latestStats = {
      ...state.latestStats,
      running: false,
      lastEvent: 'stopped'
    }
    events.onStats?.({ ...state.latestStats })
    events.onLog?.('worker stopped')
  }

  return {
    stop,
    stats: () => ({ ...state.latestStats })
  }
}

function normalizeConfig(input: BrowserWorkerConfig): BrowserWorkerConfig {
  const schedulerBaseUrl = input.schedulerBaseUrl.trim().replace(/\/+$/, '')
  const workerPubkey = input.workerPubkey.trim()
  const runtimeIds = input.runtimeIds.map((id) => id.trim()).filter((id) => id.length > 0)
  return {
    schedulerBaseUrl,
    workerPubkey,
    runtimeIds: runtimeIds.length > 0
      ? runtimeIds
      : ['0000000000000000000000000000000000000000000000000000000000000000'],
    version: input.version.trim() || 'browser-0.1.0',
    capacity: {
      max_concurrent: Math.max(1, Math.floor(input.capacity.max_concurrent || 1)),
      mem_bytes: Math.max(64 * 1024 * 1024, Math.floor(input.capacity.mem_bytes || 268_435_456))
    }
  }
}

function fallbackBrowserWorkerPubkey(): string {
  const alphabet = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'
  let out = 'browser-worker-'
  for (let i = 0; i < 24; i += 1) {
    const idx = Math.floor(Math.random() * alphabet.length)
    out += alphabet[idx]
  }
  return out
}

function resolveWorkerAssetUrl(path: string): URL {
  if (typeof window !== 'undefined') {
    return new URL(path, window.location.origin)
  }
  return new URL(path, 'http://127.0.0.1')
}
