import { SchedulerControlWsClient } from './scheduler-control-ws'

type WorkerCapacity = {
  max_concurrent: number
  mem_bytes: number
}

type BrowserWorkerConfig = {
  schedulerBaseUrl: string
  workerPubkey: string
  runtimeIds: string[]
  version: string
  capacity: WorkerCapacity
}

type BrowserWorkerStats = {
  running: boolean
  heartbeats: number
  assignmentsSeen: number
  resultsSubmitted: number
  failuresSubmitted: number
  lastError: string
  lastEvent: string
}

type HeartbeatResponse = {
  ok?: boolean
  next_poll_ms?: number
}

type Limits = {
  max_memory_bytes: number
  max_instructions: number
}

type QueuedAssignment = {
  job_id: string
  bundle_hash: string
  bundle_url: string
  runtime_id: string
  abi_version?: number
  limits: Limits
}

type AssignmentsResponse = {
  jobs?: QueuedAssignment[]
}

type BundleGetResponse = {
  ok?: boolean
  bundle_hash?: string
  payload_b64?: string
}

type RuntimeDigest = {
  bundle_hash: string
  abi_version: number
  runtime_id: string
  output_hash: string
  output_len: number
  input_len: number
  max_memory_bytes: number
  max_instructions: number
  fuel_limit: number
  fuel_remaining: number
}

type RuntimeModule = {
  default: (moduleOrPath?: string | URL | Request) => Promise<unknown>
  execute_bundle_payload_bytes_for_runtime_and_abi_digest_strict: (
    bundlePayload: Uint8Array,
    expectedRuntimeIdHex: string,
    expectedAbiVersion: number
  ) => RuntimeDigest
}

type MainToWorkerMessage =
  | { type: 'start'; config: BrowserWorkerConfig }
  | { type: 'stop' }

type WorkerToMainMessage =
  | { type: 'stats'; stats: BrowserWorkerStats }
  | { type: 'log'; line: string }

let stopped = true
let loopPromise: Promise<void> | null = null
let runtimePromise: Promise<RuntimeModule> | null = null
let controlClient: SchedulerControlWsClient | null = null
let stats: BrowserWorkerStats = {
  running: false,
  heartbeats: 0,
  assignmentsSeen: 0,
  resultsSubmitted: 0,
  failuresSubmitted: 0,
  lastError: '',
  lastEvent: 'idle'
}

self.onmessage = (event: MessageEvent<MainToWorkerMessage>) => {
  void handleMainMessage(event.data)
}

async function handleMainMessage(message: MainToWorkerMessage): Promise<void> {
  if (message.type === 'stop') {
    stopped = true
    controlClient?.close()
    controlClient = null
    stats.running = false
    stats.lastEvent = 'stopped'
    emitStats()
    return
  }
  if (message.type !== 'start') return
  if (!stopped || loopPromise) return

  const config = normalizeConfig(message.config)
  stopped = false
  stats = {
    running: true,
    heartbeats: 0,
    assignmentsSeen: 0,
    resultsSubmitted: 0,
    failuresSubmitted: 0,
    lastError: '',
    lastEvent: 'starting'
  }
  emitStats()
  emitLog(`browser worker started: ${config.workerPubkey}`)

  loopPromise = runLoop(config).finally(() => {
    loopPromise = null
    if (!stopped) stopped = true
    stats.running = false
    if (stats.lastEvent !== 'stopped-with-error') stats.lastEvent = 'stopped'
    emitStats()
  })
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

async function runLoop(config: BrowserWorkerConfig): Promise<void> {
  controlClient ??= new SchedulerControlWsClient(config.schedulerBaseUrl, `browser-worker-${config.workerPubkey}`)
  while (!stopped) {
    try {
      const pollIntervalMs = await heartbeat(config, controlClient)
      if (stopped) return
      await processAssignments(config, controlClient)
      if (stopped) return
      await delay(Math.max(300, pollIntervalMs))
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      stats.lastError = message
      stats.lastEvent = 'loop-error'
      emitStats()
      emitLog(`worker loop error: ${message}`)
      await delay(1500)
    }
  }
}

async function heartbeat(config: BrowserWorkerConfig, control: SchedulerControlWsClient): Promise<number> {
  stats.lastEvent = 'heartbeat'
  emitStats()
  const payload = await control.request<HeartbeatResponse>('worker.heartbeat', {
    worker_pubkey: config.workerPubkey,
    runtime_ids: config.runtimeIds,
    version: config.version,
    capacity: config.capacity
  })
  stats.heartbeats += 1
  stats.lastError = ''
  stats.lastEvent = 'heartbeat-ok'
  emitStats()
  emitLog(`heartbeat ok (#${stats.heartbeats})`)
  return Number(payload.next_poll_ms || 2000)
}

async function processAssignments(config: BrowserWorkerConfig, control: SchedulerControlWsClient): Promise<void> {
  stats.lastEvent = 'poll-assignments'
  emitStats()
  const payload = await control.request<AssignmentsResponse>('worker.assignments', {
    worker_pubkey: config.workerPubkey
  })
  const jobs = Array.isArray(payload.jobs) ? payload.jobs : []
  if (jobs.length === 0) {
    stats.lastEvent = 'idle'
    emitStats()
    return
  }

  stats.assignmentsSeen += jobs.length
  stats.lastEvent = `processing-${jobs.length}`
  emitStats()
  emitLog(`received ${jobs.length} assignment(s)`)
  for (const assignment of jobs) {
    if (stopped) return
    await processSingleAssignment(config, control, assignment)
  }
}

async function processSingleAssignment(
  config: BrowserWorkerConfig,
  control: SchedulerControlWsClient,
  assignment: QueuedAssignment
): Promise<void> {
  const idempotencyKey = createIdempotencyKey()
  try {
    const bundleResponse = await control.request<BundleGetResponse>('bundle.get', {
      bundle_hash: assignment.bundle_hash
    })
    const payloadB64 = String(bundleResponse.payload_b64 || '').trim()
    if (!payloadB64) {
      throw new Error(`bundle fetch failed for ${assignment.bundle_hash}`)
    }
    const bundlePayload = decodeBase64(payloadB64)
    const runtime = await loadRuntimeWeb()
    const digest = runtime.execute_bundle_payload_bytes_for_runtime_and_abi_digest_strict(
      bundlePayload,
      assignment.runtime_id,
      Number(assignment.abi_version ?? 1)
    )

    await control.request('worker.result', {
      idempotency_key: idempotencyKey,
      worker_pubkey: config.workerPubkey,
      job_id: assignment.job_id,
      bundle_hash: assignment.bundle_hash,
      output_hash: digest.output_hash,
      output_len: digest.output_len
    })

    await control.request('worker.replay', {
      idempotency_key: `${idempotencyKey}-replay`,
      worker_pubkey: config.workerPubkey,
      job_id: assignment.job_id,
      artifact: {
        bundle_hash: assignment.bundle_hash,
        ok: true,
        abi_version: digest.abi_version,
        runtime_id: digest.runtime_id,
        output_hash: digest.output_hash,
        output_len: digest.output_len,
        input_len: digest.input_len,
        max_memory_bytes: digest.max_memory_bytes,
        max_instructions: digest.max_instructions,
        fuel_limit: digest.fuel_limit,
        fuel_remaining: digest.fuel_remaining
      }
    })

    stats.resultsSubmitted += 1
    stats.lastError = ''
    emitStats()
    emitLog(`result submitted: job=${assignment.job_id}`)
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err)
    stats.lastError = message
    stats.failuresSubmitted += 1
    emitStats()
    emitLog(`assignment failed: job=${assignment.job_id} error=${message}`)
    await control.request('worker.failure', {
      idempotency_key: `${idempotencyKey}-failure`,
      worker_pubkey: config.workerPubkey,
      job_id: assignment.job_id,
      bundle_hash: assignment.bundle_hash,
      phase: 'runtime_execute',
      error_code: 'BrowserRuntimeError',
      error_message: message
    }).catch(() => {
      // best effort failure report
    })
  }
}

async function loadRuntimeWeb(): Promise<RuntimeModule> {
  runtimePromise ??= (async () => {
    const modulePath = '/wasm/edgerun-runtime-web/edgerun_runtime_web.js'
    const mod = await import(modulePath)
    const runtimeMod = mod as unknown as RuntimeModule
    await runtimeMod.default('/wasm/edgerun-runtime-web/edgerun_runtime_web_bg.wasm')
    return runtimeMod
  })()
  return runtimePromise
}

function emitStats(): void {
  const message: WorkerToMainMessage = {
    type: 'stats',
    stats: { ...stats }
  }
  self.postMessage(message)
}

function emitLog(line: string): void {
  const message: WorkerToMainMessage = {
    type: 'log',
    line
  }
  self.postMessage(message)
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function createIdempotencyKey(): string {
  if (typeof self.crypto?.randomUUID === 'function') {
    return self.crypto.randomUUID()
  }
  return `browser-${Date.now()}-${Math.floor(Math.random() * 1_000_000)}`
}

function decodeBase64(payload: string): Uint8Array {
  const binary = atob(payload)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i)
  }
  return bytes
}
