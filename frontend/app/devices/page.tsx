// SPDX-License-Identifier: Apache-2.0
import { For, createMemo, createSignal } from 'solid-js'
import { PageHero } from '../../components/layout/page-hero'
import { PageShell } from '../../components/layout/page-shell'
import { Card, CardContent, CardHeader, CardTitle } from '../../components/ui/card'
import { Input } from '../../components/ui/input'
import { Select } from '../../components/ui/select'
import { Button } from '../../components/ui/button'

type DeviceStatus = 'online' | 'degraded' | 'offline'
type DeviceRow = {
  id: string
  name: string
  region: string
  status: DeviceStatus
  temp: number
  cpu: number
  mem: number
  jobs: number
  queue: number
  latencyMs: number
}

const allDevices: DeviceRow[] = Array.from({ length: 24 }, (_, index) => {
  const id = `edge-${String(index + 1).padStart(3, '0')}`
  const status: DeviceStatus = index % 11 === 0 ? 'offline' : index % 5 === 0 ? 'degraded' : 'online'
  return {
    id,
    name: `Node ${index + 1}`,
    region: ['us-west-2', 'us-east-1', 'eu-central-1', 'ap-southeast-1'][index % 4]!,
    status,
    temp: 54 + ((index * 3) % 17),
    cpu: 38 + ((index * 7) % 58),
    mem: 42 + ((index * 9) % 53),
    jobs: 2 + (index % 10),
    queue: 3 + ((index * 5) % 27),
    latencyMs: 16 + ((index * 4) % 41)
  }
})

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

function formatPercent(value: number): string {
  return `${Math.round(value)}%`
}

export default function DevicesPage() {
  const [statusFilter, setStatusFilter] = createSignal<'all' | DeviceStatus>('all')
  const [query, setQuery] = createSignal('')
  const [blingMode, setBlingMode] = createSignal(true)

  const filteredDevices = createMemo(() => {
    const status = statusFilter()
    const search = query().trim().toLowerCase()
    return allDevices.filter((device) => {
      if (status !== 'all' && device.status !== status) return false
      if (!search) return true
      return (
        device.id.toLowerCase().includes(search) ||
        device.name.toLowerCase().includes(search) ||
        device.region.toLowerCase().includes(search)
      )
    })
  })

  const fleetKpis = createMemo(() => {
    const active = filteredDevices()
    const online = active.filter((device) => device.status === 'online').length
    const degraded = active.filter((device) => device.status === 'degraded').length
    const offline = active.filter((device) => device.status === 'offline').length
    const avgCpu = active.length > 0 ? active.reduce((sum, device) => sum + device.cpu, 0) / active.length : 0
    const avgMem = active.length > 0 ? active.reduce((sum, device) => sum + device.mem, 0) / active.length : 0
    const queueDepth = active.reduce((sum, device) => sum + device.queue, 0)
    const sortedLatency = active.map((device) => device.latencyMs).sort((left, right) => left - right)
    const p95Index = sortedLatency.length > 0 ? Math.max(0, Math.ceil(sortedLatency.length * 0.95) - 1) : 0
    const p95Latency = sortedLatency.length > 0 ? sortedLatency[p95Index] : 0

    return [
      { label: 'Visible Devices', value: String(active.length) },
      { label: 'Online', value: String(online) },
      { label: 'Degraded', value: String(degraded) },
      { label: 'Offline', value: String(offline) },
      { label: 'Avg CPU', value: formatPercent(avgCpu) },
      { label: 'Avg Memory', value: formatPercent(avgMem) },
      { label: 'Queue Depth', value: String(queueDepth) },
      { label: 'P95 Latency', value: `${p95Latency}ms` }
    ]
  })

  const alerts = createMemo(() => (
    filteredDevices()
      .filter((device) => device.status !== 'online')
      .slice(0, 6)
      .map((device, index) => ({
        severity: device.status === 'offline' ? 'critical' : 'warning',
        message: `${device.id} ${device.status === 'offline' ? 'did not answer route probe' : 'showing elevated CPU and queue depth'}`,
        age: `${18 + (index * 13)}s`
      }))
  ))
  const renderedAlerts = createMemo(() => {
    const activeAlerts = alerts()
    if (activeAlerts.length > 0) return activeAlerts
    return [{ severity: 'info', message: 'No active alerts in filtered view.', age: '0s' }]
  })

  return (
    <PageShell>
      <PageHero
        title="Devices"
        badge="Demo Bling"
        description="TV-safe fleet board with interactive filters and high-density status visibility."
      />

      <section class="mx-auto max-w-[1920px] px-4 py-5 sm:px-6 lg:px-8" data-testid="devices-dashboard">
        <Card class="border-border/90 bg-card/80">
          <CardHeader class="p-3 pb-2">
            <CardTitle class="text-sm uppercase tracking-[0.14em] text-muted-foreground">
              View Controls
            </CardTitle>
          </CardHeader>
          <CardContent class="grid gap-2 p-3 md:grid-cols-[minmax(14rem,1fr)_12rem_auto]">
              <Input
                value={query()}
                onInput={(event: InputEvent & { currentTarget: HTMLInputElement }) => setQuery(event.currentTarget.value)}
                placeholder="Search device id, name, or region"
                data-testid="devices-search-input"
              />
              <Select
                value={statusFilter()}
                onChange={(event: Event & { currentTarget: HTMLSelectElement }) => setStatusFilter(event.currentTarget.value as 'all' | DeviceStatus)}
                data-testid="devices-status-filter"
              >
              <option value="all">All statuses</option>
              <option value="online">Online</option>
              <option value="degraded">Degraded</option>
              <option value="offline">Offline</option>
            </Select>
            <div class="flex gap-2">
              <Button
                variant={blingMode() ? 'default' : 'outline'}
                onClick={() => setBlingMode((enabled) => !enabled)}
                data-testid="devices-bling-toggle"
              >
                {blingMode() ? 'Bling On' : 'Bling Off'}
              </Button>
              <Button
                variant="outline"
                onClick={() => {
                  setQuery('')
                  setStatusFilter('all')
                }}
                data-testid="devices-reset-filters"
              >
                Reset
              </Button>
            </div>
          </CardContent>
        </Card>

        <div class="mt-3 grid grid-cols-2 gap-3 md:grid-cols-4 xl:grid-cols-8" data-testid="devices-kpis">
          <For each={fleetKpis()}>{(kpi) => (
            <Card class={`border-border/90 ${blingMode() ? 'bg-sky-950/20' : 'bg-card/80'}`} data-testid="devices-kpi-card">
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
              <CardTitle class="text-sm uppercase tracking-[0.14em] text-muted-foreground">
                Fleet Table ({filteredDevices().length}/{allDevices.length})
              </CardTitle>
            </CardHeader>
            <CardContent class="overflow-x-auto p-0">
              <table class="min-w-full border-collapse text-[11px]">
                <thead>
                  <tr class="border-y border-border/80 bg-muted/30 text-left text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
                    <th class="px-3 py-2">Device</th>
                    <th class="px-3 py-2">Name</th>
                    <th class="px-3 py-2">Region</th>
                    <th class="px-3 py-2">Status</th>
                    <th class="px-3 py-2">CPU</th>
                    <th class="px-3 py-2">Mem</th>
                    <th class="px-3 py-2">Temp</th>
                    <th class="px-3 py-2">Jobs</th>
                  </tr>
                </thead>
                <tbody>
                  <For each={filteredDevices()}>{(device) => (
                    <tr class="border-b border-border/50 last:border-b-0">
                      <td class="px-3 py-1.5 font-mono">{device.id}</td>
                      <td class="px-3 py-1.5">{device.name}</td>
                      <td class="px-3 py-1.5">{device.region}</td>
                      <td class="px-3 py-1.5">
                        <span class={`inline-flex rounded px-1.5 py-0.5 text-[10px] uppercase ${
                          device.status === 'online'
                            ? 'bg-emerald-500/15 text-emerald-300'
                            : device.status === 'degraded'
                              ? 'bg-amber-500/15 text-amber-300'
                              : 'bg-rose-500/15 text-rose-300'
                        }`}>
                          {device.status}
                        </span>
                      </td>
                      <td class="px-3 py-1.5 font-mono">{device.cpu}%</td>
                      <td class="px-3 py-1.5 font-mono">{device.mem}%</td>
                      <td class="px-3 py-1.5 font-mono">{device.temp}C</td>
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
              <For each={renderedAlerts()}>{(alert) => (
                <div class="rounded border border-border/70 bg-muted/25 p-2">
                  <p class={`text-[10px] uppercase tracking-[0.14em] ${
                    alert.severity === 'critical' ? 'text-rose-300' : alert.severity === 'warning' ? 'text-amber-300' : 'text-sky-300'
                  }`}>
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

          <Card class={`border-border/90 ${blingMode() ? 'bg-sky-950/20' : 'bg-card/80'}`} data-testid="devices-capacity-heatmap">
            <CardHeader class="p-3 pb-2">
              <CardTitle class="text-sm uppercase tracking-[0.14em] text-muted-foreground">Capacity Grid</CardTitle>
            </CardHeader>
            <CardContent class="grid grid-cols-6 gap-1 p-3 pt-1">
              <For each={filteredDevices().map((device) => Math.round((device.cpu * 0.6) + (device.mem * 0.4)))}>{(value) => (
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
