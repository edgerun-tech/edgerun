// SPDX-License-Identifier: Apache-2.0
import { Card, CardContent, CardHeader, CardTitle } from '../../components/ui/card'
import { GeneratingIndicator } from '../../components/ui/generating-indicator'
import { PageHero } from '../../components/layout/page-hero'
import { PageShell } from '../../components/layout/page-shell'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../../components/ui/tabs'

export default function DashboardPage() {
  return (
    <PageShell>
      <PageHero
        title="Dashboard"
        badge="On-Chain Truth"
        description="Core network metrics are fetched from live runtime telemetry and control-plane endpoints."
      />

      <section class="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        <Tabs defaultValue="chain">
          <TabsList>
            <TabsTrigger value="chain">Chain Metrics</TabsTrigger>
            <TabsTrigger value="worker">Worker Health</TabsTrigger>
          </TabsList>
          <TabsContent value="chain" class="p-0">
            <Card class="border-0">
              <CardHeader><CardTitle>On-Chain Throughput</CardTitle></CardHeader>
              <CardContent class="space-y-2 text-sm">
                <p>Slot: <span class="font-mono" data-chain-field="slot">loading...</span></p>
                <p>Block Height: <span class="font-mono" data-chain-field="blockHeight">loading...</span></p>
                <p>TPS: <span class="font-mono" data-chain-field="tps">loading...</span></p>
                <p>Epoch: <span class="font-mono" data-chain-field="epoch">loading...</span></p>
                <p>Cluster: <span class="font-mono" data-chain-field="cluster">loading...</span></p>
              </CardContent>
            </Card>
          </TabsContent>
          <TabsContent value="worker" class="p-0">
            <Card class="border-0">
              <CardHeader><CardTitle>Worker Health</CardTitle></CardHeader>
              <CardContent>
                <p class="text-muted-foreground">
                  Fleet-wide worker telemetry and historical performance panels are still shipping.
                </p>
                <GeneratingIndicator class="mt-3 inline-flex text-sm" />
              </CardContent>
            </Card>
          </TabsContent>
        </Tabs>

        <Card class="mt-4 border-0" data-testid="dashboard-control-plane-card">
          <CardHeader>
            <CardTitle>Control Plane Connectivity</CardTitle>
          </CardHeader>
          <CardContent class="grid gap-2 text-sm md:grid-cols-2">
            <p>Control Base: <span class="font-mono" data-control-field="controlBase">loading...</span></p>
            <p>Control WS: <span class="font-mono" data-control-field="controlWs">loading...</span></p>
            <p>Latency: <span class="font-mono" data-control-field="controlWsLatency">loading...</span></p>
            <p>Last Check: <span class="font-mono" data-control-field="controlCheckedAt">loading...</span></p>
          </CardContent>
        </Card>
      </section>
    </PageShell>
  )
}
