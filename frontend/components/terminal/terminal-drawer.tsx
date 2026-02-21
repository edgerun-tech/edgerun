import { For, Show, createEffect, createMemo, createSignal, onCleanup, onMount } from 'solid-js'
import {
  ensureTerminalDrawerStore,
  getTerminalDrawerState,
  getTerminalPaneSrc,
  subscribeTerminalDrawer,
  terminalDrawerActions,
  type TerminalDrawerState,
  type TerminalSplitMode,
  type TerminalTab
} from '../../lib/terminal-drawer-store'

function splitClassName(tab: TerminalTab): string {
  if (tab.split === 'split-cols') return 'terminal-grid-split-cols'
  if (tab.split === 'split-rows') return 'terminal-grid-split-rows'
  return 'terminal-grid-split-none'
}

export function TerminalDrawer() {
  const [state, setState] = createSignal<TerminalDrawerState>(getTerminalDrawerState())
  const [dragging, setDragging] = createSignal(false)

  const activeTab = createMemo(() => {
    const current = state()
    return current.tabs.find((tab) => tab.id === current.activeTabId) ?? current.tabs[0]
  })

  const drawerHeight = createMemo(() => {
    const current = state()
    if (!current.open) return 48
    return Math.round(window.innerHeight * current.heightRatio)
  })

  const splitChange = (mode: TerminalSplitMode) => {
    terminalDrawerActions.setSplit(mode)
  }

  createEffect(() => {
    if (typeof document === 'undefined') return
    document.documentElement.style.setProperty('--terminal-drawer-height', `${drawerHeight()}px`)
  })

  onMount(() => {
    ensureTerminalDrawerStore()
    setState(getTerminalDrawerState())

    const unsubscribe = subscribeTerminalDrawer((next) => setState(next))

    const onPointerMove = (ev: PointerEvent) => {
      if (!dragging()) return
      const minPx = Math.round(window.innerHeight * 0.2)
      const maxPx = Math.round(window.innerHeight * 0.85)
      const nextHeight = window.innerHeight - ev.clientY
      const clamped = Math.max(minPx, Math.min(maxPx, nextHeight))
      terminalDrawerActions.setHeightRatio(clamped / window.innerHeight)
    }

    const onPointerUp = () => {
      setDragging(false)
    }

    const onResize = () => {
      setState(getTerminalDrawerState())
    }

    window.addEventListener('pointermove', onPointerMove)
    window.addEventListener('pointerup', onPointerUp)
    window.addEventListener('pointercancel', onPointerUp)
    window.addEventListener('resize', onResize)

    onCleanup(() => {
      unsubscribe()
      document.documentElement.style.removeProperty('--terminal-drawer-height')
      window.removeEventListener('pointermove', onPointerMove)
      window.removeEventListener('pointerup', onPointerUp)
      window.removeEventListener('pointercancel', onPointerUp)
      window.removeEventListener('resize', onResize)
    })
  })

  return (
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
          class="absolute left-0 right-0 top-0 h-2 cursor-row-resize border-0 bg-transparent"
          aria-label="Resize terminal drawer"
          onPointerDown={(ev) => {
            ev.preventDefault()
            setDragging(true)
            terminalDrawerActions.setOpen(true)
          }}
        />
      </Show>

      <div class="flex h-full flex-col">
        <div class="flex items-center gap-2 border-b border-border/70 px-3 py-2">
          <div class="flex flex-1 items-center gap-1 overflow-x-auto">
            <For each={state().tabs}>{(tab) => (
              <button
                type="button"
                class={`rounded-md border px-2 py-1 text-xs ${tab.id === state().activeTabId ? 'border-primary/70 bg-primary/20 text-foreground' : 'border-border/70 bg-card/60 text-muted-foreground hover:text-foreground'}`}
                onClick={() => terminalDrawerActions.setActiveTab(tab.id)}
              >
                {tab.title}
              </button>
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
            <button
              type="button"
              class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground"
              onClick={() => splitChange('split-cols')}
            >
              Split V
            </button>
            <button
              type="button"
              class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground"
              onClick={() => splitChange('split-rows')}
            >
              Split H
            </button>
            <button
              type="button"
              class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground"
              onClick={() => splitChange('none')}
            >
              Single
            </button>
            <button
              type="button"
              class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground"
              onClick={() => terminalDrawerActions.closeActiveTab()}
            >
              Close Tab
            </button>
            <button
              type="button"
              class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground"
              aria-label="Close terminal drawer"
              onClick={() => terminalDrawerActions.toggle()}
            >
              Hide
            </button>
          </div>
        </div>

        <Show when={state().open} fallback={<div class="flex h-full items-center px-3 text-xs text-muted-foreground">Terminal hidden. Use the navbar toggle to open.</div>}>
          <div class="flex items-center gap-2 border-b border-border/60 px-3 py-2">
            <span class="text-[11px] uppercase tracking-wide text-muted-foreground">Base URL</span>
            <input
              id="global-terminal-base-url"
              type="text"
              value={state().baseUrl}
              class="h-8 w-full max-w-xl rounded-md border border-border bg-background/80 px-2 font-mono text-xs text-foreground"
              onChange={(ev) => terminalDrawerActions.setBaseUrl(ev.currentTarget.value)}
            />
          </div>

          <div class="min-h-0 flex-1 p-2">
            <Show when={activeTab()}>
              {(tab) => (
                <div class={`grid h-full gap-2 ${splitClassName(tab())}`}>
                  <For each={tab().panes}>{(pane) => (
                    <iframe
                      title={`Terminal ${pane.id}`}
                      class="h-full min-h-0 w-full rounded-md border border-border/70 bg-black"
                      src={getTerminalPaneSrc(pane.baseUrl || state().baseUrl, pane.id)}
                      loading="eager"
                      allow="clipboard-read; clipboard-write"
                    />
                  )}</For>
                </div>
              )}
            </Show>
          </div>
        </Show>
      </div>
    </section>
  )
}
