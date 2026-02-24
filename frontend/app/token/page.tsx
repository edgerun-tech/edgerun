// SPDX-License-Identifier: Apache-2.0
import { For } from 'solid-js'
import { Card, CardContent, CardHeader } from '../../components/ui/card'
import { Button } from '../../components/ui/button'
import { PageHero } from '../../components/layout/page-hero'
import { PageShell } from '../../components/layout/page-shell'
import {
  COMMITTEE_TIERS,
  PROGRAM_DEFAULT_PROTOCOL_FEE_BPS,
  RUN_DEFAULT_ESCROW_FLOOR_LAMPORTS,
  SCHEDULER_DEFAULT_FLAT_FEE_LAMPORTS,
  SCHEDULER_DEFAULT_LAMPORTS_PER_BILLION_INSTRUCTIONS,
  SCHEDULER_DEFAULT_REDUNDANCY_MULTIPLIER,
  computeFinalizePayouts,
  lamportsToSol,
  requiredInstructionEscrowLamports,
  requiredLockForJobLamports
} from '../../lib/economics'

const DEMO_MAX_INSTRUCTIONS = 10_000
const demoMinEscrowLamports = requiredInstructionEscrowLamports(
  DEMO_MAX_INSTRUCTIONS,
  SCHEDULER_DEFAULT_LAMPORTS_PER_BILLION_INSTRUCTIONS,
  SCHEDULER_DEFAULT_REDUNDANCY_MULTIPLIER,
  SCHEDULER_DEFAULT_FLAT_FEE_LAMPORTS
)
const demoEscrowLamports = Math.max(RUN_DEFAULT_ESCROW_FLOOR_LAMPORTS, demoMinEscrowLamports)
const demoTier = COMMITTEE_TIERS.find((tier) => demoEscrowLamports >= tier.minEscrowLamports && (tier.maxEscrowLamportsExclusive === null || demoEscrowLamports < tier.maxEscrowLamportsExclusive)) ?? COMMITTEE_TIERS[0]!
const demoRequiredLockLamports = requiredLockForJobLamports(demoEscrowLamports, demoTier.committeeSize)
const demoPayout = computeFinalizePayouts(demoEscrowLamports, PROGRAM_DEFAULT_PROTOCOL_FEE_BPS, demoTier.quorum)

function formatSolFromLamports(lamports: number): string {
  return `${lamportsToSol(lamports).toLocaleString('en-US', { maximumFractionDigits: 6 })} SOL`
}

function formatLamports(lamports: number): string {
  return lamports.toLocaleString('en-US')
}

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

        <div class="mb-8 flex flex-wrap gap-3">
          <a href="/run/"><Button>Open Run Flow</Button></a>
          <a href="/workers/"><Button variant="outline">Open Worker Flow</Button></a>
        </div>

        <Card data-testid="economics-pricing-model">
          <CardHeader>
            <p class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">Deterministic Pricing Model</p>
          </CardHeader>
          <CardContent class="space-y-3 text-sm">
            <p class="font-mono">
              min_escrow_lamports = ceil(max_instructions * lamports_per_billion * redundancy / 1_000_000_000) + flat_fee
            </p>
            <p>
              Baseline scheduler defaults: <span class="font-mono">lamports_per_billion={formatLamports(SCHEDULER_DEFAULT_LAMPORTS_PER_BILLION_INSTRUCTIONS)}</span>, <span class="font-mono">redundancy={SCHEDULER_DEFAULT_REDUNDANCY_MULTIPLIER}</span>, <span class="font-mono">flat_fee={formatLamports(SCHEDULER_DEFAULT_FLAT_FEE_LAMPORTS)}</span>.
            </p>
            <p>
              Example envelope (`max_instructions={formatLamports(DEMO_MAX_INSTRUCTIONS)}`) deterministic minimum:
              <span class="ml-2 font-mono">{formatLamports(demoMinEscrowLamports)} lamports</span>
              <span class="ml-2 text-muted-foreground">({formatSolFromLamports(demoMinEscrowLamports)})</span>
            </p>
          </CardContent>
        </Card>

        <Card class="mt-6" data-testid="economics-committee-tiers">
          <CardHeader>
            <p class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">Committee Tiering by Escrow</p>
          </CardHeader>
          <CardContent>
            <div class="overflow-x-auto">
              <table class="w-full text-left text-sm">
                <thead>
                  <tr class="border-b border-border/80 text-xs uppercase tracking-wide text-muted-foreground">
                    <th class="py-2 pr-3">Tier</th>
                    <th class="py-2 pr-3">Escrow Range</th>
                    <th class="py-2 pr-3">Committee</th>
                    <th class="py-2 pr-3">Quorum</th>
                  </tr>
                </thead>
                <tbody>
                  <For each={COMMITTEE_TIERS}>{(tier) => (
                    <tr class="border-b border-border/40">
                      <td class="py-2 pr-3 font-medium">{tier.name}</td>
                      <td class="py-2 pr-3 font-mono">
                        {tier.maxEscrowLamportsExclusive === null
                          ? `>= ${formatLamports(tier.minEscrowLamports)}`
                          : `${formatLamports(tier.minEscrowLamports)} - ${formatLamports(tier.maxEscrowLamportsExclusive - 1)}`}
                      </td>
                      <td class="py-2 pr-3 font-mono">{tier.committeeSize}</td>
                      <td class="py-2 pr-3 font-mono">{tier.quorum}</td>
                    </tr>
                  )}</For>
                </tbody>
              </table>
            </div>
          </CardContent>
        </Card>

        <Card class="mt-6" data-testid="economics-settlement-model">
          <CardHeader>
            <p class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">Settlement and Stake Lock Model</p>
          </CardHeader>
          <CardContent class="space-y-3 text-sm">
            <p class="font-mono">
              required_lock = escrow * 3 / 2 / committee_size
            </p>
            <p class="font-mono">
              protocol_fee = escrow * protocol_fee_bps / 10_000
            </p>
            <p class="font-mono">
              payout_each = floor((escrow - protocol_fee) / winners)
            </p>
            <p>
              Example using run baseline escrow <span class="font-mono">{formatLamports(demoEscrowLamports)} lamports</span>:
            </p>
            <ul class="list-disc space-y-1 pl-6">
              <li>Tier/quorum: <span class="font-mono">{demoTier.name}</span> with committee <span class="font-mono">{demoTier.committeeSize}</span> and quorum <span class="font-mono">{demoTier.quorum}</span>.</li>
              <li>Required lock per assigned worker: <span class="font-mono">{formatLamports(demoRequiredLockLamports)} lamports</span> ({formatSolFromLamports(demoRequiredLockLamports)}).</li>
              <li>Protocol fee @ {PROGRAM_DEFAULT_PROTOCOL_FEE_BPS} bps: <span class="font-mono">{formatLamports(demoPayout.protocolFeeLamports)} lamports</span>.</li>
              <li>Payout per winner ({demoTier.quorum} winners): <span class="font-mono">{formatLamports(demoPayout.payoutEachLamports)} lamports</span>, remainder <span class="font-mono">{formatLamports(demoPayout.payoutRemainderLamports)}</span>.</li>
            </ul>
          </CardContent>
        </Card>

        <Card class="mt-6 border-border/80 bg-card/50" data-testid="economics-notes">
          <CardContent class="p-6 text-sm text-muted-foreground">
            All formulas above mirror runtime logic in scheduler/program code paths. This screen intentionally avoids speculative APR, staking pool yields, or synthetic token metrics.
          </CardContent>
        </Card>

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
