// SPDX-License-Identifier: Apache-2.0
import { PageShell } from '../../components/layout/page-shell'
import { PageHero } from '../../components/layout/page-hero'

export default function VisionPage() {
  return (
    <PageShell>
      <PageHero
        title="CloudOS Direction"
        badge="Vision"
        badgeVariant="secondary"
        description="Browser-first control, device capability routing, and wallet-anchored user ownership inform the Edgerun product direction."
      />
      <section class="mx-auto max-w-5xl px-4 py-10 sm:px-6 lg:px-8">
        <div class="grid gap-6 md:grid-cols-2">
          <article class="rounded-lg border border-border bg-card p-5">
            <h2 class="text-xl font-semibold">Core Model</h2>
            <ul class="mt-3 space-y-2 text-sm text-muted-foreground">
              <li>Browser-first control plane for a unified command surface.</li>
              <li>Device capability plane via local agents (compute, files, sensors, services).</li>
              <li>Workflow execution across local/remote devices through explicit routing.</li>
              <li>Integration plane for cloud/SaaS domains (storage, comms, infrastructure).</li>
            </ul>
          </article>
          <article class="rounded-lg border border-border bg-card p-5">
            <h2 class="text-xl font-semibold">Identity and Ownership</h2>
            <ul class="mt-3 space-y-2 text-sm text-muted-foreground">
              <li>Wallet-anchored identity and cryptographic data ownership.</li>
              <li>Bring-your-own storage backends with encrypted profile/state.</li>
              <li>Session access requires wallet auth plus authorized storage access.</li>
            </ul>
          </article>
          <article class="rounded-lg border border-border bg-card p-5 md:col-span-2">
            <h2 class="text-xl font-semibold">Resilience and Economic Layer</h2>
            <p class="mt-3 text-sm text-muted-foreground">
              Connected devices increase redundancy, synchronize encrypted state, and can optionally contribute execution capacity.
              Scheduling and policy constraints determine how external workloads are accepted and routed.
            </p>
            <p class="mt-3 text-sm text-muted-foreground">
              Full archived direction text: <a class="text-foreground underline" href="/docs/main/cloud-os-direction/">CloudOS Direction</a>
            </p>
          </article>
        </div>
      </section>
    </PageShell>
  )
}
