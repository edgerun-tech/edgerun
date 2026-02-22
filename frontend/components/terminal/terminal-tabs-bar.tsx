import { For, Show } from 'solid-js'
import { terminalDrawerActions } from '../../lib/terminal-drawer-store'
import type { TerminalTabsController } from './use-terminal-drawer-controller'

type Props = {
  controller: TerminalTabsController
}

export function TerminalTabsBar(props: Props) {
  const tabs = () => props.controller
  const transportLabel = () => {
    const mode = tabs().activeTransport()
    if (mode === 'mux') return 'MUX'
    if (mode === 'raw') return 'RAW'
    return '...'
  }
  const transportClass = () => {
    const mode = tabs().activeTransport()
    if (mode === 'mux') return 'border-emerald-500/40 text-emerald-300'
    if (mode === 'raw') return 'border-amber-500/40 text-amber-300'
    return 'border-border/70 text-muted-foreground'
  }
  return (
    <div class="flex items-center gap-2 border-b border-border/70 px-3 py-2">
      <div class="flex flex-1 items-center gap-1 overflow-x-auto">
        <For each={tabs().state().tabs}>{(tab) => (
          <div class={`relative inline-flex items-center rounded-md border ${tab.id === tabs().state().activeTabId ? 'border-primary/70 bg-primary/20 text-foreground' : 'border-border/70 bg-card/60 text-muted-foreground hover:text-foreground'}`}>
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
                tabs().setTabMenuTabId(tabs().tabMenuTabId() === tab.id ? null : tab.id)
              }}
            >
              ⋮
            </button>
            <Show when={tabs().state().tabs.length > 1}>
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

            <Show when={tabs().tabMenuTabId() === tab.id}>
              <div
                data-tab-menu
                class="absolute left-0 top-full z-[90] mt-1 w-44 rounded-md border border-border/70 bg-card/95 p-1 shadow-xl backdrop-blur"
              >
                <button
                  type="button"
                  class="block w-full rounded px-2 py-1 text-left text-xs text-muted-foreground hover:bg-muted/50 hover:text-foreground disabled:opacity-50"
                  disabled={!tabs().hasOtherTabs()}
                  onClick={() => {
                    terminalDrawerActions.closeOtherTabs(tab.id)
                    tabs().setTabMenuTabId(null)
                  }}
                >
                  Close Others
                </button>
                <button
                  type="button"
                  class="block w-full rounded px-2 py-1 text-left text-xs text-muted-foreground hover:bg-muted/50 hover:text-foreground disabled:opacity-50"
                  disabled={!tabs().hasTabsLeft(tab.id)}
                  onClick={() => {
                    terminalDrawerActions.closeTabsLeft(tab.id)
                    tabs().setTabMenuTabId(null)
                  }}
                >
                  Close Left
                </button>
                <button
                  type="button"
                  class="block w-full rounded px-2 py-1 text-left text-xs text-muted-foreground hover:bg-muted/50 hover:text-foreground disabled:opacity-50"
                  disabled={!tabs().hasTabsRight(tab.id)}
                  onClick={() => {
                    terminalDrawerActions.closeTabsRight(tab.id)
                    tabs().setTabMenuTabId(null)
                  }}
                >
                  Close Right
                </button>
                <button
                  type="button"
                  class="block w-full rounded px-2 py-1 text-left text-xs text-muted-foreground hover:bg-muted/50 hover:text-foreground disabled:opacity-50"
                  disabled={!tabs().hasOtherTabs()}
                  onClick={() => {
                    terminalDrawerActions.closeAllTabs()
                    tabs().setTabMenuTabId(null)
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
        <div class={`rounded-md border bg-card/60 px-2 py-1 text-[10px] font-semibold tracking-[0.08em] ${transportClass()}`} aria-label={`Terminal transport ${transportLabel()}`}>
          {transportLabel()}
        </div>
        <button type="button" class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground" onClick={() => tabs().splitChange('split-cols')}>Split V</button>
        <button type="button" class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground" onClick={() => tabs().splitChange('split-rows')}>Split H</button>
        <button type="button" class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground" onClick={() => tabs().splitChange('none')}>Single</button>
        <button type="button" class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground" aria-label="Close terminal drawer" onClick={() => terminalDrawerActions.toggle()}>Hide</button>
      </div>
    </div>
  )
}
