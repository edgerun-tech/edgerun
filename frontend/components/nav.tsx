import { createSignal, For } from 'solid-js'
import { Button } from './ui/button'
import { WalletButton } from './solana/wallet-button'
import { Sheet, SheetClose, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from './ui/sheet'

const navLinks = [
  { href: '/', label: 'Home' },
  { href: '/run/', label: 'Run Job' },
  { href: '/workers/', label: 'Workers' },
  { href: '/token/', label: 'Economics' },
  { href: '/docs/', label: 'Docs' },
  { href: '/blog/', label: 'Blog' }
]

export function Nav() {
  const [mobileOpen, setMobileOpen] = createSignal(false)

  return (
    <nav class="border-b border-border bg-background/95 backdrop-blur sticky top-0 z-50">
      <div class="mx-auto h-16 max-w-7xl px-4 sm:px-6 lg:px-8">
        <div class="flex h-full items-center justify-between gap-3">
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

          <div class="flex items-center gap-2">
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
            <a href="/dashboard/" class="hidden sm:inline-flex"><Button variant="outline" size="sm">Dashboard</Button></a>
            <div class="hidden sm:block"><WalletButton /></div>
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
            <div class="pt-2"><WalletButton /></div>
          </div>
        </SheetContent>
      </Sheet>
    </nav>
  )
}
