// SPDX-License-Identifier: Apache-2.0
import { For } from 'solid-js'
import { Card, CardDescription, CardHeader, CardTitle } from '../ui/card'

const features = [
  { title: 'Deterministic WASM', description: 'Multiple workers execute identical WASM bytecode. Same input always produces same output. Consensus proves correctness.', icon: '⚡' },
  { title: 'Staking Enforces Quality', description: 'Workers stake capital to participate. Incorrect outputs result in slashed stake. Financial consequences enforce honest execution.', icon: '💎' },
  { title: 'Market-Driven Pricing', description: 'Workers compete for jobs. Supply and demand determine fees. Capital flows to the most efficient operators.', icon: '📊' },
  { title: 'Cryptographic Settlement', description: 'Execution proofs settle on Solana with immutable records and deterministic payout accounting.', icon: '🔐' },
  { title: 'Slashing Protection', description: 'Redundant execution prevents single points of failure. Malicious behavior is penalized on-chain.', icon: '⚔️' },
  { title: 'Universal WASM', description: 'Compile from Rust, C, Go, and more into deterministic WASM jobs.', icon: '🛠️' }
]

export function FeaturesSection() {
  return (
    <section class="bg-card/50 py-20">
      <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div class="mx-auto mb-16 max-w-3xl text-center">
          <h2 class="mb-4 text-balance text-3xl font-bold md:text-4xl">How It Works</h2>
          <p class="text-lg text-muted-foreground">Redundant execution by staked workers. Incorrect results lose money.</p>
        </div>
        <div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
          <For each={features}>{(feature: (typeof features)[number]) => (
            <Card class="transition-colors hover:border-primary/50">
              <CardHeader>
                <div class="mb-4 text-3xl">{feature.icon}</div>
                <CardTitle class="text-xl">{feature.title}</CardTitle>
                <CardDescription class="text-base leading-relaxed">{feature.description}</CardDescription>
              </CardHeader>
            </Card>
          )}</For>
        </div>
      </div>
    </section>
  )
}
