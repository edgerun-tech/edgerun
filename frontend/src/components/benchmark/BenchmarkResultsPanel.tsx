import type { ComponentProps } from "solid-js"
import { For, createMemo, createSignal, onCleanup, onMount, splitProps } from "solid-js"
import { $metricFocus } from "@/stores/bench-filter"
import { benchmarkResults, type MetricFocus } from "@/data/benchmarks"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Progress } from "@/components/ui/progress"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Table, TableBody, TableCell, TableHead, TableHeadCell, TableRow } from "@/components/ui/table"

const metricOptions: { value: MetricFocus; label: string }[] = [
  { value: "all", label: "All metrics" },
  { value: "throughput", label: "Throughput" },
  { value: "latency", label: "Latency" },
  { value: "memory", label: "Memory" },
  { value: "cpu", label: "CPU" },
]

const metricBadge = {
  all: "default",
  throughput: "secondary",
  latency: "outline",
  memory: "outline",
  cpu: "outline",
} as const

const fairnessScores = (focus: MetricFocus, run: { fairness: { metric: string; normalizedScore: number; confidence: string; weight: number; notes: string }[] }) => {
  if (focus === "all") {
    const score = run.fairness.reduce(
      (acc, value) => acc + value.normalizedScore * (value.weight / 100),
      0,
    )
    return Math.round(score)
  }

  const entry = run.fairness.find((item) =>
    item.metric.toLowerCase().includes(focus === "latency" ? "p99" : focus),
  )
  return entry?.normalizedScore ?? "N/A"
}

export type BenchProps = ComponentProps<"section"> & { title: string; intro: string }

export const BenchmarkResultsPanel = (props: BenchProps) => {
  const [metricLocal, setMetricLocal] = createSignal<MetricFocus>($metricFocus.get())
  const [local, rest] = splitProps(props, ["title", "intro", "class"])

  onMount(() => {
    const unsubscribe = $metricFocus.subscribe((next) => setMetricLocal(next))
    onCleanup(unsubscribe)
  })

  const filtered = createMemo(() => {
    const target = metricLocal()
    if (target === "all") {
      return benchmarkResults
    }

    return benchmarkResults.filter((item) =>
      item.fairness.some((metric) => metric.metric.toLowerCase().includes(target)),
    )
  })

  const updateFocus = (event: Event) => {
    const value = (event.currentTarget as HTMLSelectElement).value as MetricFocus
    $metricFocus.set(value)
    setMetricLocal(value)
  }

  return (
    <section class={["reveal section-stack", local.class].filter(Boolean).join(" ")} {...rest}>
      <header class="space-y-4">
        <h2 class="text-2xl font-semibold text-white">{local.title}</h2>
        <p class="text-slate-300">{local.intro}</p>
      </header>
      <label class="flex items-center gap-3 text-sm text-slate-200">
        Filter:
        <select
          class="rounded-md border border-slate-500 bg-slate-900/60 px-3 py-2 text-sm text-slate-100"
          value={metricLocal()}
          onChange={updateFocus}
        >
          <For each={metricOptions}>
            {(item) => <option value={item.value}>{item.label}</option>}
          </For>
        </select>
      </label>
      <For each={filtered()}>
        {(entry) => {
          const score = fairnessScores(metricLocal(), entry)
          return (
            <Card>
              <CardHeader>
                <div class="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <CardTitle>{entry.run.suite}</CardTitle>
                    <CardDescription>
                      {entry.run.workload} • {entry.run.environment} • {entry.run.commit} • {entry.run.date}
                    </CardDescription>
                  </div>
                  <Badge tone={metricBadge[metricLocal()]}>{metricLocal().toUpperCase()}</Badge>
                </div>
                <div class="text-xs text-slate-300">
                  Branch: {entry.run.branch} · Confidence score uses weighted fairness model
                </div>
              </CardHeader>
              <CardContent>
                <div class="grid gap-4 md:grid-cols-2">
                  <div class="space-y-3 rounded-lg border border-white/10 bg-slate-950/40 p-4">
                    <div class="flex items-center justify-between">
                      <span class="text-sm text-slate-300">Weighted score</span>
                      <span class="text-sm font-semibold text-white">{score}/100</span>
                    </div>
                    <Progress value={typeof score === "number" ? score : 0} />
                  </div>
                  <Table>
                    <TableHead>
                      <TableRow>
                        <TableHeadCell>Metric</TableHeadCell>
                        <TableHeadCell>Weight</TableHeadCell>
                        <TableHeadCell>Score</TableHeadCell>
                        <TableHeadCell>Confidence</TableHeadCell>
                      </TableRow>
                    </TableHead>
                    <TableBody>
                      <For each={entry.fairness}>
                        {(row) => (
                          <TableRow>
                            <TableCell>{row.metric}</TableCell>
                            <TableCell>{row.weight}%</TableCell>
                            <TableCell>{row.normalizedScore}</TableCell>
                            <TableCell>{row.confidence}</TableCell>
                          </TableRow>
                        )}
                      </For>
                    </TableBody>
                  </Table>
                </div>
                <div class="mt-4 flex items-center justify-end gap-2">
                  <Button variant="outline">Raw logs</Button>
                  <Button>Signed manifest</Button>
                </div>
              </CardContent>
            </Card>
          )
        }}
      </For>
    </section>
  )
}
