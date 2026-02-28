// SPDX-License-Identifier: Apache-2.0
import { For, createSignal, onCleanup, onMount } from 'solid-js'
import { Button } from './ui/button'
import { Sheet, SheetClose, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from './ui/sheet'
import { PersonalizationMenu } from './personalization-menu'
import {
  ensureTerminalDrawerStore,
  getTerminalDrawerState,
  subscribeTerminalDrawer,
  terminalDrawerActions
} from '../lib/terminal-drawer-store'

const navLinks = [
  { href: '/', label: 'Home' },
  { href: '/run/', label: 'Run Job' },
  { href: '/workers/', label: 'Workers' },
  { href: '/devices/', label: 'Devices' },
  { href: '/token/', label: 'Economics' },
  { href: '/docs/', label: 'Docs' },
  { href: '/blog/', label: 'Blog' },
  { href: '/intent/', label: 'Intent' }
]

export function Nav() {
  const [mobileOpen, setMobileOpen] = createSignal(false)
  const [terminalOpen, setTerminalOpen] = createSignal(getTerminalDrawerState().open)
  ensureTerminalDrawerStore()

  onMount(() => {
    setTerminalOpen(getTerminalDrawerState().open)
    const unsubscribe = subscribeTerminalDrawer((next) => {
      setTerminalOpen(next.open)
    })
    onCleanup(() => {
      unsubscribe()
    })
  })

  return (
    <nav class="border-b border-border bg-background/95 backdrop-blur sticky top-0 z-50">
      <div class="h-16 w-full px-4 sm:px-6 lg:px-8">
        <div class="grid h-full grid-cols-[auto_1fr_auto] items-center gap-3">
          <a href="/" class="flex items-center gap-2">
            <img src="/brand/edgerun-mark.svg" alt="Edgerun mark" width="32" height="32" />
            <span class="text-xl font-bold">Edgerun</span>
          </a>

          <div class="hidden items-center gap-1 md:flex">
            <For each={navLinks}>{(link: (typeof navLinks)[number]) => (
              <a href={link.href} data-nav-link class="rounded-md px-3 py-2 text-sm font-medium text-muted-foreground hover:bg-muted/50 hover:text-foreground">
                {link.label}
              </a>
            )}</For>
          </div>

          <div class="flex items-center justify-end gap-2">
            <Button
              variant="outline"
              size="sm"
              class="md:hidden"
              aria-expanded={mobileOpen()}
              aria-controls="mobile-nav"
              onClick={() => setMobileOpen((v) => !v)}
            >
              {mobileOpen() ? 'Close' : 'Menu'}
            </Button>
            <Button
              variant={terminalOpen() ? 'default' : 'outline'}
              size="sm"
              class="h-9 w-9 p-0"
              aria-label={terminalOpen() ? 'Close terminal drawer' : 'Open terminal drawer'}
              aria-controls="edgerun-terminal-drawer"
              aria-expanded={terminalOpen()}
              aria-pressed={terminalOpen()}
              title={terminalOpen() ? 'Close terminal' : 'Open terminal'}
              onClick={() => {
                terminalDrawerActions.toggle()
              }}
            >
              <svg viewBox="0 0 24 24" aria-hidden="true" class={`h-4 w-4 ${terminalOpen() ? 'text-primary-foreground' : 'text-foreground'}`}>
                <path fill="currentColor" d="M4 4h16a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2Zm0 2v9h16V6H4Zm1 14h14v2H5v-2Zm2-10 3 2-3 2v-4Zm5 3h5v1h-5v-1Z" />
              </svg>
            </Button>
            <PersonalizationMenu />
            <a href="/dashboard/" class="hidden sm:inline-flex"><Button variant="outline" size="sm">Dashboard</Button></a>
          </div>
        </div>
      </div>

      <Sheet open={mobileOpen()} onOpenChange={setMobileOpen}>
        <SheetTrigger class="hidden" aria-hidden="true" />
        <SheetContent class="md:hidden">
          <SheetHeader>
            <SheetTitle>Menu</SheetTitle>
            <SheetClose class="rounded-md border border-border px-2 py-1 text-xs text-muted-foreground hover:bg-muted/50">Close</SheetClose>
          </SheetHeader>
          <div id="mobile-nav" class="space-y-2">
            <div class="grid grid-cols-2 gap-2">
              <For each={navLinks}>{(link: (typeof navLinks)[number]) => (
                <a href={link.href} data-nav-link onClick={() => setMobileOpen(false)} class="rounded-md px-3 py-2 text-sm font-medium text-muted-foreground hover:bg-muted/50 hover:text-foreground">
                  {link.label}
                </a>
              )}</For>
            </div>
          </div>
        </SheetContent>
      </Sheet>
    </nav>
  )
}
