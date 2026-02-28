// SPDX-License-Identifier: Apache-2.0
import { Show, createMemo, createSignal, onCleanup, onMount } from 'solid-js'
import { Button } from '../../components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/card'
import { PageHero } from '../../components/layout/page-hero'
import { PageShell } from '../../components/layout/page-shell'
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from '../../components/ui/accordion'
import { docsSchedulerApiHref } from '../../lib/docs-links'
import { Input } from '../../components/ui/input'
import { Label } from '../../components/ui/label'
import { startBrowserWorker, defaultBrowserWorkerConfig, type BrowserWorkerController, type BrowserWorkerStats } from '../../lib/browser-worker-runtime'
import { runThreadBenchmark, type ThreadBenchmarkReport } from '../../lib/thread-benchmark'
import { WALLET_SESSION_EVENT, readWalletSession, type WalletSessionState } from '../../lib/wallet-session'

const EMPTY_STATS: BrowserWorkerStats = {
  running: false,
  heartbeats: 0,
  assignmentsSeen: 0,
  resultsSubmitted: 0,
  failuresSubmitted: 0,
  lastError: '',
  lastEvent: 'idle'
}

export default function WorkersPage() {
  const defaults = defaultBrowserWorkerConfig()
  const [wallet, setWallet] = createSignal<WalletSessionState>(readWalletSession())
  const [schedulerUrl, setSchedulerUrl] = createSignal(defaults.schedulerBaseUrl)
  const [workerPubkey, setWorkerPubkey] = createSignal(defaults.workerPubkey)
  const [runtimeId, setRuntimeId] = createSignal(defaults.runtimeIds[0] || '')
  const [workerVersion, setWorkerVersion] = createSignal(defaults.version)
  const [stats, setStats] = createSignal<BrowserWorkerStats>(EMPTY_STATS)
  const [logs, setLogs] = createSignal<string[]>([])
  const [benchmarkIterations, setBenchmarkIterations] = createSignal('24')
  const [benchmarkWorkScale, setBenchmarkWorkScale] = createSignal('240000')
  const [benchmarkMaxPayload, setBenchmarkMaxPayload] = createSignal('32')
  const [benchmarkRunning, setBenchmarkRunning] = createSignal(false)
  const [benchmarkError, setBenchmarkError] = createSignal('')
  const [benchmarkReport, setBenchmarkReport] = createSignal<ThreadBenchmarkReport | null>(null)
  let controller: BrowserWorkerController | null = null

  const running = createMemo(() => stats().running)

  const appendLog = (line: string) => {
    const stamped = `${new Date().toLocaleTimeString('en-US', { hour12: false })} ${line}`
    setLogs((prev) => [stamped, ...prev].slice(0, 120))
  }

  const syncWorkerPubkeyFromWallet = () => {
    const address = wallet().address.trim()
    if (!address) return
    const current = workerPubkey().trim()
    if (!current || current.startsWith('browser-worker-')) {
      setWorkerPubkey(address)
    }
  }

  const stopWorker = () => {
    if (!controller) return
    controller.stop()
    controller = null
    setStats((prev) => ({ ...prev, running: false, lastEvent: 'stopped' }))
    appendLog('worker stopped')
  }

  const runBenchmark = async () => {
    if (benchmarkRunning()) return
    setBenchmarkRunning(true)
    setBenchmarkError('')
    setBenchmarkReport(null)
    try {
      const report = await runThreadBenchmark({
        iterations: parsePositiveInt(benchmarkIterations(), 24),
        workScale: parsePositiveInt(benchmarkWorkScale(), 240000),
        maxPayloadMb: parsePositiveInt(benchmarkMaxPayload(), 32)
      })
      setBenchmarkReport(report)
    } catch (error) {
      setBenchmarkError(error instanceof Error ? error.message : 'benchmark failed')
    } finally {
      setBenchmarkRunning(false)
    }
  }

  const startWorker = () => {
    if (running()) return
    const schedulerBase = schedulerUrl().trim().replace(/\/+$/, '')
    const pubkey = workerPubkey().trim()
    const rid = runtimeId().trim()
    if (!schedulerBase || !pubkey || !rid) {
      setStats((prev) => ({ ...prev, lastError: 'Scheduler URL, worker pubkey, and runtime id are required.' }))
      return
    }
    setStats({ ...EMPTY_STATS, running: true, lastEvent: 'starting' })
    appendLog(`worker starting: scheduler=${schedulerBase} runtime=${rid}`)
    controller = startBrowserWorker(
      {
        schedulerBaseUrl: schedulerBase,
        workerPubkey: pubkey,
        runtimeIds: [rid],
        version: workerVersion().trim() || 'browser-0.1.0',
        capacity: {
          max_concurrent: 1,
          mem_bytes: 268_435_456
        }
      },
      {
        onStats: (next) => setStats(next),
        onLog: appendLog
      }
    )
  }

  onMount(() => {
    syncWorkerPubkeyFromWallet()
    const onWalletSession = (event: Event) => {
      const custom = event as CustomEvent<WalletSessionState>
      const nextWallet = custom.detail || readWalletSession()
      setWallet(nextWallet)
      syncWorkerPubkeyFromWallet()
    }
    window.addEventListener(WALLET_SESSION_EVENT, onWalletSession as EventListener)
    onCleanup(() => {
      window.removeEventListener(WALLET_SESSION_EVENT, onWalletSession as EventListener)
      stopWorker()
    })
  })

  return (
    <PageShell>
      <PageHero
        title="Stake Capital. Execute Jobs. Earn Fees."
        badge="Registration Live"
        description="Run deterministic compute, participate in verification, and earn workload payouts."
        actions={
          <>
            <a href={docsSchedulerApiHref('main')}><Button variant="outline">Read Worker Endpoints</Button></a>
            <a href="/devices/"><Button>Open Fleet Console</Button></a>
          </>
        }
      />

      <section class="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        <h2 class="mb-4 text-2xl font-bold">Live Network Activity</h2>
        <div class="mb-8 grid gap-4 md:grid-cols-4">
          <Card><CardHeader class="pb-2"><p class="text-xs text-muted-foreground">SLOT</p></CardHeader><CardContent><p class="font-mono text-xl" data-chain-field="slot">loading...</p></CardContent></Card>
          <Card><CardHeader class="pb-2"><p class="text-xs text-muted-foreground">TPS</p></CardHeader><CardContent><p class="font-mono text-xl" data-chain-field="tps">loading...</p></CardContent></Card>
          <Card><CardHeader class="pb-2"><p class="text-xs text-muted-foreground">EPOCH</p></CardHeader><CardContent><p class="font-mono text-xl" data-chain-field="epoch">loading...</p></CardContent></Card>
          <Card><CardHeader class="pb-2"><p class="text-xs text-muted-foreground">BLOCK HEIGHT</p></CardHeader><CardContent><p class="font-mono text-xl" data-chain-field="blockHeight">loading...</p></CardContent></Card>
        </div>

        <h2 class="mb-4 text-2xl font-bold">Worker Requirements</h2>
        <Accordion class="mb-2">
          <AccordionItem value="stake">
            <AccordionTrigger value="stake">Stake & Register</AccordionTrigger>
            <AccordionContent value="stake">Workers commit deterministic capacity declarations and expose runtime availability for scheduler selection.</AccordionContent>
          </AccordionItem>
          <AccordionItem value="execute">
            <AccordionTrigger value="execute">Execute & Attest</AccordionTrigger>
            <AccordionContent value="execute">Compute tasks are redundantly executed and cross-checked. Workers submit output + verification metadata.</AccordionContent>
          </AccordionItem>
          <AccordionItem value="settle">
            <AccordionTrigger value="settle">Settle and attest</AccordionTrigger>
            <AccordionContent value="settle">Payouts and penalties follow on-chain settlement paths with deterministic escrow, quorum, and lock constraints.</AccordionContent>
          </AccordionItem>
        </Accordion>

        <h2 class="mb-4 mt-8 text-2xl font-bold">Browser Worker Runtime</h2>
        <Card class="mb-8" data-testid="browser-worker-card">
          <CardHeader>
            <CardTitle>Run Worker In Browser</CardTitle>
            <CardDescription>Starts a browser-hosted worker loop that heartbeats, accepts assignments, executes with `edgerun-runtime-web`, and reports back to scheduler.</CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="grid gap-4 md:grid-cols-2">
              <div class="space-y-2">
                <Label for="browser-worker-scheduler">Scheduler URL</Label>
                <Input
                  id="browser-worker-scheduler"
                  data-testid="browser-worker-scheduler"
                  value={schedulerUrl()}
                  onInput={(event: Event & { currentTarget: HTMLInputElement }) => setSchedulerUrl(event.currentTarget.value)}
                />
              </div>
              <div class="space-y-2">
                <Label for="browser-worker-pubkey">Worker Pubkey</Label>
                <Input
                  id="browser-worker-pubkey"
                  data-testid="browser-worker-pubkey"
                  value={workerPubkey()}
                  onInput={(event: Event & { currentTarget: HTMLInputElement }) => setWorkerPubkey(event.currentTarget.value)}
                />
              </div>
              <div class="space-y-2">
                <Label for="browser-worker-runtime-id">Runtime ID</Label>
                <Input
                  id="browser-worker-runtime-id"
                  data-testid="browser-worker-runtime-id"
                  value={runtimeId()}
                  onInput={(event: Event & { currentTarget: HTMLInputElement }) => setRuntimeId(event.currentTarget.value)}
                />
              </div>
              <div class="space-y-2">
                <Label for="browser-worker-version">Worker Version</Label>
                <Input
                  id="browser-worker-version"
                  data-testid="browser-worker-version"
                  value={workerVersion()}
                  onInput={(event: Event & { currentTarget: HTMLInputElement }) => setWorkerVersion(event.currentTarget.value)}
                />
              </div>
            </div>
            <div class="flex flex-wrap items-center gap-3">
              <Button
                data-testid="browser-worker-start"
                onClick={startWorker}
                disabled={running()}
              >
                Start Browser Worker
              </Button>
              <Button
                variant="outline"
                data-testid="browser-worker-stop"
                onClick={stopWorker}
                disabled={!running()}
              >
                Stop
              </Button>
              <p data-testid="browser-worker-state" class="text-sm text-muted-foreground">
                State: {running() ? 'running' : 'stopped'} · Last event: {stats().lastEvent}
              </p>
            </div>
            <div class="grid gap-3 text-sm md:grid-cols-4">
              <p data-testid="browser-worker-heartbeats">Heartbeats: <span class="font-mono">{stats().heartbeats}</span></p>
              <p data-testid="browser-worker-assignments">Assignments: <span class="font-mono">{stats().assignmentsSeen}</span></p>
              <p data-testid="browser-worker-results">Results: <span class="font-mono">{stats().resultsSubmitted}</span></p>
              <p data-testid="browser-worker-failures">Failures: <span class="font-mono">{stats().failuresSubmitted}</span></p>
            </div>
            <p
              data-testid="browser-worker-error"
              class={`text-sm ${stats().lastError ? 'text-destructive' : 'text-muted-foreground'}`}
            >
              {stats().lastError || 'No runtime errors reported.'}
            </p>
            <div class="rounded-md border border-border bg-muted/20 p-3">
              <p class="mb-2 text-xs uppercase tracking-[0.16em] text-muted-foreground">Worker Log</p>
              <pre
                data-testid="browser-worker-log"
                class="max-h-56 overflow-auto whitespace-pre-wrap break-words font-mono text-xs"
              >{logs().join('\n') || 'No events yet.'}</pre>
            </div>
          </CardContent>
        </Card>

        <h2 class="mb-4 mt-8 text-2xl font-bold">Thread Benchmark</h2>
        <Card class="mb-8" data-testid="thread-benchmark-card">
          <CardHeader>
            <CardTitle>Main vs Worker Throughput</CardTitle>
            <CardDescription>Benchmarks deterministic CPU loops on main thread and dedicated worker thread, then probes message payload transfer limits.</CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="grid gap-4 md:grid-cols-3">
              <div class="space-y-2">
                <Label for="thread-benchmark-iterations">Iterations</Label>
                <Input
                  id="thread-benchmark-iterations"
                  data-testid="thread-benchmark-iterations"
                  value={benchmarkIterations()}
                  onInput={(event: Event & { currentTarget: HTMLInputElement }) => setBenchmarkIterations(event.currentTarget.value)}
                />
              </div>
              <div class="space-y-2">
                <Label for="thread-benchmark-work-scale">Work Scale</Label>
                <Input
                  id="thread-benchmark-work-scale"
                  data-testid="thread-benchmark-work-scale"
                  value={benchmarkWorkScale()}
                  onInput={(event: Event & { currentTarget: HTMLInputElement }) => setBenchmarkWorkScale(event.currentTarget.value)}
                />
              </div>
              <div class="space-y-2">
                <Label for="thread-benchmark-max-payload">Max Payload MB</Label>
                <Input
                  id="thread-benchmark-max-payload"
                  data-testid="thread-benchmark-max-payload"
                  value={benchmarkMaxPayload()}
                  onInput={(event: Event & { currentTarget: HTMLInputElement }) => setBenchmarkMaxPayload(event.currentTarget.value)}
                />
              </div>
            </div>
            <div class="flex flex-wrap items-center gap-3">
              <Button
                data-testid="thread-benchmark-run"
                onClick={() => void runBenchmark()}
                disabled={benchmarkRunning()}
              >
                {benchmarkRunning() ? 'Running Benchmark…' : 'Run Benchmark'}
              </Button>
              <p data-testid="thread-benchmark-status" class="text-sm text-muted-foreground">
                Status: {benchmarkRunning() ? 'running' : benchmarkError() ? 'failed' : benchmarkReport() ? 'completed' : 'idle'}
              </p>
            </div>
            <p
              data-testid="thread-benchmark-error"
              class={`text-sm ${benchmarkError() ? 'text-destructive' : 'text-muted-foreground'}`}
            >
              {benchmarkError() || 'No benchmark errors reported.'}
            </p>
            <Show when={benchmarkReport()}>
              {(report) => (
              <div class="space-y-3 rounded-md border border-border bg-muted/20 p-3">
                <p class="text-xs uppercase tracking-[0.16em] text-muted-foreground">Benchmark Summary</p>
                <div class="grid gap-3 text-sm md:grid-cols-3">
                  <p data-testid="thread-benchmark-main-total">Main total: <span class="font-mono">{formatMs(report().main.totalMs)} ms</span></p>
                  <p data-testid="thread-benchmark-worker-total">Worker total: <span class="font-mono">{formatMs(report().worker.totalMs)} ms</span></p>
                  <p data-testid="thread-benchmark-speedup">Speedup: <span class="font-mono">{formatRatio(report().speedup)}x</span></p>
                  <p data-testid="thread-benchmark-main-ops">Main ops/sec: <span class="font-mono">{formatOps(report().main.opsPerSec)}</span></p>
                  <p data-testid="thread-benchmark-worker-ops">Worker ops/sec: <span class="font-mono">{formatOps(report().worker.opsPerSec)}</span></p>
                  <p data-testid="thread-benchmark-p95">P95 (main/worker): <span class="font-mono">{formatMs(report().main.p95Ms)} / {formatMs(report().worker.p95Ms)} ms</span></p>
                  <p data-testid="thread-benchmark-main-payload">Main payload cap: <span class="font-mono">{report().payload.maxMainPayloadMb} MB</span></p>
                  <p data-testid="thread-benchmark-worker-payload">Worker payload cap: <span class="font-mono">{report().payload.maxWorkerPayloadMb} MB</span></p>
                  <p data-testid="thread-benchmark-checksum">Checksums: <span class="font-mono">{report().main.checksum} / {report().worker.checksum}</span></p>
                </div>
              </div>
              )}
            </Show>
          </CardContent>
        </Card>

        <h2 class="mb-4 mt-8 text-2xl font-bold">Quick Start</h2>
        <div class="mb-8 grid gap-4 md:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle>Install Worker Daemon</CardTitle>
              <CardDescription>Get a node online with deterministic runtime support.</CardDescription>
            </CardHeader>
            <CardContent>
              <p class="font-mono text-sm text-muted-foreground">$ edgerun-worker init --cluster devnet</p>
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>Register Worker</CardTitle>
              <CardDescription>Stake and register node metadata for scheduler eligibility.</CardDescription>
            </CardHeader>
            <CardContent>
              <p class="font-mono text-sm text-muted-foreground">$ edgerun-worker register --capacity 50 --control $EDGERUN_ROUTE_CONTROL_BASE</p>
            </CardContent>
          </Card>
        </div>

        <h2 class="mb-4 text-2xl font-bold">Top Workers</h2>
        <div class="grid gap-4 md:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle>Leaderboard Source</CardTitle>
              <CardDescription>Ranking is exposed from scheduler telemetry and should not be synthesized in this client.</CardDescription>
            </CardHeader>
            <CardContent>
              <p class="text-sm text-muted-foreground">
                Live ranking is unavailable when scheduler telemetry is offline. Use `/devices/` for current fleet state and queue health.
              </p>
            </CardContent>
          </Card>
        </div>
      </section>
    </PageShell>
  )
}

function parsePositiveInt(value: string, fallback: number): number {
  const parsed = Number.parseInt(value, 10)
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback
}

function formatMs(value: number): string {
  return Number.isFinite(value) ? value.toFixed(2) : '0.00'
}

function formatRatio(value: number): string {
  return Number.isFinite(value) ? value.toFixed(2) : '0.00'
}

function formatOps(value: number): string {
  return Number.isFinite(value) ? value.toFixed(1) : '0.0'
}
