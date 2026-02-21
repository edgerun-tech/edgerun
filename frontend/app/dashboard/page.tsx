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
        description="Core network metrics are fetched directly from Solana RPC. No mocked chain views."
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
      </section>
    </PageShell>
  )
}
