// SPDX-License-Identifier: Apache-2.0
import { For, createSignal, onCleanup, onMount } from 'solid-js'
import { Button } from './ui/button'
import { WalletButton } from './solana/wallet-button'
import { Sheet, SheetClose, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from './ui/sheet'
import { PersonalizationMenu } from './personalization-menu'
import {
  ensureTerminalDrawerStore,
  getTerminalDrawerState,
  subscribeTerminalDrawer,
  type TerminalDrawerState,
  terminalDrawerActions
} from '../lib/terminal-drawer-store'
import { getWebRtcPeerSupervisor, initWebRtcPeerSupervisor } from '../lib/webrtc-peer-supervisor'
import {
  getCachedRouteControlStatus,
  getRouteControlBaseSelection,
  probeRouteControlStatus,
  type RouteControlProbeStatus,
  type RouteControlSource
} from '../lib/webrtc-route-client'
import { JOB_TAB_STATUS_EVENT, type JobTabStatus } from '../lib/tab-job-status'
import { WALLET_SESSION_EVENT, readWalletSession, type WalletSessionState } from '../lib/wallet-session'

const navLinks = [
  { href: '/', label: 'Home' },
  { href: '/run/', label: 'Run Job' },
  { href: '/workers/', label: 'Workers' },
  { href: '/token/', label: 'Economics' },
  { href: '/docs/', label: 'Docs' },
  { href: '/blog/', label: 'Blog' }
]

export function Nav() {
  const ROUTE_ADVERT_STALE_MS = 15000
  const [mobileOpen, setMobileOpen] = createSignal(false)
  const [terminalOpen, setTerminalOpen] = createSignal(getTerminalDrawerState().open)
  const [onlineNodes, setOnlineNodes] = createSignal(0)
  const [totalNodes, setTotalNodes] = createSignal(0)
  const [activeWorkers, setActiveWorkers] = createSignal(0)
  const [walletConnected, setWalletConnected] = createSignal(readWalletSession().connected)
  const [schedulerBase, setSchedulerBase] = createSignal('')
  const [schedulerSource, setSchedulerSource] = createSignal<RouteControlSource>('default')
  const [schedulerReachable, setSchedulerReachable] = createSignal(false)
  const [schedulerWsReachable, setSchedulerWsReachable] = createSignal(false)
  const [routeSignalConnected, setRouteSignalConnected] = createSignal(false)
  const [schedulerStatus, setSchedulerStatus] = createSignal<string>('')
  const [routePeers, setRoutePeers] = createSignal(0)
  const [routeEntries, setRouteEntries] = createSignal(0)
  const [routeAdvertised, setRouteAdvertised] = createSignal(false)
  const [routeAdvertAgeMs, setRouteAdvertAgeMs] = createSignal(0)
  ensureTerminalDrawerStore()

  const refreshNodeCounts = (state: TerminalDrawerState) => {
    setOnlineNodes(state.devices.filter((device) => device.status === 'online').length)
    setTotalNodes(state.devices.length)
  }

  const routeSourceLabel = (source: RouteControlSource): string => {
    if (source === 'configured') return 'cfg'
    if (source === 'storage') return 'store'
    if (source === 'local') return 'local'
    if (source === 'origin') return 'orig'
    return 'def'
  }

  const applySchedulerProbe = (status: RouteControlProbeStatus | null): void => {
    if (!status) return
    setSchedulerReachable(status.httpReachable)
    setSchedulerWsReachable(status.controlWsReachable)
    const statusParts: string[] = []
    statusParts.push(status.httpReachable ? 'scheduler online' : 'scheduler offline')
    if (status.httpStatus) statusParts.push(`HTTP ${status.httpStatus}`)
    if (status.controlWsReachable) statusParts.push('control-ws up')
    if (status.error) statusParts.push(`error: ${status.error}`)
    setSchedulerStatus(statusParts.join(' · '))
    setSchedulerBase(status.base)
    setSchedulerSource(status.source)
  }

  const refreshRouteDebug = async () => {
    const selection = getRouteControlBaseSelection()
    setSchedulerBase(selection.selected)
    setSchedulerSource(selection.source)
    const cached = getCachedRouteControlStatus()
    if (cached && cached.base === selection.selected) {
      applySchedulerProbe(cached)
    }

    const candidates = selection.candidates.length > 0
      ? selection.candidates
      : [{ base: selection.selected, source: selection.source }]
    let fallbackStatus: RouteControlProbeStatus | null = null

    for (const candidate of candidates) {
      try {
        const status = await probeRouteControlStatus(candidate.base, candidate.source)
        if (!fallbackStatus) {
          fallbackStatus = status
        }
        if (status.httpReachable || status.controlWsReachable) {
          applySchedulerProbe(status)
          return
        }
        if (!fallbackStatus.httpReachable && !fallbackStatus.controlWsReachable) {
          fallbackStatus = status
        }
      } catch {
        // keep probing until a usable candidate is found
      }
    }

    if (fallbackStatus) {
      applySchedulerProbe(fallbackStatus)
      return
    }

    setSchedulerReachable(false)
    setSchedulerWsReachable(false)
    setSchedulerStatus('scheduler probe failed')

    try {
      const status = getWebRtcPeerSupervisor().getStatus()
      setRoutePeers(status.directPeers)
      setRouteEntries(status.routeEntries)
      setRouteSignalConnected(status.controlSignalConnected)
      const latestAdvert = Math.max(status.lastAdvertBroadcastAt, status.lastRouteAdvertReceivedAt, 0)
      const advertAge = latestAdvert > 0 ? Date.now() - latestAdvert : null
      const advertFresh = advertAge !== null ? advertAge < ROUTE_ADVERT_STALE_MS : false
      setRouteAdvertAgeMs(advertAge ?? 0)
      setRouteAdvertised(advertFresh && (status.directPeers > 0 || status.controlSignalConnected))
    } catch {
      setRoutePeers(0)
      setRouteEntries(0)
      setRouteSignalConnected(false)
      setRouteAdvertised(false)
      setRouteAdvertAgeMs(0)
    }
  }

  onMount(() => {
    try {
      initWebRtcPeerSupervisor()
    } catch {
      setRouteSignalConnected(false)
      setRoutePeers(0)
      setRouteEntries(0)
      setRouteAdvertised(false)
    }
    refreshNodeCounts(getTerminalDrawerState())
    void refreshRouteDebug()
    setTerminalOpen(getTerminalDrawerState().open)
    const unsubscribe = subscribeTerminalDrawer((next) => {
      refreshNodeCounts(next)
      setTerminalOpen(next.open)
    })
    const onWalletSession = (event: Event) => {
      const custom = event as CustomEvent<WalletSessionState>
      setWalletConnected(Boolean(custom.detail?.connected))
    }
    const onJobStatus = (event: Event) => {
      const custom = event as CustomEvent<JobTabStatus | null>
      const next = custom.detail ?? null
      const workersActive = Number.isFinite(next?.workersActive ?? NaN)
        ? Math.max(0, Math.round((next?.workersActive ?? 0)))
        : 0
      setActiveWorkers(workersActive)
    }
    const routeDebugInterval = window.setInterval(() => {
      void refreshRouteDebug()
    }, 5000)
    window.addEventListener(WALLET_SESSION_EVENT, onWalletSession as EventListener)
    window.addEventListener(JOB_TAB_STATUS_EVENT, onJobStatus as EventListener)
    onCleanup(() => {
      unsubscribe()
      window.removeEventListener(WALLET_SESSION_EVENT, onWalletSession as EventListener)
      window.removeEventListener(JOB_TAB_STATUS_EVENT, onJobStatus as EventListener)
      window.clearInterval(routeDebugInterval)
    })
  })

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
            <div class="hidden items-center gap-3 rounded-md border border-border/80 bg-muted/20 px-2 py-1 text-[11px] text-muted-foreground sm:flex">
              <span data-testid="route-debug-nodes">
                <span class={`mr-1 inline-block h-1.5 w-1.5 rounded-full ${totalNodes() > 0 && onlineNodes() > 0 ? 'bg-emerald-500' : 'bg-border'}`} />
                <span class="font-mono">
                  {totalNodes() === 0 ? 'nodes: none' : `nodes ${onlineNodes()}/${totalNodes()}`}
                </span>
              </span>
              <span data-testid="route-debug-workers">
                <span class={`mr-1 inline-block h-1.5 w-1.5 rounded-full ${activeWorkers() > 0 ? 'bg-emerald-500' : 'bg-border'}`} />
                <span class="font-mono">
                  workers {activeWorkers()}
                </span>
              </span>
            </div>
            <div class="mt-1 hidden flex-wrap items-center gap-2 rounded-md border border-border/80 bg-muted/20 px-2 py-1 text-[10px] text-muted-foreground sm:flex">
              <span data-testid="route-debug-scheduler" title={`${schedulerStatus()} · ${schedulerBase()}`}>
                <span class={`mr-1 inline-block h-1.5 w-1.5 rounded-full ${schedulerReachable() ? 'bg-emerald-500' : 'bg-rose-500'}`} />
                <span class="font-mono">
                  {schedulerReachable() ? 'scheduler online' : 'scheduler offline'}{schedulerSource() ? ` (${routeSourceLabel(schedulerSource())})` : ''}
                </span>
              </span>
              <span
                data-testid="route-debug-control-ws"
                class={`font-mono ${schedulerWsReachable() ? 'text-emerald-400' : 'text-muted-foreground'}`}
              >
                ws {schedulerWsReachable() ? 'ok' : 'down'}
              </span>
              <span
                data-testid="route-debug-overlay-ws"
                class={`font-mono ${routeSignalConnected() ? 'text-emerald-400' : 'text-muted-foreground'}`}
              >
                overlay-ws {routeSignalConnected() ? 'on' : 'off'}
              </span>
              <span data-testid="route-debug-overlay-summary" class="font-mono">
                overlay {routePeers()} peers / {routeEntries()} routes
              </span>
              <span
                data-testid="route-debug-route-advert"
                class={`font-mono ${routeAdvertised() ? 'text-emerald-400' : 'text-muted-foreground'}`}
              >
                route-advert {routeAdvertised() ? `active (${routeAdvertAgeMs()}ms)` : 'idle'}
              </span>
            </div>
            <div class="flex items-center gap-1.5 rounded-md border border-border/80 bg-muted/20 px-1.5 py-0.5 text-[10px] text-muted-foreground sm:hidden">
              <span class="font-mono">{onlineNodes()}/{totalNodes()}</span>
              <span class="text-muted-foreground">•</span>
              <span class="font-mono">{activeWorkers()}</span>
            </div>
            <div class="sm:hidden mt-1 rounded-md border border-border/80 bg-muted/20 px-1.5 py-0.5 text-[9px] text-muted-foreground">
              <span class={`mr-1 inline-block h-1.5 w-1.5 rounded-full ${schedulerReachable() ? 'bg-emerald-500' : 'bg-rose-500'}`} />
              <span class="font-mono" title={schedulerBase()}>
                {schedulerReachable() ? 'sched ok' : 'sched off'}
              </span>
              <span class="mx-1">•</span>
              <span>{routePeers()}p/{routeEntries()}r</span>
              <span class="mx-1">•</span>
              <span class={`font-mono ${routeSignalConnected() ? 'text-emerald-400' : 'text-muted-foreground'}`}>
                {routeSignalConnected() ? 'sig on' : 'sig off'}
              </span>
              <span>{routeAdvertised() ? `advert ok (${routeAdvertAgeMs()}ms)` : 'advert idle'}</span>
            </div>
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
              variant={terminalOpen() && walletConnected() ? 'default' : 'outline'}
              size="sm"
              class="h-9 w-9 p-0"
              aria-label={walletConnected() ? (terminalOpen() ? 'Close terminal drawer' : 'Open terminal drawer') : 'Connect wallet to use terminal drawer'}
              aria-controls="edgerun-terminal-drawer"
              aria-expanded={walletConnected() ? terminalOpen() : false}
              aria-pressed={walletConnected() ? terminalOpen() : false}
              title={walletConnected() ? (terminalOpen() ? 'Close terminal' : 'Open terminal') : 'Connect wallet first'}
              disabled={!walletConnected()}
              onClick={() => {
                if (!walletConnected()) return
                terminalDrawerActions.toggle()
              }}
            >
              <svg viewBox="0 0 24 24" aria-hidden="true" class={`h-4 w-4 ${terminalOpen() ? 'text-primary-foreground' : 'text-foreground'}`}>
                <path fill="currentColor" d="M4 4h16a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2Zm0 2v9h16V6H4Zm1 14h14v2H5v-2Zm2-10 3 2-3 2v-4Zm5 3h5v1h-5v-1Z" />
              </svg>
            </Button>
            <PersonalizationMenu />
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
