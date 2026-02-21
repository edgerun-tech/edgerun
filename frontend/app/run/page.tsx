import { createSignal } from 'solid-js'
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

const ADDRESS_DEFAULTS = {
  jobName: 'solana-address-prefix-search',
  runtimeId: '0000000000000000000000000000000000000000000000000000000000000000',
  prefix: 'So1',
  startCounter: '0',
  endCounter: '1000000',
  chunkAttempts: '50000',
  workerCount: '5',
  escrowPerJobLamports: '1000000',
  maxEscrowLamports: '20000000'
}

export default function RunPage() {
  const [safetyOpen, setSafetyOpen] = createSignal(false)

  return (
    <PageShell>
      <PageHero
        title="Execute Workflow"
        badge="Guided Start"
        description="Start with the Get Started guide, then trace each step through scheduler execution and settlement."
        actions={
          <>
            <a href="/docs/getting-started/quick-start/"><Button>Open Get Started Guide</Button></a>
            <Button variant="outline" disabled>
              Web Submission UI
              <GeneratingIndicator class="ml-2 text-[10px]" />
            </Button>
          </>
        }
      />

      <section class="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        <p class="mb-6 text-sm font-mono text-muted-foreground">Recommended onboarding path: address generator CLI to scheduler API (`http://127.0.0.1:8080`) with deterministic worker output verification.</p>

        <div class="mb-6 grid gap-4 md:grid-cols-3">
          <a href="/docs/getting-started/quick-start/">
            <Card class="h-full cursor-pointer transition-colors hover:border-primary/50">
              <CardHeader>
                <CardTitle>1. Get Started Guide</CardTitle>
                <CardDescription>Follow the guided setup for secure-local and distributed execution options.</CardDescription>
              </CardHeader>
            </Card>
          </a>
          <a href="/docs/main/address-generator-payload.html">
            <Card class="h-full cursor-pointer transition-colors hover:border-primary/50">
              <CardHeader>
                <CardTitle>2. Payload Reference</CardTitle>
                <CardDescription>Review deterministic payload behavior and `(seed, counter)` output encoding.</CardDescription>
              </CardHeader>
            </Card>
          </a>
          <a href="/docs/main/scheduler-api.html">
            <Card class="h-full cursor-pointer transition-colors hover:border-primary/50">
              <CardHeader>
                <CardTitle>3. API + Verification</CardTitle>
                <CardDescription>Inspect generated scheduler endpoints and verify end-to-end state transitions.</CardDescription>
              </CardHeader>
            </Card>
          </a>
        </div>

        <div class="grid gap-4 lg:grid-cols-3">
          <Card class="lg:col-span-2">
            <CardHeader>
              <CardTitle>Job Configuration</CardTitle>
              <CardDescription>Pre-filled to mirror the Get Started guide so you can reproduce the same flow quickly.</CardDescription>
            </CardHeader>
            <CardContent>
              <Alert class="mb-4">
                <AlertTitle>Guided Starter Configuration</AlertTitle>
                <AlertDescription>
                  These defaults mirror the distributed address-generator workflow for consistent CLI, docs, and API validation.
                </AlertDescription>
              </Alert>

              <form class="space-y-4">
                <div class="space-y-1">
                  <Label for="job-name">Job Name</Label>
                  <Input id="job-name" aria-label="Job Name" value={ADDRESS_DEFAULTS.jobName} />
                </div>
                <div class="space-y-1">
                  <Label for="runtime-id">Runtime ID (hex)</Label>
                  <Input id="runtime-id" aria-label="Runtime ID" class="font-mono text-xs" value={ADDRESS_DEFAULTS.runtimeId} />
                  <p class="text-xs text-muted-foreground">Matches the runtime ID shown in the Get Started guide. Must match scheduler allowlist.</p>
                </div>
                <div class="grid gap-4 md:grid-cols-3">
                  <div class="space-y-1">
                    <Label for="prefix">Prefix</Label>
                    <Input id="prefix" aria-label="Prefix" value={ADDRESS_DEFAULTS.prefix} class="font-mono text-xs" />
                  </div>
                  <div class="space-y-1">
                    <Label for="start-counter">Start Counter</Label>
                    <Input id="start-counter" aria-label="Start Counter" type="number" min="0" value={ADDRESS_DEFAULTS.startCounter} class="font-mono text-xs" />
                  </div>
                  <div class="space-y-1">
                    <Label for="end-counter">End Counter</Label>
                    <Input id="end-counter" aria-label="End Counter" type="number" min="1" value={ADDRESS_DEFAULTS.endCounter} class="font-mono text-xs" />
                  </div>
                </div>
                <div class="grid gap-4 md:grid-cols-2">
                  <div class="space-y-1">
                    <Label for="chunk-attempts">Chunk Attempts</Label>
                    <Input id="chunk-attempts" aria-label="Chunk Attempts" type="number" min="1" value={ADDRESS_DEFAULTS.chunkAttempts} class="font-mono text-xs" />
                  </div>
                  <div class="space-y-1">
                    <Label for="worker-count">Worker Count</Label>
                    <Input id="worker-count" aria-label="Worker Count" type="number" min="3" max="10" value={ADDRESS_DEFAULTS.workerCount} />
                  </div>
                </div>
                <div class="grid gap-4 md:grid-cols-2">
                  <div class="space-y-1">
                    <Label for="escrow-per-job">Escrow Per Job (lamports)</Label>
                    <Input id="escrow-per-job" aria-label="Escrow Per Job lamports" type="number" min="1" value={ADDRESS_DEFAULTS.escrowPerJobLamports} class="font-mono text-xs" />
                  </div>
                  <div class="space-y-1">
                    <Label for="max-escrow">Max Escrow (lamports)</Label>
                    <Input id="max-escrow" aria-label="Max Escrow lamports" type="number" min="1" value={ADDRESS_DEFAULTS.maxEscrowLamports} class="font-mono text-xs" />
                  </div>
                </div>
                <div class="grid gap-4 md:grid-cols-2">
                  <div class="space-y-1">
                    <Label for="wasm-file">WASM Module</Label>
                    <Input id="wasm-file" aria-label="WASM Module" type="file" accept=".wasm" />
                    <p class="text-xs text-muted-foreground">Use the generated address payload wasm artifact from your current build output.</p>
                  </div>
                  <div class="space-y-1">
                    <Label for="input-file">Input Data (optional file)</Label>
                    <Input id="input-file" aria-label="Input Data optional" type="file" />
                    <p class="text-xs text-muted-foreground">For distributed mode request data; secure-local mode keeps seed only on client.</p>
                  </div>
                </div>
                <div class="space-y-1">
                  <Label for="execution-mode">Execution Mode</Label>
                  <Select id="execution-mode" aria-label="Execution Mode" value="distributed-insecure">
                    <option value="secure-local">secure-local</option>
                    <option value="distributed-insecure">distributed-insecure</option>
                  </Select>
                </div>
                <div class="space-y-1">
                  <Label for="scheduler-url">Scheduler URL</Label>
                  <Input id="scheduler-url" aria-label="Scheduler URL" value="http://127.0.0.1:8080" class="font-mono text-xs" />
                </div>
                <div class="space-y-1">
                  <Label for="note">Notes (optional)</Label>
                  <Textarea
                    id="note"
                    aria-label="Notes optional"
                    value="guided workflow mirror; compare with docs and scheduler API snapshots"
                  />
                </div>
                <div class="flex items-center gap-2">
                  <Checkbox id="allow-seed-exposure" aria-label="Allow worker seed exposure" checked />
                  <Label for="allow-seed-exposure" class="text-xs text-muted-foreground">Allow worker seed exposure (required for distributed-insecure mode).</Label>
                </div>
                <div class="flex justify-end">
                  <Button type="button" variant="outline" size="sm" onClick={() => setSafetyOpen(true)}>
                    Mode Safety
                  </Button>
                </div>
                <Separator />
                <Button type="submit" disabled class="w-full">
                  Submit Job
                  <GeneratingIndicator class="ml-2 text-xs" />
                </Button>
                <p class="text-xs text-muted-foreground">
                  End-to-end docs:
                  {' '}
                  <a class="underline" href="/docs/getting-started/quick-start/">get started guide</a>
                  {' '}
                  +
                  {' '}
                  <a class="underline" href="/docs/main/address-generator-payload.html">payload reference</a>.
                </p>
              </form>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Cost Estimate</CardTitle>
              <CardDescription>Estimated costs for this job.</CardDescription>
            </CardHeader>
            <CardContent class="space-y-3 text-sm">
              <Table>
                <TableBody>
                  <TableRow>
                    <TableCell class="text-muted-foreground">Workers</TableCell>
                    <TableCell class="text-right font-mono">{ADDRESS_DEFAULTS.workerCount}</TableCell>
                  </TableRow>
                  <TableRow>
                    <TableCell class="text-muted-foreground">Est. Runtime</TableCell>
                    <TableCell class="text-right font-mono">~12s</TableCell>
                  </TableRow>
                  <TableRow>
                    <TableCell class="text-muted-foreground">Escrow / chunk</TableCell>
                    <TableCell class="text-right font-mono">{ADDRESS_DEFAULTS.escrowPerJobLamports}</TableCell>
                  </TableRow>
                  <TableRow>
                    <TableCell class="text-muted-foreground">Max Escrow</TableCell>
                    <TableCell class="text-right font-mono">{ADDRESS_DEFAULTS.maxEscrowLamports}</TableCell>
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

        <div class="mt-8 grid gap-4 md:grid-cols-3">
          <Card>
            <CardHeader>
              <CardTitle>1. Submit Bundle + Limits</CardTitle>
              <CardDescription>Upload payload, runtime ID, and worker count envelope.</CardDescription>
            </CardHeader>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>2. Deterministic Execution</CardTitle>
              <CardDescription>Workers execute identical WASM and produce attestable outputs.</CardDescription>
            </CardHeader>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>3. Collect Result + Proof</CardTitle>
              <CardDescription>Read job status, consensus result, and settlement evidence.</CardDescription>
            </CardHeader>
          </Card>
        </div>
      </section>

      <Dialog open={safetyOpen()} onOpenChange={setSafetyOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Execution Mode Safety</DialogTitle>
            <DialogDescription>Choose mode intentionally based on key exposure requirements.</DialogDescription>
          </DialogHeader>
          <div class="space-y-3 text-sm text-muted-foreground">
            <p><strong class="text-foreground">secure-local:</strong> seed material stays on client. Lowest exposure, no distributed worker search.</p>
            <p><strong class="text-foreground">distributed-insecure:</strong> sends seed-derived work to workers for distributed throughput. Requires explicit exposure acceptance.</p>
            <p>Use secure-local for sensitive production keys. Use distributed mode only for acceptable exposure scenarios.</p>
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
