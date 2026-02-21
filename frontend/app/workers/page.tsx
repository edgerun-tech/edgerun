import { Button } from '../../components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/card'
import { GeneratingIndicator } from '../../components/ui/generating-indicator'
import { PageHero } from '../../components/layout/page-hero'
import { PageShell } from '../../components/layout/page-shell'
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from '../../components/ui/accordion'

export default function WorkersPage() {
  return (
    <PageShell>
      <PageHero
        title="Stake Capital. Execute Jobs. Earn Fees."
        badge="Registration Live"
        description="Run deterministic compute, participate in verification, and earn SOL-based payouts."
        actions={
          <>
            <a href="/docs/main/scheduler-api.html"><Button variant="outline">Read Worker Endpoints</Button></a>
            <Button disabled>
              Fleet Health Console
              <GeneratingIndicator class="ml-2 text-[10px]" />
            </Button>
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
            <AccordionContent value="stake">Workers commit SOL-denominated stake and expose deterministic runtime capacity for scheduler selection.</AccordionContent>
          </AccordionItem>
          <AccordionItem value="execute">
            <AccordionTrigger value="execute">Execute & Attest</AccordionTrigger>
            <AccordionContent value="execute">Compute tasks are redundantly executed and cross-checked. Workers submit output + verification metadata.</AccordionContent>
          </AccordionItem>
          <AccordionItem value="settle">
            <AccordionTrigger value="settle">Settle on Solana</AccordionTrigger>
            <AccordionContent value="settle">Payouts and penalties follow on-chain settlement paths. Advanced fleet tooling panels are still generating.</AccordionContent>
          </AccordionItem>
        </Accordion>

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
              <p class="font-mono text-sm text-muted-foreground">$ edgerun-worker register --stake 50 --rpc $SOLANA_RPC_URL</p>
            </CardContent>
          </Card>
        </div>

        <h2 class="mb-4 text-2xl font-bold">Top Workers</h2>
        <div class="grid gap-4 md:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle>Leaderboard</CardTitle>
              <CardDescription>Ranked performance table and fee competitiveness index.</CardDescription>
            </CardHeader>
            <CardContent>
              <GeneratingIndicator class="text-sm" />
            </CardContent>
          </Card>
        </div>
      </section>
    </PageShell>
  )
}
