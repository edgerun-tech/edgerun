import { For, Show, createEffect, createMemo, createSignal, onCleanup, onMount, untrack } from 'solid-js'
import QRCode from 'qrcode'
import {
  ensureTerminalDrawerStore,
  getTerminalDrawerState,
  getTerminalPaneSrc,
  subscribeTerminalDrawer,
  terminalDrawerActions,
  type DeviceStatus,
  type TerminalDrawerState,
  type TerminalSplitMode,
  type TerminalTab
} from '../../lib/terminal-drawer-store'
import { WALLET_SESSION_EVENT, readWalletSession, type WalletSessionState } from '../../lib/wallet-session'
import { canUseCurrentOriginAsDevice, importTailscaleBridgeDevices, refreshTerminalDevices } from '../../lib/terminal-device-service'

function splitClassName(tab: TerminalTab): string {
  if (tab.split === 'split-cols') return 'terminal-grid-split-cols'
  if (tab.split === 'split-rows') return 'terminal-grid-split-rows'
  return 'terminal-grid-split-none'
}

function statusBadge(status: DeviceStatus): string {
  if (status === 'online') return 'text-emerald-400'
  if (status === 'offline') return 'text-rose-400'
  return 'text-muted-foreground'
}

export function TerminalDrawer() {
  const [state, setState] = createSignal<TerminalDrawerState>(getTerminalDrawerState())
  const [dragging, setDragging] = createSignal(false)
  const [wallet, setWallet] = createSignal<WalletSessionState>(readWalletSession())
  const [deviceNameInput, setDeviceNameInput] = createSignal('')
  const [deviceUrlInput, setDeviceUrlInput] = createSignal('')
  const [qrDeviceUrl, setQrDeviceUrl] = createSignal('')
  const [qrImageDataUrl, setQrImageDataUrl] = createSignal('')
  const [tailscaleImporting, setTailscaleImporting] = createSignal(false)
  const [tailscaleImportNote, setTailscaleImportNote] = createSignal('')
  const [tabMenuTabId, setTabMenuTabId] = createSignal<string | null>(null)
  let lastAutoImportAt = 0

  const walletConnected = createMemo(() => wallet().connected)

  const activeTab = createMemo(() => {
    const current = state()
    return current.tabs.find((tab) => tab.id === current.activeTabId) ?? current.tabs[0]
  })

  const drawerHeight = createMemo(() => {
    if (!walletConnected()) return 0
    const current = state()
    if (!current.open) return 48
    return Math.round(window.innerHeight * current.heightRatio)
  })

  const splitChange = (mode: TerminalSplitMode) => {
    terminalDrawerActions.setSplit(mode)
  }

  const tabIndex = (tabId: string): number => state().tabs.findIndex((entry) => entry.id === tabId)
  const hasTabsLeft = (tabId: string): boolean => tabIndex(tabId) > 0
  const hasTabsRight = (tabId: string): boolean => {
    const idx = tabIndex(tabId)
    return idx >= 0 && idx < state().tabs.length - 1
  }
  const hasOtherTabs = (): boolean => state().tabs.length > 1

  const refreshDeviceStatus = async () => {
    const connected = untrack(() => walletConnected())
    if (!connected) return
    const devices = untrack(() => state().devices)
    await refreshTerminalDevices(
      devices,
      (id, status) => terminalDrawerActions.markDeviceStatus(id, status)
    )
  }

  const maybeRegisterCurrentOriginDevice = async () => {
    if (typeof window === 'undefined') return
    const origin = window.location.origin
    const existing = untrack(() => state().devices.some((device) => device.baseUrl === origin))
    if (existing) return

    try {
      const available = await canUseCurrentOriginAsDevice(origin)
      if (!available) return
      terminalDrawerActions.addDevice('This Laptop', origin)
      await refreshDeviceStatus()
    } catch {
      // ignore: origin is not running term-server
    }
  }

  const importTailscaleDevices = async (opts?: { silent?: boolean }) => {
    const silent = Boolean(opts?.silent)
    if (tailscaleImporting()) return
    setTailscaleImporting(true)
    if (!silent) setTailscaleImportNote('')
    const endpoints = [
      'http://127.0.0.1:49201/v1/tailscale/devices',
      'http://localhost:49201/v1/tailscale/devices'
    ] as const

    const existing = new Set(untrack(() => state().devices.map((device) => device.baseUrl)))
    const result = await importTailscaleBridgeDevices(endpoints, existing)
    if (result.error) {
      if (!silent) setTailscaleImportNote(result.error)
      setTailscaleImporting(false)
      return
    }

    for (const device of result.added) {
      terminalDrawerActions.addDevice(device.name, device.baseUrl)
    }
    await refreshDeviceStatus()
    const imported = result.added.length
    if (!silent) {
      setTailscaleImportNote(imported > 0 ? `Imported ${imported} device${imported === 1 ? '' : 's'} from Tailscale.` : 'No new Tailscale devices to import.')
    }
    setTailscaleImporting(false)
  }

  const autoImportTailscaleDevices = () => {
    if (!untrack(() => state().autoImportTailscale)) return
    const now = Date.now()
    if (now - lastAutoImportAt < 15000) return
    lastAutoImportAt = now
    void importTailscaleDevices({ silent: true })
  }

  const restoreLastDevice = () => {
    terminalDrawerActions.restoreLastDeviceOnActiveTab()
  }

  createEffect(() => {
    const target = qrDeviceUrl().trim()
    if (!target) {
      setQrImageDataUrl('')
      return
    }
    void QRCode.toDataURL(target, {
      width: 220,
      margin: 1,
      errorCorrectionLevel: 'M'
    })
      .then((dataUrl: string) => setQrImageDataUrl(dataUrl))
      .catch(() => setQrImageDataUrl(''))
  })

  createEffect(() => {
    if (typeof document === 'undefined') return
    if (!walletConnected()) {
      document.documentElement.style.removeProperty('--terminal-drawer-height')
      return
    }
    document.documentElement.style.setProperty('--terminal-drawer-height', `${drawerHeight()}px`)
  })

  onMount(() => {
    ensureTerminalDrawerStore()
    setState(getTerminalDrawerState())
    const initialWallet = readWalletSession()
    setWallet(initialWallet)
    void maybeRegisterCurrentOriginDevice()
    if (initialWallet.connected) {
      restoreLastDevice()
      autoImportTailscaleDevices()
    }

    const unsubscribe = subscribeTerminalDrawer((next) => setState(next))

    const onWalletSession = (event: Event) => {
      const custom = event as CustomEvent<WalletSessionState>
      const nextWallet = custom.detail || readWalletSession()
      setWallet(nextWallet)
      if (nextWallet.connected) {
        void maybeRegisterCurrentOriginDevice()
        autoImportTailscaleDevices()
        restoreLastDevice()
        void refreshDeviceStatus()
      }
    }

    const onPointerMove = (ev: PointerEvent) => {
      if (!dragging() || !walletConnected()) return
      const minPx = Math.round(window.innerHeight * 0.2)
      const maxPx = Math.round(window.innerHeight * 0.85)
      const nextHeight = window.innerHeight - ev.clientY
      const clamped = Math.max(minPx, Math.min(maxPx, nextHeight))
      terminalDrawerActions.setHeightRatio(clamped / window.innerHeight)
    }

    const onPointerUp = () => setDragging(false)
    const onResize = () => setState(getTerminalDrawerState())
    const onPointerDown = (event: PointerEvent) => {
      const menuTab = tabMenuTabId()
      if (!menuTab) return
      const target = event.target as HTMLElement | null
      if (target?.closest('[data-tab-menu]')) return
      if (target?.closest('[data-tab-menu-trigger]')) return
      setTabMenuTabId(null)
    }

    window.addEventListener('pointermove', onPointerMove)
    window.addEventListener('pointerup', onPointerUp)
    window.addEventListener('pointercancel', onPointerUp)
    window.addEventListener('resize', onResize)
    window.addEventListener('pointerdown', onPointerDown)
    window.addEventListener(WALLET_SESSION_EVENT, onWalletSession as EventListener)

    void refreshDeviceStatus()
    const timer = window.setInterval(() => {
      void refreshDeviceStatus()
    }, 12000)

    onCleanup(() => {
      unsubscribe()
      document.documentElement.style.removeProperty('--terminal-drawer-height')
      window.removeEventListener('pointermove', onPointerMove)
      window.removeEventListener('pointerup', onPointerUp)
      window.removeEventListener('pointercancel', onPointerUp)
      window.removeEventListener('resize', onResize)
      window.removeEventListener('pointerdown', onPointerDown)
      window.removeEventListener(WALLET_SESSION_EVENT, onWalletSession as EventListener)
      window.clearInterval(timer)
    })
  })

  return (
    <Show when={walletConnected()}>
      <section
        id="edgerun-terminal-drawer"
        aria-label="Terminal Drawer"
        aria-hidden={!state().open}
        data-open={state().open ? 'true' : 'false'}
        class="terminal-drawer fixed inset-x-0 bottom-0 z-[70] border-t border-border/80 bg-black/90 backdrop-blur"
      >
      <Show when={state().open}>
        <button
          type="button"
          class="group absolute inset-x-0 -top-1 z-10 h-5 touch-none cursor-row-resize border-0 bg-transparent"
          aria-label="Resize terminal drawer"
          onPointerDown={(ev) => {
            ev.preventDefault()
            setDragging(true)
            terminalDrawerActions.setOpen(true)
          }}
        >
          <span class="pointer-events-none mx-auto mt-1 block h-1 w-16 rounded-full bg-border/80 transition-colors group-hover:bg-primary/70" />
        </button>
      </Show>

      <div class="flex h-full min-h-0 flex-col">
        <div class="flex items-center gap-2 border-b border-border/70 px-3 py-2">
          <div class="flex flex-1 items-center gap-1 overflow-x-auto">
            <For each={state().tabs}>{(tab) => (
              <div class={`relative inline-flex items-center rounded-md border ${tab.id === state().activeTabId ? 'border-primary/70 bg-primary/20 text-foreground' : 'border-border/70 bg-card/60 text-muted-foreground hover:text-foreground'}`}>
                <button
                  type="button"
                  class="px-2 py-1 text-xs"
                  onClick={() => terminalDrawerActions.setActiveTab(tab.id)}
                >
                  {tab.title}
                </button>
                <button
                  type="button"
                  class="px-1.5 py-1 text-xs text-muted-foreground hover:text-foreground"
                  aria-label={`Tab options for ${tab.title}`}
                  data-tab-menu-trigger
                  onClick={(event) => {
                    event.stopPropagation()
                    setTabMenuTabId(tabMenuTabId() === tab.id ? null : tab.id)
                  }}
                >
                  ⋮
                </button>
                <Show when={state().tabs.length > 1}>
                  <button
                    type="button"
                    class="px-1.5 py-1 text-xs text-muted-foreground hover:text-foreground"
                    aria-label={`Close ${tab.title}`}
                    onClick={(event) => {
                      event.stopPropagation()
                      terminalDrawerActions.closeTab(tab.id)
                    }}
                  >
                    x
                  </button>
                </Show>

                <Show when={tabMenuTabId() === tab.id}>
                  <div
                    data-tab-menu
                    class="absolute left-0 top-full z-[90] mt-1 w-44 rounded-md border border-border/70 bg-card/95 p-1 shadow-xl backdrop-blur"
                  >
                    <button
                      type="button"
                      class="block w-full rounded px-2 py-1 text-left text-xs text-muted-foreground hover:bg-muted/50 hover:text-foreground disabled:opacity-50"
                      disabled={!hasOtherTabs()}
                      onClick={() => {
                        terminalDrawerActions.closeOtherTabs(tab.id)
                        setTabMenuTabId(null)
                      }}
                    >
                      Close Others
                    </button>
                    <button
                      type="button"
                      class="block w-full rounded px-2 py-1 text-left text-xs text-muted-foreground hover:bg-muted/50 hover:text-foreground disabled:opacity-50"
                      disabled={!hasTabsLeft(tab.id)}
                      onClick={() => {
                        terminalDrawerActions.closeTabsLeft(tab.id)
                        setTabMenuTabId(null)
                      }}
                    >
                      Close Left
                    </button>
                    <button
                      type="button"
                      class="block w-full rounded px-2 py-1 text-left text-xs text-muted-foreground hover:bg-muted/50 hover:text-foreground disabled:opacity-50"
                      disabled={!hasTabsRight(tab.id)}
                      onClick={() => {
                        terminalDrawerActions.closeTabsRight(tab.id)
                        setTabMenuTabId(null)
                      }}
                    >
                      Close Right
                    </button>
                    <button
                      type="button"
                      class="block w-full rounded px-2 py-1 text-left text-xs text-muted-foreground hover:bg-muted/50 hover:text-foreground disabled:opacity-50"
                      disabled={!hasOtherTabs()}
                      onClick={() => {
                        terminalDrawerActions.closeAllTabs()
                        setTabMenuTabId(null)
                      }}
                    >
                      Close All
                    </button>
                  </div>
                </Show>
              </div>
            )}</For>
            <button
              type="button"
              class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground"
              onClick={() => terminalDrawerActions.addTab()}
            >
              + Tab
            </button>
          </div>

          <div class="flex items-center gap-1">
            <button type="button" class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground" onClick={() => splitChange('split-cols')}>Split V</button>
            <button type="button" class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground" onClick={() => splitChange('split-rows')}>Split H</button>
            <button type="button" class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground" onClick={() => splitChange('none')}>Single</button>
            <button type="button" class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground" aria-label="Close terminal drawer" onClick={() => terminalDrawerActions.toggle()}>Hide</button>
          </div>
        </div>

        <Show when={state().open} fallback={<div class="flex h-full items-center px-3 text-xs text-muted-foreground">Terminal hidden. Use the navbar toggle to open.</div>}>
          <div class="grid min-h-0 flex-1 grid-cols-[280px_minmax(0,1fr)] gap-0">
            <aside class="border-r border-border/70 bg-card/30 p-3">
              <div class="mb-2 flex items-center justify-between">
                <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Connected Devices</p>
                <div class="flex items-center gap-1">
                  <button
                    type="button"
                    class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-[11px] text-muted-foreground hover:text-foreground"
                    onClick={() => void refreshDeviceStatus()}
                  >
                    Refresh
                  </button>
                  <button
                    type="button"
                    class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-[11px] text-muted-foreground hover:text-foreground disabled:opacity-60"
                    disabled={tailscaleImporting()}
                    onClick={() => void importTailscaleDevices({ silent: false })}
                  >
                    {tailscaleImporting() ? 'Importing...' : 'Import TS'}
                  </button>
                </div>
              </div>
              <label class="mb-2 flex items-center gap-2 text-[11px] text-muted-foreground">
                <input
                  type="checkbox"
                  checked={state().autoImportTailscale}
                  onChange={(event) => terminalDrawerActions.setAutoImportTailscale((event.currentTarget as HTMLInputElement).checked)}
                />
                Auto import Tailscale on wallet connect
              </label>
              <Show when={tailscaleImportNote().length > 0}>
                <p class="mb-2 text-[11px] text-muted-foreground">{tailscaleImportNote()}</p>
              </Show>

              <div class="mb-3 space-y-2">
                <input
                  type="text"
                  value={deviceNameInput()}
                  placeholder="Device name"
                  class="h-8 w-full rounded-md border border-border bg-background/80 px-2 text-xs text-foreground"
                  onInput={(ev) => setDeviceNameInput(ev.currentTarget.value)}
                />
                <input
                  type="text"
                  value={deviceUrlInput()}
                  placeholder="https://device.edgerun.tech"
                  class="h-8 w-full rounded-md border border-border bg-background/80 px-2 font-mono text-xs text-foreground"
                  onInput={(ev) => setDeviceUrlInput(ev.currentTarget.value)}
                />
                <button
                  type="button"
                  class="h-8 w-full rounded-md border border-border/70 bg-card/70 px-2 text-xs text-muted-foreground hover:text-foreground"
                  onClick={() => {
                    terminalDrawerActions.addDevice(deviceNameInput(), deviceUrlInput())
                    setDeviceNameInput('')
                    setDeviceUrlInput('')
                    void refreshDeviceStatus()
                  }}
                >
                  Add Device
                </button>
              </div>

              <div class="space-y-2 overflow-y-auto pr-1">
                <Show when={state().devices.length > 0} fallback={<p class="text-xs text-muted-foreground">No devices yet. Add a relay URL, then connect a tab.</p>}>
                  <For each={state().devices}>{(device) => (
                    <div class="rounded-md border border-border/70 bg-background/40 p-2">
                      <div class="flex items-center justify-between gap-2">
                        <p class="truncate text-xs font-medium text-foreground">{device.name}</p>
                        <span class={`text-[10px] uppercase ${statusBadge(device.status)}`}>{device.status}</span>
                      </div>
                      <p class="mt-1 truncate font-mono text-[10px] text-muted-foreground">{device.baseUrl}</p>
                      <div class="mt-2 flex gap-1">
                        <button
                          type="button"
                          class="flex-1 rounded-md border border-border/70 bg-card/60 px-2 py-1 text-[11px] text-muted-foreground hover:text-foreground"
                          onClick={() => terminalDrawerActions.connectActiveTabToDevice(device.id)}
                        >
                          Connect
                        </button>
                        <button
                          type="button"
                          class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-[11px] text-muted-foreground hover:text-foreground"
                          onClick={() => setQrDeviceUrl(device.baseUrl)}
                        >
                          QR
                        </button>
                        <button
                          type="button"
                          class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-[11px] text-muted-foreground hover:text-foreground"
                          onClick={() => terminalDrawerActions.removeDevice(device.id)}
                        >
                          Remove
                        </button>
                      </div>
                    </div>
                  )}</For>
                </Show>
              </div>

              <Show when={qrImageDataUrl().length > 0}>
                <div class="mt-3 rounded-md border border-border/70 bg-background/40 p-2">
                  <p class="mb-2 text-[11px] uppercase tracking-wide text-muted-foreground">Device QR</p>
                  <img src={qrImageDataUrl()} alt="Device URL QR code" class="mx-auto h-40 w-40 rounded border border-border/70 bg-white p-1" />
                  <p class="mt-2 truncate font-mono text-[10px] text-muted-foreground">{qrDeviceUrl()}</p>
                  <div class="mt-2 flex gap-1">
                    <button
                      type="button"
                      class="flex-1 rounded-md border border-border/70 bg-card/60 px-2 py-1 text-[11px] text-muted-foreground hover:text-foreground"
                      onClick={async () => {
                        try {
                          await navigator.clipboard.writeText(qrDeviceUrl())
                        } catch {
                          // ignore copy failures
                        }
                      }}
                    >
                      Copy URL
                    </button>
                    <button
                      type="button"
                      class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-[11px] text-muted-foreground hover:text-foreground"
                      onClick={() => setQrDeviceUrl('')}
                    >
                      Close
                    </button>
                  </div>
                </div>
              </Show>
            </aside>

            <div class="min-h-0 p-2">
              <Show when={activeTab()}>
                {(tab) => (
                  <div class={`grid h-full gap-2 ${splitClassName(tab())}`}>
                    <For each={tab().panes}>{(pane) => {
                      const src = createMemo(() => getTerminalPaneSrc(pane.baseUrl, pane.id))
                      return (
                        <Show
                          when={src().length > 0}
                          fallback={
                            <div class="flex h-full min-h-0 items-center justify-center rounded-md border border-dashed border-border/70 bg-background/40 p-4 text-center text-xs text-muted-foreground">
                              Select a connected device to open this pane.
                            </div>
                          }
                        >
                          <iframe
                            title={`Terminal ${pane.id}`}
                            class="h-full min-h-0 w-full rounded-md border border-border/70 bg-black"
                            src={src()}
                            loading="eager"
                            allow="clipboard-read; clipboard-write"
                          />
                        </Show>
                      )
                    }}</For>
                  </div>
                )}
              </Show>
            </div>
          </div>
        </Show>
      </div>
      </section>
    </Show>
  )
}
