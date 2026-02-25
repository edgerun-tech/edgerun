// SPDX-License-Identifier: Apache-2.0
import { Button } from '../ui/button'

export function HeroSection() {
  return (
    <section class="bg-background py-20 md:py-32">
      <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div class="mx-auto max-w-4xl space-y-8 text-center">
          <div class="inline-flex items-center gap-2 rounded-full border border-primary/20 bg-primary/10 px-4 py-2">
            <div class="h-2 w-2 animate-pulse rounded-full bg-primary" />
            <span class="text-sm font-medium text-primary" data-deployment-badge>Live on Localnet</span>
          </div>
          <h1 class="text-balance text-5xl font-bold leading-tight md:text-6xl lg:text-7xl">Dependable Compute.<br />Financially Enforced.<br />Independently verifiable.</h1>
          <p class="mx-auto max-w-3xl text-balance text-xl leading-relaxed text-muted-foreground md:text-2xl">Security by stake. Pricing by deterministic work.</p>
          <p class="mx-auto max-w-3xl text-sm leading-relaxed text-muted-foreground" data-deployment-detail>
            Cluster availability is derived from deployed on-chain program IDs.
          </p>
          <div class="flex flex-col items-center justify-center gap-4 sm:flex-row">
            <a href="/run/"><Button size="lg">Run a Job</Button></a>
            <a href="/workers/"><Button size="lg" variant="outline">Become a Worker</Button></a>
          </div>
          <div
            class="mx-auto mt-8 w-full max-w-4xl rounded-2xl border border-primary/20 bg-gradient-to-br from-primary/10 via-background to-background p-4 md:p-6"
            data-testid="hero-chain-snapshot"
          >
            <div class="mb-4 flex items-center justify-between gap-2">
              <p class="text-left text-sm font-semibold text-foreground">Live Chain Snapshot</p>
              <p class="text-left text-xs text-muted-foreground">Derived from active RPC session</p>
            </div>
            <div class="grid gap-3 text-left sm:grid-cols-2 lg:grid-cols-4">
              <div class="rounded-xl border border-border/70 bg-background/80 p-3">
                <p class="text-xs uppercase tracking-wide text-muted-foreground">Cluster</p>
                <p class="mt-1 font-mono text-sm" data-chain-field="cluster">loading...</p>
              </div>
              <div class="rounded-xl border border-border/70 bg-background/80 p-3">
                <p class="text-xs uppercase tracking-wide text-muted-foreground">Slot</p>
                <p class="mt-1 font-mono text-sm" data-chain-field="slot">loading...</p>
              </div>
              <div class="rounded-xl border border-border/70 bg-background/80 p-3">
                <p class="text-xs uppercase tracking-wide text-muted-foreground">TPS</p>
                <p class="mt-1 font-mono text-sm" data-chain-field="tps">loading...</p>
              </div>
              <div class="rounded-xl border border-border/70 bg-background/80 p-3">
                <p class="text-xs uppercase tracking-wide text-muted-foreground">Block Height</p>
                <p class="mt-1 font-mono text-sm" data-chain-field="blockHeight">loading...</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}
