import { Button } from '../ui/button'
import { Card } from '../ui/card'

export function CTASection() {
  return (
    <section class="bg-background py-20">
      <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <Card class="border-primary/20 bg-gradient-to-r from-primary/10 via-accent/10 to-primary/10 p-12">
          <div class="mx-auto max-w-3xl space-y-8 text-center">
            <h2 class="text-balance text-3xl font-bold md:text-4xl">Ready to Start Running Verifiable Compute?</h2>
            <p class="text-lg text-muted-foreground">Follow the Get Started guide to run the complete workflow with clear security tradeoffs and deployment steps.</p>
            <div class="flex flex-col items-center justify-center gap-4 sm:flex-row">
              <a href="/docs/getting-started/quick-start/"><Button size="lg">Get Started</Button></a>
              <a href="/workers/"><Button size="lg" variant="outline">Become a Worker</Button></a>
            </div>
          </div>
        </Card>
      </div>
    </section>
  )
}
