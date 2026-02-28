// SPDX-License-Identifier: Apache-2.0
import { createSignal, For } from 'solid-js'
import { Nav } from '../../components/nav'
import { Footer } from '../../components/footer'
import { DocsSidebar } from '../../components/docs/docs-sidebar'
import { Badge } from '../../components/ui/badge'
import { Button } from '../../components/ui/button'
import { Card, CardDescription, CardHeader, CardTitle } from '../../components/ui/card'
import { GeneratingIndicator } from '../../components/ui/generating-indicator'
import { PageHero } from '../../components/layout/page-hero'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../../components/ui/tabs'
import { Sheet, SheetClose, SheetContent, SheetHeader, SheetTitle } from '../../components/ui/sheet'
import {
  docsApiReferenceHref,
  docsChangelogHref,
  docsQuickStartHref,
  docsLeafPrettyHref
} from '../../lib/docs-links'
import { getDocsNav } from '../../lib/docs-nav'

const quickLinks = [
  { title: 'Get Started Guide', description: 'Step-by-step onboarding with local and distributed execution modes', href: docsQuickStartHref(), live: true },
  { title: 'API Reference', description: 'Generated HTTP, Rust, and CLI references from source', href: docsApiReferenceHref('main'), live: true },
  { title: 'Changelog', description: 'Auto-generated release log with commit dates and diff links', href: docsChangelogHref('main'), live: true },
  { title: 'Routed Terminal Protocol v2', description: 'Current wire contract for routed terminal sessions over multi-hop WebRTC overlays', href: docsLeafPrettyHref('main', 'routed-terminal-protocol-v2'), live: true }
]

const docsNav = getDocsNav('main')

export default function DocsHomePage() {
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
              title="Documentation"
              badge="Versioned"
              badgeVariant="secondary"
              description="Everything you need to build with Edgerun."
              maxWidthClass="max-w-4xl"
            />
            <div class="mx-auto max-w-4xl px-4 py-10 sm:px-6 lg:px-8 sm:py-12">
              <div class="mb-12 grid gap-6 md:grid-cols-2 xl:grid-cols-3">
                <For each={quickLinks}>{(link: (typeof quickLinks)[number]) => (
                  <a href={link.href}>
                    <Card class="h-full cursor-pointer transition-colors hover:border-primary/50">
                      <CardHeader>
                        <CardTitle class="flex items-center justify-between gap-2 text-lg">
                          <span>{link.title}</span>
                          {link.live ? <Badge>Live</Badge> : <GeneratingIndicator class="text-[10px]" />}
                        </CardTitle>
                        <CardDescription>{link.description}</CardDescription>
                      </CardHeader>
                    </Card>
                  </a>
                )}</For>
              </div>

              <section>
                <h2 class="mb-4 text-2xl font-bold">Getting Started</h2>
                <Tabs defaultValue="workflow">
                  <TabsList>
                    <TabsTrigger value="workflow">Get Started</TabsTrigger>
                    <TabsTrigger value="api">Scheduler API</TabsTrigger>
                    <TabsTrigger value="protocol">Protocol</TabsTrigger>
                  </TabsList>
                  <TabsContent value="workflow">
                    <CardHeader class="p-0">
                      <CardTitle>Recommended First Run</CardTitle>
                      <CardDescription>Start with the Get Started guide, then compare secure-local and distributed execution paths.</CardDescription>
                    </CardHeader>
                  </TabsContent>
                  <TabsContent value="api">
                    <CardHeader class="p-0">
                      <CardTitle>Generated Endpoint Surface</CardTitle>
                      <CardDescription>Use the generated scheduler API snapshot to trace each request in the onboarding flow end to end.</CardDescription>
                    </CardHeader>
                  </TabsContent>
                  <TabsContent value="protocol">
                    <CardHeader class="p-0">
                      <CardTitle>Economic + Verification Model</CardTitle>
                      <CardDescription>Map worker staking, deterministic execution, and attestation paths from source docs.</CardDescription>
                    </CardHeader>
                  </TabsContent>
                </Tabs>
              </section>
            </div>
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
