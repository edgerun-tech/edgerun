// SPDX-License-Identifier: Apache-2.0
import { For } from 'solid-js'
import { PageHero } from '../../components/layout/page-hero'
import { PageShell } from '../../components/layout/page-shell'
import { Card, CardContent, CardHeader, CardTitle } from '../../components/ui/card'

const fleetKpis = [
  { label: 'Total Devices', value: '384' },
  { label: 'Online', value: '372' },
  { label: 'Degraded', value: '9' },
  { label: 'Offline', value: '3' },
  { label: 'Avg CPU', value: '64%' },
  { label: 'Avg Memory', value: '71%' },
  { label: 'Queue Depth', value: '214' },
  { label: 'P95 Latency', value: '42ms' }
]

const devices = Array.from({ length: 18 }, (_, index) => {
  const id = `edge-${String(index + 1).padStart(3, '0')}`
  return {
    id,
    region: ['us-west-2', 'us-east-1', 'eu-central-1'][index % 3],
    status: index % 7 === 0 ? 'degraded' : 'online',
    temp: `${58 + (index % 9)}C`,
    cpu: `${49 + ((index * 5) % 36)}%`,
    mem: `${52 + ((index * 7) % 34)}%`,
    jobs: `${4 + (index % 8)}`
  }
})

const alerts = [
  { severity: 'critical', message: 'edge-007 storage wear exceeded threshold', age: '19s' },
  { severity: 'warning', message: 'edge-042 packet loss spike on uplink B', age: '31s' },
  { severity: 'warning', message: 'edge-111 route convergence delay', age: '1m' },
  { severity: 'info', message: 'fleet config batch 296 applied', age: '2m' }
]

const commandQueue = [
  { cmd: 'sync route-table --fleet us-west', target: '112 nodes', eta: '12s' },
  { cmd: 'rotate cert --group edge-a', target: '64 nodes', eta: '48s' },
  { cmd: 'warm cache --profile inference', target: '208 nodes', eta: '2m' },
  { cmd: 'apply qos policy v14', target: 'all devices', eta: '2m' }
]

const serviceHealth = [
  { service: 'Route Signaler', value: 'healthy', detail: '3/3 replicas' },
  { service: 'Job Scheduler', value: 'healthy', detail: 'queue stable' },
  { service: 'Artifact Mirror', value: 'degraded', detail: 'eu cache lag 14s' },
  { service: 'Telemetry Ingest', value: 'healthy', detail: '14.1k msg/s' },
  { service: 'Control WS', value: 'healthy', detail: '2.8k active sessions' }
]

export default function DevicesPage() {
  return (
    <PageShell>
      <PageHero
        title="Devices"
        badge="Demo Dashboard"
        description="Dense fleet operations view tuned for large displays. Data shown here is demo-only."
      />

      <section class="mx-auto max-w-[1920px] px-4 py-5 sm:px-6 lg:px-8" data-testid="devices-dashboard">
        <div class="grid grid-cols-2 gap-3 md:grid-cols-4 xl:grid-cols-8" data-testid="devices-kpis">
          <For each={fleetKpis}>{(kpi) => (
            <Card class="border-border/90 bg-card/80" data-testid="devices-kpi-card">
              <CardHeader class="space-y-0 p-3">
                <p class="text-[10px] uppercase tracking-[0.18em] text-muted-foreground">{kpi.label}</p>
                <CardTitle class="text-lg leading-none sm:text-xl">{kpi.value}</CardTitle>
              </CardHeader>
            </Card>
          )}</For>
        </div>

        <div class="mt-3 grid gap-3 xl:grid-cols-[2.3fr_1fr_1fr]">
          <Card class="border-border/90 bg-card/80" data-testid="devices-fleet-table">
            <CardHeader class="p-3 pb-2">
              <CardTitle class="text-sm uppercase tracking-[0.14em] text-muted-foreground">Fleet Table</CardTitle>
            </CardHeader>
            <CardContent class="p-0">
              <table class="w-full border-collapse text-[11px]">
                <thead>
                  <tr class="border-y border-border/80 bg-muted/30 text-left text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
                    <th class="px-3 py-2">Device</th>
                    <th class="px-3 py-2">Region</th>
                    <th class="px-3 py-2">Status</th>
                    <th class="px-3 py-2">CPU</th>
                    <th class="px-3 py-2">Mem</th>
                    <th class="px-3 py-2">Temp</th>
                    <th class="px-3 py-2">Jobs</th>
                  </tr>
                </thead>
                <tbody>
                  <For each={devices}>{(device) => (
                    <tr class="border-b border-border/50 last:border-b-0">
                      <td class="px-3 py-1.5 font-mono">{device.id}</td>
                      <td class="px-3 py-1.5">{device.region}</td>
                      <td class="px-3 py-1.5">
                        <span class={`inline-flex rounded px-1.5 py-0.5 text-[10px] uppercase ${device.status === 'online' ? 'bg-emerald-500/15 text-emerald-300' : 'bg-amber-500/15 text-amber-300'}`}>
                          {device.status}
                        </span>
                      </td>
                      <td class="px-3 py-1.5 font-mono">{device.cpu}</td>
                      <td class="px-3 py-1.5 font-mono">{device.mem}</td>
                      <td class="px-3 py-1.5 font-mono">{device.temp}</td>
                      <td class="px-3 py-1.5 font-mono">{device.jobs}</td>
                    </tr>
                  )}</For>
                </tbody>
              </table>
            </CardContent>
          </Card>

          <Card class="border-border/90 bg-card/80" data-testid="devices-alerts">
            <CardHeader class="p-3 pb-2">
              <CardTitle class="text-sm uppercase tracking-[0.14em] text-muted-foreground">Alerts</CardTitle>
            </CardHeader>
            <CardContent class="space-y-2 p-3 pt-1">
              <For each={alerts}>{(alert) => (
                <div class="rounded border border-border/70 bg-muted/25 p-2">
                  <p class={`text-[10px] uppercase tracking-[0.14em] ${alert.severity === 'critical' ? 'text-rose-300' : alert.severity === 'warning' ? 'text-amber-300' : 'text-sky-300'}`}>
                    {alert.severity}
                  </p>
                  <p class="mt-1 text-xs leading-tight">{alert.message}</p>
                  <p class="mt-1 text-[10px] font-mono text-muted-foreground">{alert.age} ago</p>
                </div>
              )}</For>
            </CardContent>
          </Card>

          <Card class="border-border/90 bg-card/80" data-testid="devices-services">
            <CardHeader class="p-3 pb-2">
              <CardTitle class="text-sm uppercase tracking-[0.14em] text-muted-foreground">Service Health</CardTitle>
            </CardHeader>
            <CardContent class="space-y-2 p-3 pt-1">
              <For each={serviceHealth}>{(row) => (
                <div class="grid grid-cols-[1fr_auto] gap-2 border-b border-border/50 pb-1.5 text-xs last:border-b-0">
                  <div>
                    <p>{row.service}</p>
                    <p class="text-[10px] text-muted-foreground">{row.detail}</p>
                  </div>
                  <p class={`text-[10px] uppercase tracking-[0.14em] ${row.value === 'healthy' ? 'text-emerald-300' : 'text-amber-300'}`}>
                    {row.value}
                  </p>
                </div>
              )}</For>
            </CardContent>
          </Card>
        </div>

        <div class="mt-3 grid gap-3 xl:grid-cols-[1.3fr_1fr]">
          <Card class="border-border/90 bg-card/80" data-testid="devices-command-queue">
            <CardHeader class="p-3 pb-2">
              <CardTitle class="text-sm uppercase tracking-[0.14em] text-muted-foreground">Command Queue</CardTitle>
            </CardHeader>
            <CardContent class="p-0">
              <table class="w-full border-collapse text-[11px]">
                <thead>
                  <tr class="border-y border-border/80 bg-muted/30 text-left text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
                    <th class="px-3 py-2">Command</th>
                    <th class="px-3 py-2">Target</th>
                    <th class="px-3 py-2">ETA</th>
                  </tr>
                </thead>
                <tbody>
                  <For each={commandQueue}>{(row) => (
                    <tr class="border-b border-border/50 last:border-b-0">
                      <td class="px-3 py-1.5 font-mono">{row.cmd}</td>
                      <td class="px-3 py-1.5">{row.target}</td>
                      <td class="px-3 py-1.5 font-mono">{row.eta}</td>
                    </tr>
                  )}</For>
                </tbody>
              </table>
            </CardContent>
          </Card>

          <Card class="border-border/90 bg-card/80" data-testid="devices-capacity-heatmap">
            <CardHeader class="p-3 pb-2">
              <CardTitle class="text-sm uppercase tracking-[0.14em] text-muted-foreground">Capacity Grid</CardTitle>
            </CardHeader>
            <CardContent class="grid grid-cols-6 gap-1 p-3 pt-1">
              <For each={Array.from({ length: 42 }, (_, index) => 28 + (index * 9 % 68))}>{(value) => (
                <div
                  class={`h-6 rounded border border-border/60 text-[9px] font-mono leading-6 text-center ${
                    value > 78
                      ? 'bg-rose-500/25'
                      : value > 58
                        ? 'bg-amber-400/20'
                        : 'bg-emerald-500/20'
                  }`}
                >
                  {value}
                </div>
              )}</For>
            </CardContent>
          </Card>
        </div>
      </section>
    </PageShell>
  )
}
