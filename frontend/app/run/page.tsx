// SPDX-License-Identifier: Apache-2.0
import { For, createEffect, createMemo, createSignal, onCleanup } from 'solid-js'
import { Button } from '../../components/ui/button'
import { Input } from '../../components/ui/input'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/card'
import { GeneratingIndicator } from '../../components/ui/generating-indicator'
import { PageHero } from '../../components/layout/page-hero'
import { PageShell } from '../../components/layout/page-shell'
import { Label } from '../../components/ui/label'
import { Select } from '../../components/ui/select'
import { Textarea } from '../../components/ui/textarea'
import { Checkbox } from '../../components/ui/checkbox'
import { Alert, AlertDescription, AlertTitle } from '../../components/ui/alert'
import { Separator } from '../../components/ui/separator'
import { Table, TableBody, TableCell, TableRow } from '../../components/ui/table'
import { Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '../../components/ui/dialog'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../../components/ui/tabs'
import { clearJobTabStatus, publishJobTabStatus } from '../../lib/tab-job-status'

type PresetApp = {
  id: string
  name: string
  tagline: string
  inputLabel: string
  outputLabel: string
  outcome: string
  defaultJobName: string
  defaultRuntimeId: string
  defaultInputJson: string
  benchmarkHint: string
}

type SubmissionStatus = 'idle' | 'pending' | 'success' | 'error'
type RunStep = 'step-1' | 'step-2' | 'step-3'

type VanitySearchSpace = {
  prefix: string
  startCounter: string
  endCounter: string
  chunkAttempts: string
  workerCount: string
}

const PRESET_APPS: PresetApp[] = [
  {
    id: 'vanity-generator',
    name: 'Solana Vanity Address Generator',
    tagline: 'Best onboarding path: interactive and tangible output with predictable compute profile.',
    inputLabel: 'Prefix and counter range (+ optional seed handling mode).',
    outputLabel: 'Matching address, keypair artifact, and deterministic execution proof envelope.',
    outcome: 'Workers execute deterministic counter chunks; scheduler returns consensus output and settlement evidence.',
    defaultJobName: 'solana-vanity-address-search',
    defaultRuntimeId: '0000000000000000000000000000000000000000000000000000000000000000',
    defaultInputJson: '{\n  "prefix": "So1",\n  "startCounter": 0,\n  "endCounter": 1000000,\n  "chunkAttempts": 50000\n}',
    benchmarkHint: 'Use this as the canonical benchmark to normalize compute/fee multipliers across workers.'
  },
  {
    id: 'json-transform',
    name: 'JSON Transform Module',
    tagline: 'Useful for schema-safe ETL style payload validation on worker fleets.',
    inputLabel: 'Input JSON blob and transform configuration.',
    outputLabel: 'Transformed JSON document plus deterministic hash.',
    outcome: 'Each worker applies the same transform rules and returns attestable output hashes.',
    defaultJobName: 'json-transform-check',
    defaultRuntimeId: '1111111111111111111111111111111111111111111111111111111111111111',
    defaultInputJson: '{\n  "document": {"status": "pending"},\n  "rules": [{"op": "set", "path": "status", "value": "ready"}]\n}',
    benchmarkHint: 'Good secondary benchmark for IO-light, branch-heavy workloads.'
  },
  {
    id: 'text-scoring',
    name: 'Text Scoring Module',
    tagline: 'Demonstrates weighted scoring with deterministic output ordering.',
    inputLabel: 'Text payload and scorer configuration.',
    outputLabel: 'Scored records sorted by deterministic tie-break policy.',
    outcome: 'Workers compute identical scores; scheduler verifies sorted output consistency.',
    defaultJobName: 'text-score-pass',
    defaultRuntimeId: '2222222222222222222222222222222222222222222222222222222222222222',
    defaultInputJson: '{\n  "records": ["alpha", "beta", "gamma"],\n  "weights": {"length": 1.0, "entropy": 0.4}\n}',
    benchmarkHint: 'Useful for compute normalization under short iterative loops.'
  }
]

const DEFAULT_PRESET = PRESET_APPS[0]!
const DEFAULT_VANITY_SEARCH_SPACE: VanitySearchSpace = {
  prefix: 'So1',
  startCounter: '0',
  endCounter: '1000000',
  chunkAttempts: '50000',
  workerCount: '5'
}

function defaultSchedulerUrl(): string {
  if (typeof window === 'undefined') return 'http://127.0.0.1:8090'
  const injected = String((window as any).__EDGERUN_API_BASE || '').trim()
  if (injected) return injected
  const host = window.location.hostname
  if (host === '127.0.0.1' || host === 'localhost') return 'http://127.0.0.1:8090'
  return 'https://api.edgerun.tech'
}

export default function RunPage() {
  const [activeStep, setActiveStep] = createSignal<RunStep>('step-1')
  const [safetyOpen, setSafetyOpen] = createSignal(false)
  const [submissionMode, setSubmissionMode] = createSignal<'preset' | 'custom'>('preset')
  const [selectedPresetId, setSelectedPresetId] = createSignal(DEFAULT_PRESET.id)
  const [inputMode, setInputMode] = createSignal<'predefined' | 'json' | 'file'>('predefined')
  const [executionMode, setExecutionMode] = createSignal<'secure-local' | 'distributed-insecure'>('distributed-insecure')
  const [jobName, setJobName] = createSignal(DEFAULT_PRESET.defaultJobName)
  const [runtimeId, setRuntimeId] = createSignal(DEFAULT_PRESET.defaultRuntimeId)
  const [inputJson, setInputJson] = createSignal(DEFAULT_PRESET.defaultInputJson)
  const [schedulerUrl, setSchedulerUrl] = createSignal(defaultSchedulerUrl())
  const [customModuleName, setCustomModuleName] = createSignal('')
  const [customWasmFileName, setCustomWasmFileName] = createSignal('')
  const [inputFileName, setInputFileName] = createSignal('')
  const [vanitySearchSpace, setVanitySearchSpace] = createSignal<VanitySearchSpace>(DEFAULT_VANITY_SEARCH_SPACE)
  const [allowSeedExposure, setAllowSeedExposure] = createSignal(true)
  const [submitStatus, setSubmitStatus] = createSignal<SubmissionStatus>('idle')
  const [submitMessage, setSubmitMessage] = createSignal('')
  const [validationErrors, setValidationErrors] = createSignal<string[]>([])
  const [lastReceiptId, setLastReceiptId] = createSignal('')

  createEffect(() => {
    const status = submitStatus()
    if (status === 'pending') {
      const workersActive = Number(vanitySearchSpace().workerCount) || undefined
      let progress = 9
      const timer = window.setInterval(() => {
        progress = progress >= 92 ? 92 : progress + 7
        publishJobTabStatus({
          phase: 'running',
          progressPercent: progress,
          workersActive
        })
      }, 500)
      publishJobTabStatus({
        phase: 'running',
        progressPercent: progress,
        workersActive
      })
      onCleanup(() => window.clearInterval(timer))
      return
    }
    if (status === 'success') {
      publishJobTabStatus({
        phase: 'settled',
        flashIfHidden: true
      })
      return
    }
    if (status === 'error') {
      publishJobTabStatus({
        phase: 'error',
        flashIfHidden: true
      })
      return
    }
    clearJobTabStatus()
  })

  onCleanup(() => {
    clearJobTabStatus()
  })

  const selectedPreset = createMemo<PresetApp>(() => PRESET_APPS.find((app) => app.id === selectedPresetId()) ?? DEFAULT_PRESET)
  const isVanityApp = createMemo(() => selectedPresetId() === 'vanity-generator')
  const appInputSummary = createMemo(() => {
    if (isVanityApp()) {
      const spec = vanitySearchSpace()
      return `Vanity search space: prefix "${spec.prefix}" from counter ${spec.startCounter} to ${spec.endCounter} with chunk size ${spec.chunkAttempts} across ${spec.workerCount} workers.`
    }
    if (inputMode() === 'json') {
      return 'App input comes from raw JSON payload.'
    }
    if (inputMode() === 'file') {
      return `App input comes from uploaded file${inputFileName() ? ` (${inputFileName()})` : ''}.`
    }
    return 'Use app-specific JSON payload for this module.'
  })

  const onPresetChange = (nextPresetId: string) => {
    const preset = PRESET_APPS.find((app) => app.id === nextPresetId) ?? DEFAULT_PRESET
    setSelectedPresetId(nextPresetId)
    setJobName(preset.defaultJobName)
    setRuntimeId(preset.defaultRuntimeId)
    setInputJson(preset.defaultInputJson)
    setInputMode(nextPresetId === 'vanity-generator' ? 'predefined' : 'json')
    setSubmitStatus('idle')
    setValidationErrors([])
  }

  const applyRecommendedDemo = () => {
    onPresetChange('vanity-generator')
    setSubmissionMode('preset')
    setExecutionMode('distributed-insecure')
    setSchedulerUrl('http://127.0.0.1:8090')
    setAllowSeedExposure(true)
    setActiveStep('step-2')
  }

  const isKnownLocalScheduler = (urlText: string) => {
    try {
      const parsed = new URL(urlText)
      const host = parsed.host
      return host === '127.0.0.1:8090' || host === 'localhost:8090' || host === 'api.edgerun.tech'
    } catch {
      return false
    }
  }

  const validateBeforeSubmit = () => {
    const errors: string[] = []

    if (jobName().trim().length < 3) {
      errors.push('Job name must be at least 3 characters.')
    }

    if (!/^[0-9a-fA-F]{64}$/.test(runtimeId().trim())) {
      errors.push('Runtime ID must be a 64-character hex string.')
    }

    try {
      new URL(schedulerUrl().trim())
    } catch {
      errors.push('Scheduler URL must be a valid URL.')
    }

    if (submissionMode() === 'custom') {
      if (!customModuleName().trim()) {
        errors.push('Custom module name is required when using Upload Custom Module.')
      }
      if (!customWasmFileName()) {
        errors.push('A WASM module file is required for custom submissions.')
      }
    }

    if (inputMode() === 'json') {
      try {
        JSON.parse(inputJson())
      } catch {
        errors.push('Input JSON is invalid. Fix JSON syntax before submitting.')
      }
    }

    if (inputMode() === 'file' && !inputFileName()) {
      errors.push('Input file is required when Input Source is set to Upload input file.')
    }

    if (isVanityApp() && inputMode() === 'predefined') {
      const start = Number(vanitySearchSpace().startCounter)
      const end = Number(vanitySearchSpace().endCounter)
      const chunk = Number(vanitySearchSpace().chunkAttempts)
      const workers = Number(vanitySearchSpace().workerCount)
      if (!Number.isFinite(start) || !Number.isFinite(end) || end <= start) {
        errors.push('Vanity search space must use numeric counters and End Counter must be greater than Start Counter.')
      }
      if (!Number.isFinite(chunk) || chunk <= 0) {
        errors.push('Vanity search space chunk attempts must be greater than 0.')
      }
      if (!Number.isFinite(workers) || workers < 1) {
        errors.push('Vanity worker count must be at least 1.')
      }
      if (!vanitySearchSpace().prefix.trim()) {
        errors.push('Vanity prefix is required.')
      }
    }

    if (executionMode() === 'distributed-insecure' && !allowSeedExposure()) {
      errors.push('Distributed mode requires explicit worker seed exposure acknowledgement.')
    }

    return errors
  }

  const handleSubmit = async () => {
    setSubmitStatus('idle')
    setSubmitMessage('')
    setValidationErrors([])

    const errors = validateBeforeSubmit()
    if (errors.length > 0) {
      setValidationErrors(errors)
      setSubmitStatus('error')
      setSubmitMessage('Submission blocked. Resolve the highlighted contract issues and retry.')
      return
    }

    setSubmitStatus('pending')
    setSubmitMessage('Submitting job envelope to scheduler...')

    await new Promise((resolve) => setTimeout(resolve, 650))

    if (!isKnownLocalScheduler(schedulerUrl().trim())) {
      setSubmitStatus('error')
      setSubmitMessage(`Scheduler unreachable at ${schedulerUrl().trim()}. Use https://api.edgerun.tech or start local scheduler on 127.0.0.1:8090.`)
      return
    }

    const receiptId = `demo-${Date.now().toString(36)}`
    setLastReceiptId(receiptId)
    setSubmitStatus('success')
    setSubmitMessage('Job accepted. Track receipt and move to /job/:id for execution status.')
  }

  return (
    <PageShell>
      <PageHero
        title="Run Job"
        badge="Guided Flow"
        description="Understand value, guarantees, and execution in one guided flow, then run with recommended defaults or your own app module."
        actions={
          <>
            <Button onClick={applyRecommendedDemo}>Use Recommended Demo</Button>
            <a href="/docs/getting-started/quick-start/"><Button variant="outline">Open Get Started Guide</Button></a>
            <Button variant="outline" disabled>
              Live Submission
              <GeneratingIndicator class="ml-2 text-[10px]" />
            </Button>
          </>
        }
      />

      <section class="mx-auto max-w-7xl space-y-6 px-4 py-8 sm:px-6 lg:px-8">
        <Alert>
          <AlertTitle>Vanity Generator = Onboarding + Benchmark Baseline</AlertTitle>
          <AlertDescription>
            Start users with vanity address generation for an interactive first run and reuse its runtime profile to normalize compute calculations later.
          </AlertDescription>
        </Alert>

        <div class="grid gap-4 md:grid-cols-3" data-testid="journey-overview">
          <Card>
            <CardHeader>
              <CardTitle>Why This Helps You</CardTitle>
              <CardDescription>Run distributed compute and still get deterministic output you can trust.</CardDescription>
            </CardHeader>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>Guarantees You Get</CardTitle>
              <CardDescription>Deterministic execution, cross-worker consistency checks, and auditable submission receipts.</CardDescription>
            </CardHeader>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>How To Use It</CardTitle>
              <CardDescription>Choose app, configure platform + app fields, review exact IO contract, submit.</CardDescription>
            </CardHeader>
          </Card>
        </div>

        <Tabs value={activeStep()} onValueChange={(next) => setActiveStep(next as RunStep)} class="space-y-4">
          <TabsList class="w-full justify-start gap-1 overflow-auto">
            <TabsTrigger value="step-1">1. Choose Module</TabsTrigger>
            <TabsTrigger value="step-2">2. Define Inputs</TabsTrigger>
            <TabsTrigger value="step-3">3. Review + Run</TabsTrigger>
          </TabsList>

          <TabsContent value="step-1" class="space-y-4" data-testid="run-step-choose">
            <div class="grid gap-4 lg:grid-cols-3">
              <Card class="lg:col-span-2">
                <CardHeader>
                  <CardTitle>Module Source</CardTitle>
                  <CardDescription>Select a curated app for fast onboarding, or upload your own WASM module.</CardDescription>
                </CardHeader>
                <CardContent class="space-y-4">
                  <Alert>
                    <AlertTitle>Frictionless Start</AlertTitle>
                    <AlertDescription>Pick the recommended vanity demo to see end-to-end output first. Change advanced fields only when needed.</AlertDescription>
                  </Alert>
                  <div class="space-y-1">
                    <Label for="submission-mode">Submission Mode</Label>
                    <Select
                      id="submission-mode"
                      aria-label="Submission Mode"
                      value={submissionMode()}
                      onInput={(event: Event & { currentTarget: HTMLSelectElement }) => setSubmissionMode(event.currentTarget.value as 'preset' | 'custom')}
                    >
                      <option value="preset">Preset App</option>
                      <option value="custom">Upload Custom Module</option>
                    </Select>
                  </div>

                  <div classList={{ hidden: submissionMode() !== 'preset' }} class="space-y-3" data-testid="preset-mode-panel">
                    <div class="space-y-1">
                      <Label for="preset-app">Preset App</Label>
                      <Select
                        id="preset-app"
                        aria-label="Preset App"
                        value={selectedPresetId()}
                        onInput={(event: Event & { currentTarget: HTMLSelectElement }) => onPresetChange(event.currentTarget.value)}
                      >
                        <For each={PRESET_APPS}>{(app) => (
                          <option value={app.id}>{app.name}</option>
                        )}</For>
                      </Select>
                    </div>
                    <Alert>
                      <AlertTitle>{selectedPreset().name}</AlertTitle>
                      <AlertDescription>{selectedPreset().tagline}</AlertDescription>
                    </Alert>
                  </div>

                  <div classList={{ hidden: submissionMode() !== 'custom' }} class="space-y-3" data-testid="custom-mode-panel">
                    <div class="space-y-1">
                      <Label for="custom-module-name">Custom Module Name</Label>
                      <Input id="custom-module-name" aria-label="Custom Module Name" placeholder="my-runtime-module" value={customModuleName()} onInput={(event: Event & { currentTarget: HTMLInputElement }) => setCustomModuleName(event.currentTarget.value)} />
                    </div>
                    <div class="space-y-1">
                      <Label for="custom-wasm">WASM Module</Label>
                      <Input id="custom-wasm" aria-label="Custom WASM Module" type="file" accept=".wasm" onChange={(event: Event & { currentTarget: HTMLInputElement }) => setCustomWasmFileName(event.currentTarget.files?.[0]?.name ?? '')} />
                    </div>
                    <p class="text-xs text-muted-foreground">Upload a deterministic WASM artifact and define expected output shape in step 3 before submitting.</p>
                  </div>

                  <div class="flex justify-end">
                    <Button type="button" onClick={() => setActiveStep('step-2')}>Continue to Configure App</Button>
                  </div>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle>What Happens</CardTitle>
                  <CardDescription>Execution chain that users can reason about.</CardDescription>
                </CardHeader>
                <CardContent class="space-y-2 text-sm">
                  <p><strong class="text-foreground">1.</strong> Scheduler validates runtime + policy.</p>
                  <p><strong class="text-foreground">2.</strong> Workers execute deterministic tasks.</p>
                  <p><strong class="text-foreground">3.</strong> Consensus output and settlement proof are returned.</p>
                  <Separator />
                  <p class="text-xs text-muted-foreground">No hidden steps: each run exposes inputs, target runtime, and expected output class before submission.</p>
                </CardContent>
              </Card>
            </div>
          </TabsContent>

          <TabsContent value="step-2" class="space-y-4" data-testid="run-step-inputs">
            <div class="grid gap-4 lg:grid-cols-3">
              <Card class="lg:col-span-2">
                <CardHeader>
                  <CardTitle>Configure Platform + App Fields</CardTitle>
                  <CardDescription>Platform envelope is separated from app-specific fields so users can configure a single app without guessing.</CardDescription>
                </CardHeader>
                <CardContent class="space-y-4">
                  <div class="space-y-1">
                    <h4 class="text-sm font-semibold">Platform Job Envelope</h4>
                    <p class="text-xs text-muted-foreground">These values apply to scheduling and execution regardless of app.</p>
                  </div>
                  <div class="grid gap-4 md:grid-cols-2">
                    <div class="space-y-1">
                      <Label for="job-name">Job Name</Label>
                      <Input id="job-name" aria-label="Job Name" value={jobName()} onInput={(event: Event & { currentTarget: HTMLInputElement }) => setJobName(event.currentTarget.value)} />
                    </div>
                    <div class="space-y-1">
                      <Label for="runtime-id">Runtime ID (hex)</Label>
                      <Input id="runtime-id" aria-label="Runtime ID" class="font-mono text-xs" value={runtimeId()} onInput={(event: Event & { currentTarget: HTMLInputElement }) => setRuntimeId(event.currentTarget.value)} />
                    </div>
                  </div>

                  <div class="space-y-1">
                    <Label for="execution-mode">Execution Mode</Label>
                    <Select
                      id="execution-mode"
                      aria-label="Execution Mode"
                      value={executionMode()}
                      onInput={(event: Event & { currentTarget: HTMLSelectElement }) => setExecutionMode(event.currentTarget.value as 'secure-local' | 'distributed-insecure')}
                    >
                      <option value="secure-local">secure-local</option>
                      <option value="distributed-insecure">distributed-insecure</option>
                    </Select>
                  </div>

                  <div class="space-y-1">
                    <Label for="scheduler-url">Scheduler URL</Label>
                    <Input id="scheduler-url" aria-label="Scheduler URL" value={schedulerUrl()} class="font-mono text-xs" onInput={(event: Event & { currentTarget: HTMLInputElement }) => setSchedulerUrl(event.currentTarget.value)} />
                  </div>

                  <div class="flex items-center gap-2">
                    <Checkbox id="allow-seed-exposure" aria-label="Allow worker seed exposure" checked={allowSeedExposure()} onInput={(event: Event & { currentTarget: HTMLInputElement }) => setAllowSeedExposure(event.currentTarget.checked)} />
                    <Label for="allow-seed-exposure" class="text-xs text-muted-foreground">Allow worker seed exposure (required for distributed-insecure).</Label>
                  </div>

                  <div class="flex justify-end">
                    <Button type="button" variant="outline" size="sm" onClick={() => setSafetyOpen(true)}>
                      Mode Safety
                    </Button>
                  </div>

                  <Separator />

                  <div class="space-y-1">
                    <h4 class="text-sm font-semibold">App Configuration: {selectedPreset().name}</h4>
                    <p class="text-xs text-muted-foreground">These fields define what this specific app will execute.</p>
                  </div>

                  <div class="space-y-1">
                    <Label for="input-mode">App Input Source</Label>
                    <Select
                      id="input-mode"
                      aria-label="App Input Source"
                      value={inputMode()}
                      onInput={(event: Event & { currentTarget: HTMLSelectElement }) => setInputMode(event.currentTarget.value as 'predefined' | 'json' | 'file')}
                    >
                      <option value="predefined">Predefined fields</option>
                      <option value="json">Raw JSON payload</option>
                      <option value="file">Upload input file</option>
                    </Select>
                  </div>

                  <div classList={{ hidden: inputMode() !== 'predefined' }} class="space-y-3" data-testid="predefined-input-panel">
                    <div hidden={!isVanityApp()} class="space-y-3" data-testid="vanity-app-fields">
                      <p class="text-xs text-muted-foreground">Search space is a Vanity app field. Configure it here when running the vanity generator.</p>
                      <div class="grid gap-4 md:grid-cols-3">
                        <div class="space-y-1">
                          <Label for="prefix">Prefix</Label>
                          <Input id="prefix" aria-label="Prefix" value={vanitySearchSpace().prefix} class="font-mono text-xs" onInput={(event: Event & { currentTarget: HTMLInputElement }) => setVanitySearchSpace({ ...vanitySearchSpace(), prefix: event.currentTarget.value })} />
                        </div>
                        <div class="space-y-1">
                          <Label for="start-counter">Start Counter</Label>
                          <Input id="start-counter" aria-label="Start Counter" type="number" min="0" value={vanitySearchSpace().startCounter} class="font-mono text-xs" onInput={(event: Event & { currentTarget: HTMLInputElement }) => setVanitySearchSpace({ ...vanitySearchSpace(), startCounter: event.currentTarget.value })} />
                        </div>
                        <div class="space-y-1">
                          <Label for="end-counter">End Counter</Label>
                          <Input id="end-counter" aria-label="End Counter" type="number" min="1" value={vanitySearchSpace().endCounter} class="font-mono text-xs" onInput={(event: Event & { currentTarget: HTMLInputElement }) => setVanitySearchSpace({ ...vanitySearchSpace(), endCounter: event.currentTarget.value })} />
                        </div>
                      </div>
                      <div class="grid gap-4 md:grid-cols-2">
                        <div class="space-y-1">
                          <Label for="chunk-attempts">Chunk Attempts</Label>
                          <Input id="chunk-attempts" aria-label="Chunk Attempts" type="number" min="1" value={vanitySearchSpace().chunkAttempts} class="font-mono text-xs" onInput={(event: Event & { currentTarget: HTMLInputElement }) => setVanitySearchSpace({ ...vanitySearchSpace(), chunkAttempts: event.currentTarget.value })} />
                        </div>
                        <div class="space-y-1">
                          <Label for="worker-count">Worker Count</Label>
                          <Input id="worker-count" aria-label="Worker Count" type="number" min="1" value={vanitySearchSpace().workerCount} onInput={(event: Event & { currentTarget: HTMLInputElement }) => setVanitySearchSpace({ ...vanitySearchSpace(), workerCount: event.currentTarget.value })} />
                        </div>
                      </div>
                    </div>
                    <Alert hidden={isVanityApp()}>
                      <AlertTitle>No Predefined Search Space For This App</AlertTitle>
                      <AlertDescription>Search-space controls are specific to the Vanity app. For this app, use raw JSON or input file.</AlertDescription>
                    </Alert>
                  </div>

                  <div classList={{ hidden: inputMode() !== 'json' }} class="space-y-1" data-testid="json-input-panel">
                    <Label for="input-json">Input JSON</Label>
                    <Textarea id="input-json" aria-label="Input JSON" class="font-mono text-xs" value={inputJson()} onInput={(event: Event & { currentTarget: HTMLTextAreaElement }) => setInputJson(event.currentTarget.value)} />
                  </div>

                  <div classList={{ hidden: inputMode() !== 'file' }} class="space-y-1" data-testid="file-input-panel">
                    <Label for="input-file">Input Data File</Label>
                    <Input id="input-file" aria-label="Input Data File" type="file" onChange={(event: Event & { currentTarget: HTMLInputElement }) => setInputFileName(event.currentTarget.files?.[0]?.name ?? '')} />
                  </div>

                  <div class="flex items-center justify-between">
                    <Button type="button" variant="outline" onClick={() => setActiveStep('step-1')}>Back</Button>
                    <Button type="button" onClick={() => setActiveStep('step-3')}>Continue to Review</Button>
                  </div>

                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle>Input Clarity</CardTitle>
                  <CardDescription>Make the contract obvious before run.</CardDescription>
                </CardHeader>
                <CardContent class="space-y-3 text-sm" data-testid="input-clarity-panel">
                  <p><strong class="text-foreground">Platform:</strong> Job envelope + runtime + scheduler destination.</p>
                  <p><strong class="text-foreground">App:</strong> {selectedPreset().name}</p>
                  <p><strong class="text-foreground">Input:</strong> {selectedPreset().inputLabel}</p>
                  <p><strong class="text-foreground">App Field Summary:</strong> {appInputSummary()}</p>
                  <p><strong class="text-foreground">Output:</strong> {selectedPreset().outputLabel}</p>
                  <p><strong class="text-foreground">Expected Behavior:</strong> {selectedPreset().outcome}</p>
                  <Separator />
                  <p class="text-xs text-muted-foreground">{selectedPreset().benchmarkHint}</p>
                </CardContent>
              </Card>
            </div>
          </TabsContent>

          <TabsContent value="step-3" class="space-y-4" data-testid="run-step-review">
            <div class="grid gap-4 lg:grid-cols-3">
              <Card class="lg:col-span-2">
                <CardHeader>
                  <CardTitle>Review Contract Before Submission</CardTitle>
                  <CardDescription>Users should be able to answer: what goes in, what comes out, and what the network will do.</CardDescription>
                </CardHeader>
                <CardContent class="space-y-4">
                  <div class="grid gap-4 md:grid-cols-3">
                    <Card>
                      <CardHeader>
                        <CardTitle class="text-base">Input</CardTitle>
                      </CardHeader>
                      <CardContent class="text-sm">
                        <p>Platform envelope: job name, runtime id, scheduler URL, execution policy.</p>
                        <p class="mt-2">App fields: {appInputSummary()}</p>
                      </CardContent>
                    </Card>
                    <Card>
                      <CardHeader>
                        <CardTitle class="text-base">Output</CardTitle>
                      </CardHeader>
                      <CardContent class="text-sm">
                        <p>{selectedPreset().outputLabel}</p>
                      </CardContent>
                    </Card>
                    <Card>
                      <CardHeader>
                        <CardTitle class="text-base">What Will Happen</CardTitle>
                      </CardHeader>
                      <CardContent class="text-sm">
                        <p>{selectedPreset().outcome}</p>
                      </CardContent>
                    </Card>
                  </div>

                  <Alert>
                    <AlertTitle>Onboarding Path</AlertTitle>
                    <AlertDescription>
                      Default first demo should use the vanity generator preset so users get a tangible result before exploring custom modules.
                    </AlertDescription>
                  </Alert>

                  <div class="space-y-2" data-testid="submission-feedback">
                    <Alert hidden={submitStatus() !== 'pending'}>
                      <AlertTitle>Submitting</AlertTitle>
                      <AlertDescription>{submitMessage()}</AlertDescription>
                    </Alert>
                    <Alert hidden={submitStatus() !== 'success'} data-testid="submit-success">
                      <AlertTitle>Submission Accepted</AlertTitle>
                      <AlertDescription>
                        {submitMessage()}
                        {' '}
                        Receipt:
                        {' '}
                        <span class="font-mono">{lastReceiptId()}</span>
                      </AlertDescription>
                    </Alert>
                    <Alert hidden={submitStatus() !== 'error'} data-testid="submit-error">
                      <AlertTitle>Submission Error</AlertTitle>
                      <AlertDescription>{submitMessage()}</AlertDescription>
                    </Alert>
                    <Alert hidden={validationErrors().length === 0} data-testid="validation-errors">
                      <AlertTitle>Fix Before Submit</AlertTitle>
                      <AlertDescription>
                        <ul class="list-disc pl-5">
                          <For each={validationErrors()}>{(err) => <li>{err}</li>}</For>
                        </ul>
                      </AlertDescription>
                    </Alert>
                  </div>

                  <div class="flex items-center gap-2">
                    <Button type="button" variant="outline" onClick={() => setActiveStep('step-2')}>Back</Button>
                    <Button type="button" disabled={submitStatus() === 'pending'} class="flex-1" onClick={() => void handleSubmit()}>
                      Submit Job
                      <GeneratingIndicator class="ml-2 text-xs" />
                    </Button>
                  </div>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle>Cost + Runtime Estimate</CardTitle>
                  <CardDescription>Preview based on selected profile.</CardDescription>
                </CardHeader>
                <CardContent class="space-y-3 text-sm">
                  <Table>
                    <TableBody>
                      <TableRow>
                        <TableCell class="text-muted-foreground">Mode</TableCell>
                        <TableCell class="text-right font-mono">{submissionMode()}</TableCell>
                      </TableRow>
                      <TableRow>
                        <TableCell class="text-muted-foreground">Execution</TableCell>
                        <TableCell class="text-right font-mono">{executionMode()}</TableCell>
                      </TableRow>
                      <TableRow>
                        <TableCell class="text-muted-foreground">Est. Runtime</TableCell>
                        <TableCell class="text-right font-mono">~12s</TableCell>
                      </TableRow>
                      <TableRow>
                        <TableCell class="text-muted-foreground">Est. Fee</TableCell>
                        <TableCell class="text-right font-mono">Generating</TableCell>
                      </TableRow>
                    </TableBody>
                  </Table>
                  <GeneratingIndicator class="text-xs" />
                </CardContent>
              </Card>
            </div>
          </TabsContent>
        </Tabs>
      </section>

      <Dialog open={safetyOpen()} onOpenChange={setSafetyOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Execution Mode Safety</DialogTitle>
            <DialogDescription>Choose mode intentionally based on key exposure requirements.</DialogDescription>
          </DialogHeader>
          <div class="space-y-3 text-sm text-muted-foreground">
            <p><strong class="text-foreground">secure-local:</strong> seed material stays on client. No worker seed exposure.</p>
            <p><strong class="text-foreground">distributed-insecure:</strong> sends seed-derived work to workers for distributed throughput.</p>
            <p>Use distributed-insecure only when seed exposure is acceptable for the run.</p>
          </div>
          <DialogFooter>
            <DialogClose class="inline-flex h-9 items-center rounded-md border border-border px-3 text-sm hover:bg-muted/50">
              Close
            </DialogClose>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </PageShell>
  )
}
