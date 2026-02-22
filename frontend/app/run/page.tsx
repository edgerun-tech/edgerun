import { For, createMemo, createSignal } from 'solid-js'
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

export default function RunPage() {
  const [safetyOpen, setSafetyOpen] = createSignal(false)
  const [submissionMode, setSubmissionMode] = createSignal<'preset' | 'custom'>('preset')
  const [selectedPresetId, setSelectedPresetId] = createSignal(DEFAULT_PRESET.id)
  const [inputMode, setInputMode] = createSignal<'predefined' | 'json' | 'file'>('predefined')
  const [executionMode, setExecutionMode] = createSignal<'secure-local' | 'distributed-insecure'>('distributed-insecure')

  const selectedPreset = createMemo<PresetApp>(() => PRESET_APPS.find((app) => app.id === selectedPresetId()) ?? DEFAULT_PRESET)

  return (
    <PageShell>
      <PageHero
        title="Run Job"
        badge="Guided Flow"
        description="Pick an app preset or upload your own module, define inputs, and review exactly what outputs and runtime steps will be produced."
        actions={
          <>
            <a href="/docs/getting-started/quick-start/"><Button>Open Get Started Guide</Button></a>
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

        <Tabs defaultValue="step-1" class="space-y-4">
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
                        onInput={(event: Event & { currentTarget: HTMLSelectElement }) => setSelectedPresetId(event.currentTarget.value)}
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
                      <Input id="custom-module-name" aria-label="Custom Module Name" placeholder="my-runtime-module" />
                    </div>
                    <div class="space-y-1">
                      <Label for="custom-wasm">WASM Module</Label>
                      <Input id="custom-wasm" aria-label="Custom WASM Module" type="file" accept=".wasm" />
                    </div>
                    <p class="text-xs text-muted-foreground">Upload a deterministic WASM artifact and define expected output shape in step 3 before submitting.</p>
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
                  <CardTitle>Input Configuration</CardTitle>
                  <CardDescription>Choose predefined values, paste JSON, or upload an input file.</CardDescription>
                </CardHeader>
                <CardContent class="space-y-4">
                  <div class="grid gap-4 md:grid-cols-2">
                    <div class="space-y-1">
                      <Label for="job-name">Job Name</Label>
                      <Input id="job-name" aria-label="Job Name" value={selectedPreset().defaultJobName} />
                    </div>
                    <div class="space-y-1">
                      <Label for="runtime-id">Runtime ID (hex)</Label>
                      <Input id="runtime-id" aria-label="Runtime ID" class="font-mono text-xs" value={selectedPreset().defaultRuntimeId} />
                    </div>
                  </div>

                  <div class="space-y-1">
                    <Label for="input-mode">Input Source</Label>
                    <Select
                      id="input-mode"
                      aria-label="Input Source"
                      value={inputMode()}
                      onInput={(event: Event & { currentTarget: HTMLSelectElement }) => setInputMode(event.currentTarget.value as 'predefined' | 'json' | 'file')}
                    >
                      <option value="predefined">Predefined fields</option>
                      <option value="json">Raw JSON payload</option>
                      <option value="file">Upload input file</option>
                    </Select>
                  </div>

                  <div classList={{ hidden: inputMode() !== 'predefined' }} class="space-y-3" data-testid="predefined-input-panel">
                    <div class="grid gap-4 md:grid-cols-3">
                      <div class="space-y-1">
                        <Label for="prefix">Prefix</Label>
                        <Input id="prefix" aria-label="Prefix" value="So1" class="font-mono text-xs" />
                      </div>
                      <div class="space-y-1">
                        <Label for="start-counter">Start Counter</Label>
                        <Input id="start-counter" aria-label="Start Counter" type="number" min="0" value="0" class="font-mono text-xs" />
                      </div>
                      <div class="space-y-1">
                        <Label for="end-counter">End Counter</Label>
                        <Input id="end-counter" aria-label="End Counter" type="number" min="1" value="1000000" class="font-mono text-xs" />
                      </div>
                    </div>
                    <div class="grid gap-4 md:grid-cols-2">
                      <div class="space-y-1">
                        <Label for="chunk-attempts">Chunk Attempts</Label>
                        <Input id="chunk-attempts" aria-label="Chunk Attempts" type="number" min="1" value="50000" class="font-mono text-xs" />
                      </div>
                      <div class="space-y-1">
                        <Label for="worker-count">Worker Count</Label>
                        <Input id="worker-count" aria-label="Worker Count" type="number" min="1" value="5" />
                      </div>
                    </div>
                  </div>

                  <div classList={{ hidden: inputMode() !== 'json' }} class="space-y-1" data-testid="json-input-panel">
                    <Label for="input-json">Input JSON</Label>
                    <Textarea id="input-json" aria-label="Input JSON" class="font-mono text-xs" value={selectedPreset().defaultInputJson} />
                  </div>

                  <div classList={{ hidden: inputMode() !== 'file' }} class="space-y-1" data-testid="file-input-panel">
                    <Label for="input-file">Input Data File</Label>
                    <Input id="input-file" aria-label="Input Data File" type="file" />
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
                    <Input id="scheduler-url" aria-label="Scheduler URL" value="http://127.0.0.1:8090" class="font-mono text-xs" />
                  </div>

                  <div class="flex items-center gap-2">
                    <Checkbox id="allow-seed-exposure" aria-label="Allow worker seed exposure" checked={executionMode() === 'distributed-insecure'} />
                    <Label for="allow-seed-exposure" class="text-xs text-muted-foreground">Allow worker seed exposure (required for distributed-insecure).</Label>
                  </div>

                  <div class="flex justify-end">
                    <Button type="button" variant="outline" size="sm" onClick={() => setSafetyOpen(true)}>
                      Mode Safety
                    </Button>
                  </div>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle>Input Clarity</CardTitle>
                  <CardDescription>Make the contract obvious before run.</CardDescription>
                </CardHeader>
                <CardContent class="space-y-3 text-sm" data-testid="input-clarity-panel">
                  <p><strong class="text-foreground">Input:</strong> {selectedPreset().inputLabel}</p>
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
                        <p>{selectedPreset().inputLabel}</p>
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

                  <Button type="submit" disabled class="w-full">
                    Submit Job
                    <GeneratingIndicator class="ml-2 text-xs" />
                  </Button>
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
