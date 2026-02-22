export type ThreadBenchmarkInput = {
  iterations: number
  workScale: number
  maxPayloadMb: number
}

export type CpuBenchmarkSummary = {
  totalMs: number
  avgMs: number
  p95Ms: number
  opsPerSec: number
  checksum: number
}

export type PayloadProbeSummary = {
  maxMainPayloadMb: number
  maxWorkerPayloadMb: number
}

export type ThreadBenchmarkReport = {
  main: CpuBenchmarkSummary
  worker: CpuBenchmarkSummary
  speedup: number
  payload: PayloadProbeSummary
}

type CpuBenchmarkRequest = {
  id: string
  type: 'cpu-benchmark'
  iterations: number
  workScale: number
}

type PayloadProbeRequest = {
  id: string
  type: 'payload-probe'
  payload: ArrayBuffer
}

type CpuBenchmarkResponse = {
  id: string
  type: 'cpu-benchmark'
  totalMs: number
  durationsMs: number[]
  checksum: number
}

type PayloadProbeResponse = {
  id: string
  type: 'payload-probe'
  bytes: number
  checksum: number
}

type BenchmarkResponse = CpuBenchmarkResponse | PayloadProbeResponse

const THREAD_BENCHMARK_WORKER_ASSET = '/assets/thread-benchmark.worker.js'

export async function runThreadBenchmark(input: ThreadBenchmarkInput): Promise<ThreadBenchmarkReport> {
  const params = normalizeInput(input)
  const worker = new Worker(resolveWorkerAssetUrl(THREAD_BENCHMARK_WORKER_ASSET), { type: 'module' })
  try {
    const mainRaw = runCpuBenchmarkLocal(params.iterations, params.workScale)
    const workerRaw = await runWorkerCpuBenchmark(worker, params.iterations, params.workScale)
    const payload = await probePayloadLimits(worker, params.maxPayloadMb)
    const main = summarize(mainRaw.totalMs, mainRaw.durationsMs, params.iterations, mainRaw.checksum)
    const workerSummary = summarize(workerRaw.totalMs, workerRaw.durationsMs, params.iterations, workerRaw.checksum)
    const speedup = workerSummary.totalMs > 0 ? main.totalMs / workerSummary.totalMs : 0
    return {
      main,
      worker: workerSummary,
      speedup,
      payload
    }
  } finally {
    worker.terminate()
  }
}

function normalizeInput(input: ThreadBenchmarkInput): ThreadBenchmarkInput {
  return {
    iterations: Math.max(1, Math.floor(input.iterations)),
    workScale: Math.max(1000, Math.floor(input.workScale)),
    maxPayloadMb: Math.max(1, Math.floor(input.maxPayloadMb))
  }
}

function runCpuBenchmarkLocal(iterations: number, workScale: number): {
  totalMs: number
  durationsMs: number[]
  checksum: number
} {
  const durationsMs: number[] = []
  let checksum = 0
  const start = performance.now()
  for (let i = 0; i < iterations; i += 1) {
    const runStart = performance.now()
    checksum ^= computeWork(workScale + i * 17)
    durationsMs.push(performance.now() - runStart)
  }
  return {
    totalMs: performance.now() - start,
    durationsMs,
    checksum: checksum >>> 0
  }
}

async function runWorkerCpuBenchmark(worker: Worker, iterations: number, workScale: number): Promise<{
  totalMs: number
  durationsMs: number[]
  checksum: number
}> {
  const request: CpuBenchmarkRequest = {
    id: createId(),
    type: 'cpu-benchmark',
    iterations,
    workScale
  }
  const response = await postAndWait<CpuBenchmarkResponse>(worker, request, 60_000)
  return {
    totalMs: response.totalMs,
    durationsMs: response.durationsMs,
    checksum: response.checksum
  }
}

async function probePayloadLimits(worker: Worker, maxPayloadMb: number): Promise<PayloadProbeSummary> {
  const maxBytes = maxPayloadMb * 1024 * 1024
  const mainLimitBytes = probeMainPayloadLimit(maxBytes)
  const workerLimitBytes = await probeWorkerPayloadLimit(worker, maxBytes)
  return {
    maxMainPayloadMb: bytesToMb(mainLimitBytes),
    maxWorkerPayloadMb: bytesToMb(workerLimitBytes)
  }
}

function probeMainPayloadLimit(maxBytes: number): number {
  let size = 1024 * 1024
  let best = 0
  while (size <= maxBytes) {
    try {
      const payload = new Uint8Array(size)
      payload[0] = 1
      payload[payload.length - 1] = 2
      best = size
      size *= 2
    } catch {
      break
    }
  }
  return best
}

async function probeWorkerPayloadLimit(worker: Worker, maxBytes: number): Promise<number> {
  let size = 1024 * 1024
  let best = 0
  while (size <= maxBytes) {
    try {
      const payload = new Uint8Array(size)
      payload[0] = 3
      payload[payload.length - 1] = 7
      const request: PayloadProbeRequest = {
        id: createId(),
        type: 'payload-probe',
        payload: payload.buffer
      }
      await postAndWait<PayloadProbeResponse>(worker, request, 8_000, [request.payload])
      best = size
      size *= 2
    } catch {
      break
    }
  }
  return best
}

function summarize(totalMs: number, durationsMs: number[], iterations: number, checksum: number): CpuBenchmarkSummary {
  const sorted = [...durationsMs].sort((a, b) => a - b)
  const p95Index = Math.max(0, Math.min(sorted.length - 1, Math.floor(sorted.length * 0.95)))
  const p95Ms = sorted[p95Index] ?? 0
  const avgMs = iterations > 0 ? totalMs / iterations : 0
  const opsPerSec = totalMs > 0 ? (iterations * 1000) / totalMs : 0
  return {
    totalMs,
    avgMs,
    p95Ms,
    opsPerSec,
    checksum
  }
}

function computeWork(size: number): number {
  let value = 0x811c9dc5
  for (let i = 0; i < size; i += 1) {
    value ^= (i + 0x9e3779b9) >>> 0
    value = Math.imul(value, 0x01000193) >>> 0
    value = ((value << 7) | (value >>> 25)) >>> 0
  }
  return value >>> 0
}

function bytesToMb(bytes: number): number {
  return Math.round((bytes / (1024 * 1024)) * 100) / 100
}

function createId(): string {
  if (typeof window !== 'undefined' && typeof window.crypto?.randomUUID === 'function') {
    return window.crypto.randomUUID()
  }
  return `bench-${Date.now()}-${Math.floor(Math.random() * 1_000_000)}`
}

function postAndWait<T extends BenchmarkResponse>(
  worker: Worker,
  request: CpuBenchmarkRequest | PayloadProbeRequest,
  timeoutMs: number,
  transfer: Transferable[] = []
): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = window.setTimeout(() => {
      cleanup()
      reject(new Error('worker response timeout'))
    }, timeoutMs)

    const onMessage = (event: MessageEvent<BenchmarkResponse>) => {
      const message = event.data
      if (!message || message.id !== request.id) return
      cleanup()
      resolve(message as T)
    }

    const onError = (event: ErrorEvent) => {
      cleanup()
      reject(new Error(event.message || 'worker failed'))
    }

    const cleanup = () => {
      window.clearTimeout(timer)
      worker.removeEventListener('message', onMessage as EventListener)
      worker.removeEventListener('error', onError as EventListener)
    }

    worker.addEventListener('message', onMessage as EventListener)
    worker.addEventListener('error', onError as EventListener)
    worker.postMessage(request, transfer)
  })
}

function resolveWorkerAssetUrl(path: string): URL {
  if (typeof window !== 'undefined') {
    return new URL(path, window.location.origin)
  }
  return new URL(path, 'http://127.0.0.1')
}
