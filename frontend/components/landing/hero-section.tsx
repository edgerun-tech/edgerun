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
          <h1 class="text-balance text-5xl font-bold leading-tight md:text-6xl lg:text-7xl">Dependable Compute.<br />Financially Enforced.</h1>
          <p class="mx-auto max-w-3xl text-balance text-xl leading-relaxed text-muted-foreground md:text-2xl">Security by stake. Pricing by competition.</p>
          <p class="mx-auto max-w-3xl text-sm leading-relaxed text-muted-foreground" data-deployment-detail>
            Cluster availability is derived from deployed on-chain program IDs.
          </p>
          <div class="flex flex-col items-center justify-center gap-4 sm:flex-row">
            <a href="/docs/getting-started/quick-start/"><Button size="lg">Open Get Started Guide</Button></a>
            <a href="/docs/"><Button size="lg" variant="outline">Read Documentation</Button></a>
          </div>
          <div class="grid max-w-2xl grid-cols-3 gap-8 pt-10 mx-auto">
            <div class="space-y-2">
              <p class="text-3xl font-bold md:text-4xl">99.9%</p>
              <p class="text-sm text-muted-foreground">Consensus Rate</p>
            </div>
            <div class="space-y-2">
              <p class="text-3xl font-bold md:text-4xl">&lt;150ms</p>
              <p class="text-sm text-muted-foreground">Avg Settlement</p>
            </div>
            <div class="space-y-2">
              <p class="text-3xl font-bold md:text-4xl">2.5M+</p>
              <p class="text-sm text-muted-foreground">Jobs Executed</p>
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}
