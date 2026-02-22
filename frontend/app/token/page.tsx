// SPDX-License-Identifier: Apache-2.0
import { Card, CardContent, CardHeader } from '../../components/ui/card'
import { Button } from '../../components/ui/button'
import { GeneratingIndicator } from '../../components/ui/generating-indicator'
import { PageHero } from '../../components/layout/page-hero'
import { PageShell } from '../../components/layout/page-shell'

export default function TokenPage() {
  return (
    <PageShell>
      <section class="relative overflow-hidden bg-background">
        <div class="absolute inset-0 bg-gradient-to-br from-primary/5 via-transparent to-accent/5 opacity-50" />
        <div class="relative">
          <PageHero title="SOL Economics" badge="Live" description="Escrow, staking, and payouts are SOL-denominated in the current protocol design." />
        </div>
      </section>

      <section class="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        <div class="mb-8 grid gap-4 md:grid-cols-4">
          <Card><CardHeader class="pb-2"><p class="text-xs text-muted-foreground">CLUSTER</p></CardHeader><CardContent><p class="text-xl font-bold" data-chain-field="cluster">loading...</p></CardContent></Card>
          <Card><CardHeader class="pb-2"><p class="text-xs text-muted-foreground">SOL SUPPLY</p></CardHeader><CardContent><p class="text-xl font-bold" data-chain-field="supplySol">loading...</p></CardContent></Card>
          <Card><CardHeader class="pb-2"><p class="text-xs text-muted-foreground">TREASURY</p></CardHeader><CardContent><p class="text-xl font-bold" data-chain-field="treasurySol">loading...</p></CardContent></Card>
          <Card><CardHeader class="pb-2"><p class="text-xs text-muted-foreground">RPC</p></CardHeader><CardContent><p class="truncate text-sm" data-chain-field="rpcUrl">loading...</p></CardContent></Card>
        </div>

        <div class="flex flex-wrap gap-3">
          <Button disabled class="grayscale"><span>Acquire SOL</span><GeneratingIndicator class="ml-2 text-[9px]" /></Button>
          <Button variant="outline" disabled class="grayscale"><span>Stake SOL</span><GeneratingIndicator class="ml-2 text-[9px]" /></Button>
        </div>

        <h2 class="mt-10 text-2xl font-bold mb-4">Staking Pools</h2>
        <div class="grid gap-4 md:grid-cols-3">
          <Card>
            <CardHeader class="pb-2"><p class="text-xs text-muted-foreground">POOL A</p></CardHeader>
            <CardContent><GeneratingIndicator class="text-sm" /></CardContent>
          </Card>
          <Card>
            <CardHeader class="pb-2"><p class="text-xs text-muted-foreground">POOL B</p></CardHeader>
            <CardContent><GeneratingIndicator class="text-sm" /></CardContent>
          </Card>
          <Card>
            <CardHeader class="pb-2"><p class="text-xs text-muted-foreground">POOL C</p></CardHeader>
            <CardContent><GeneratingIndicator class="text-sm" /></CardContent>
          </Card>
        </div>

        <Card class="mt-8 border-primary/30 bg-card/80 backdrop-blur-sm">
          <CardContent class="p-6">
            <p class="mb-3 text-xs uppercase tracking-wide text-muted-foreground">Reference</p>
            <blockquote class="text-balance text-lg font-medium leading-relaxed md:text-xl">
              "I think compute will be the currency of the future. I think it'll be maybe the most precious commodity in the world."
            </blockquote>
            <div class="mt-4 text-sm text-muted-foreground">
              <span>Sam Altman on the Lex Fridman Podcast. </span>
              <a href="https://www.youtube.com/watch?v=jvqFAi7vkBc&t=2s" target="_blank" rel="noreferrer" class="text-primary hover:underline">Watch source</a>
            </div>
          </CardContent>
        </Card>
      </section>
    </PageShell>
  )
}
