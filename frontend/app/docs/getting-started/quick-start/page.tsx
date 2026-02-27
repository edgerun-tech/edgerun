// SPDX-License-Identifier: Apache-2.0
import { createSignal, For } from 'solid-js'
import { Nav } from '../../../../components/nav'
import { Footer } from '../../../../components/footer'
import { DocsSidebar } from '../../../../components/docs/docs-sidebar'
import { Card, CardContent, CardHeader, CardTitle } from '../../../../components/ui/card'
import { PageHero } from '../../../../components/layout/page-hero'
import { Alert, AlertDescription, AlertTitle } from '../../../../components/ui/alert'
import { Button } from '../../../../components/ui/button'
import { Sheet, SheetClose, SheetContent, SheetHeader, SheetTitle } from '../../../../components/ui/sheet'
import { getDocsNav } from '../../../../lib/docs-nav'

const docsNav = getDocsNav('main')

function CodeBlock(props: { code: string; lang?: string }) {
  return (
    <pre class="overflow-x-auto rounded-lg border border-border bg-black/40 p-4">
      <code class={`font-mono text-xs text-foreground ${props.lang ? `language-${props.lang}` : ''}`}>{props.code}</code>
    </pre>
  )
}

export default function QuickStartPage() {
  const [mobileNavOpen, setMobileNavOpen] = createSignal(false)

  return (
    <div class="flex min-h-screen flex-col">
      <Nav />
      <main class="flex-1 bg-background">
        <div class="mx-auto grid w-full max-w-7xl gap-6 px-4 py-6 sm:px-6 lg:grid-cols-[240px_minmax(0,1fr)] lg:px-8">
          <DocsSidebar version="main" showSearch class="hidden lg:block h-fit rounded-lg border border-border bg-card/50 p-4" />
          <section class="rounded-xl border border-border bg-card/40 overflow-hidden">
            <div class="border-b border-border px-4 py-3 lg:hidden">
              <div class="flex items-center justify-between gap-3">
                <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Documentation</p>
                <Button variant="outline" size="sm" onClick={() => setMobileNavOpen(true)}>Browse</Button>
              </div>
            </div>

            <PageHero
              title="Quick Start"
              badge="Get Started"
              description="Run the end-to-end starter workflow in minutes using the guided Run Job flow and scheduler control surface."
              maxWidthClass="max-w-4xl"
            />

            <section class="mx-auto max-w-4xl px-4 py-8 sm:px-6 lg:px-8">
              <div class="space-y-8">
                <Card>
                  <CardHeader><CardTitle>1. Validate Frontend Tooling</CardTitle></CardHeader>
                  <CardContent class="space-y-3">
                    <CodeBlock lang="bash" code={`cd frontend\nbun run check\nbun run build`} />
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader><CardTitle>2. Open Guided Run Flow</CardTitle></CardHeader>
                  <CardContent class="space-y-3">
                    <CodeBlock
                      lang="text"
                      code={`Open /run/\nChoose a preset module\nSet scheduler URL and compute envelope\nReview contract, then submit`}
                    />
                    <Alert>
                      <AlertTitle>Safety model</AlertTitle>
                      <AlertDescription>Distributed mode requires explicit seed exposure acknowledgement before submission.</AlertDescription>
                    </Alert>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader><CardTitle>3. Use Deterministic JSON Payload</CardTitle></CardHeader>
                  <CardContent class="space-y-3">
                    <CodeBlock
                      lang="json"
                      code={`{
  "document": { "status": "pending" },
  "rules": [{ "op": "set", "path": "status", "value": "ready" }]
}`}
                    />
                  </CardContent>
                </Card>
              </div>
            </section>
          </section>
        </div>
      </main>
      <Footer />

      <Sheet open={mobileNavOpen()} onOpenChange={setMobileNavOpen}>
        <SheetContent class="lg:hidden">
          <SheetHeader>
            <SheetTitle>Docs Navigation</SheetTitle>
            <SheetClose class="rounded-md border border-border px-2 py-1 text-xs text-muted-foreground hover:bg-muted/50">Close</SheetClose>
          </SheetHeader>
          <nav class="space-y-2 text-sm">
            <For each={docsNav}>{(item) => (
              <a
                href={item.href}
                data-nav-link
                onClick={() => setMobileNavOpen(false)}
                class="block rounded border border-border bg-card px-3 py-2 text-muted-foreground hover:bg-muted/50 hover:text-foreground"
              >
                {item.label}
              </a>
            )}</For>
          </nav>
        </SheetContent>
      </Sheet>
    </div>
  )
}
