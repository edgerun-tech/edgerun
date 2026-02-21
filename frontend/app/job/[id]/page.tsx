import { For, untrack } from 'solid-js'
import { Badge } from '../../../components/ui/badge'
import { Button } from '../../../components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '../../../components/ui/card'
import { PageShell } from '../../../components/layout/page-shell'
import { PageHero } from '../../../components/layout/page-hero'
import { Separator } from '../../../components/ui/separator'
import { formatDate, formatHash, formatMs, formatNumber, jobs, jobStatusBadge, timelineEvents } from '../../../lib/content'

type JobDetailsProps = {
  id?: string
}

export default function JobDetailsPage(props: JobDetailsProps) {
  const fallbackJob = {
    id: 'job_generating',
    name: 'Generating job details',
    status: 'running' as const,
    createdAt: new Date().toISOString(),
    wasmHash: '0x',
    runtimeMs: 0,
    gasUsed: 0,
    executorCount: 0,
    consensusReached: false,
    settlementTx: '',
    input: { fileName: 'generating', sizeBytes: 0 },
    results: []
  }
  const id = untrack(() => props.id)
  const job = jobs.find((item) => item.id === id) || jobs[0] || fallbackJob

  return (
    <PageShell>
      <PageHero
        title={job.name}
        badge={job.status}
        badgeVariant={jobStatusBadge(job.status)}
        description={`Job ID: ${job.id}`}
        actions={<a href="/run/"><Button variant="outline">Run New Job</Button></a>}
      />

      <section class="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        <div class="grid gap-6 lg:grid-cols-3">
          <div class="space-y-6 lg:col-span-2">
            <Card>
              <CardHeader><CardTitle>Execution Summary</CardTitle></CardHeader>
              <CardContent class="grid gap-4 sm:grid-cols-2">
                <p class="text-sm text-muted-foreground">Runtime: <span class="font-mono text-foreground">{formatMs(job.runtimeMs)}</span></p>
                <p class="text-sm text-muted-foreground">Gas Used: <span class="font-mono text-foreground">{formatNumber(job.gasUsed)}</span></p>
                <p class="text-sm text-muted-foreground">Workers: <span class="font-mono text-foreground">{job.executorCount}</span></p>
                <p class="text-sm text-muted-foreground">Consensus: <span class="font-mono text-foreground">{job.consensusReached ? 'reached' : 'pending'}</span></p>
                <p class="text-sm text-muted-foreground">Created: <span class="text-foreground">{formatDate(job.createdAt)}</span></p>
                <p class="text-sm text-muted-foreground">Input: <span class="text-foreground">{job.input.fileName}</span></p>
              </CardContent>
            </Card>

            <Card>
              <CardHeader><CardTitle>Hashes + Settlement</CardTitle></CardHeader>
              <CardContent class="space-y-3">
                <p class="text-sm text-muted-foreground">WASM Hash</p>
                <p class="font-mono text-sm break-all">{job.wasmHash}</p>
                <Separator />
                <p class="text-sm text-muted-foreground">Settlement Transaction</p>
                <p class="font-mono text-sm break-all text-primary">{job.settlementTx || 'Generating'}</p>
              </CardContent>
            </Card>

            <Card>
              <CardHeader><CardTitle>Worker Results</CardTitle></CardHeader>
              <CardContent class="space-y-3">
                <For each={job.results}>{(result) => (
                  <div class="rounded-lg border border-border p-3">
                    <div class="mb-2 flex items-center justify-between">
                      <p class="font-mono text-sm">{result.workerName}</p>
                      <Badge variant={result.status === 'completed' ? 'default' : 'destructive'}>{result.status}</Badge>
                    </div>
                    <p class="text-xs text-muted-foreground">Output: <span class="font-mono">{formatHash(result.outputHash, 18, 12)}</span></p>
                    <p class="text-xs text-muted-foreground">Gas: {formatNumber(result.gasUsed)} • Runtime: {formatMs(result.runtimeMs)}</p>
                  </div>
                )}</For>
              </CardContent>
            </Card>
          </div>

          <Card class="h-fit lg:sticky lg:top-20">
            <CardHeader><CardTitle>Timeline</CardTitle></CardHeader>
            <CardContent class="space-y-4">
              <For each={timelineEvents}>{(event) => (
                <div class="flex gap-3">
                  <div class="mt-1 h-2 w-2 rounded-full bg-primary" />
                  <div>
                    <p class="text-sm font-medium">{event.title}</p>
                    <p class="text-xs text-muted-foreground">{event.description}</p>
                    <p class="mt-1 text-xs text-muted-foreground">{formatDate(event.timestamp)}</p>
                    {event.txHash && <p class="mt-1 font-mono text-xs text-primary">{formatHash(event.txHash)}</p>}
                  </div>
                </div>
              )}</For>
            </CardContent>
          </Card>
        </div>
      </section>
    </PageShell>
  )
}
