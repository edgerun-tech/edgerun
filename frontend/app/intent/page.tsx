// SPDX-License-Identifier: Apache-2.0
import { PageShell } from '../../components/layout/page-shell'

export default function IntentPage() {
  return (
    <PageShell>
      <section class="panel p-4 md:p-5">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <div>
            <p class="pill">Intent UI</p>
            <h1 class="mt-2 text-xl font-semibold">Unified assistant workspace</h1>
            <p class="mt-1 text-sm text-muted-foreground">
              Embedded migrated Intent UI. Open standalone if you need a dedicated tab.
            </p>
          </div>
          <a
            href="/intent-ui/"
            class="inline-flex h-9 items-center rounded-md border border-border px-3 text-sm hover:bg-muted/50"
            target="_blank"
            rel="noopener noreferrer"
          >
            Open standalone
          </a>
        </div>
        <div class="mt-4 h-[72vh] min-h-[540px] overflow-hidden rounded-lg border border-border bg-black/35">
          <iframe
            title="Intent UI"
            src="/intent-ui/"
            class="h-full w-full border-0"
            loading="lazy"
            referrerPolicy="same-origin"
          />
        </div>
      </section>
    </PageShell>
  )
}
