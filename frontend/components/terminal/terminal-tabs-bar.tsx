import { For, Show, type Accessor, type Setter } from 'solid-js'
import { terminalDrawerActions, type TerminalDrawerState, type TerminalSplitMode } from '../../lib/terminal-drawer-store'

type Props = {
  state: Accessor<TerminalDrawerState>
  tabMenuTabId: Accessor<string | null>
  setTabMenuTabId: Setter<string | null>
  hasTabsLeft: (tabId: string) => boolean
  hasTabsRight: (tabId: string) => boolean
  hasOtherTabs: () => boolean
  splitChange: (mode: TerminalSplitMode) => void
}

export function TerminalTabsBar(props: Props) {
  return (
    <div class="flex items-center gap-2 border-b border-border/70 px-3 py-2">
      <div class="flex flex-1 items-center gap-1 overflow-x-auto">
        <For each={props.state().tabs}>{(tab) => (
          <div class={`relative inline-flex items-center rounded-md border ${tab.id === props.state().activeTabId ? 'border-primary/70 bg-primary/20 text-foreground' : 'border-border/70 bg-card/60 text-muted-foreground hover:text-foreground'}`}>
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
                props.setTabMenuTabId(props.tabMenuTabId() === tab.id ? null : tab.id)
              }}
            >
              ⋮
            </button>
            <Show when={props.state().tabs.length > 1}>
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

            <Show when={props.tabMenuTabId() === tab.id}>
              <div
                data-tab-menu
                class="absolute left-0 top-full z-[90] mt-1 w-44 rounded-md border border-border/70 bg-card/95 p-1 shadow-xl backdrop-blur"
              >
                <button
                  type="button"
                  class="block w-full rounded px-2 py-1 text-left text-xs text-muted-foreground hover:bg-muted/50 hover:text-foreground disabled:opacity-50"
                  disabled={!props.hasOtherTabs()}
                  onClick={() => {
                    terminalDrawerActions.closeOtherTabs(tab.id)
                    props.setTabMenuTabId(null)
                  }}
                >
                  Close Others
                </button>
                <button
                  type="button"
                  class="block w-full rounded px-2 py-1 text-left text-xs text-muted-foreground hover:bg-muted/50 hover:text-foreground disabled:opacity-50"
                  disabled={!props.hasTabsLeft(tab.id)}
                  onClick={() => {
                    terminalDrawerActions.closeTabsLeft(tab.id)
                    props.setTabMenuTabId(null)
                  }}
                >
                  Close Left
                </button>
                <button
                  type="button"
                  class="block w-full rounded px-2 py-1 text-left text-xs text-muted-foreground hover:bg-muted/50 hover:text-foreground disabled:opacity-50"
                  disabled={!props.hasTabsRight(tab.id)}
                  onClick={() => {
                    terminalDrawerActions.closeTabsRight(tab.id)
                    props.setTabMenuTabId(null)
                  }}
                >
                  Close Right
                </button>
                <button
                  type="button"
                  class="block w-full rounded px-2 py-1 text-left text-xs text-muted-foreground hover:bg-muted/50 hover:text-foreground disabled:opacity-50"
                  disabled={!props.hasOtherTabs()}
                  onClick={() => {
                    terminalDrawerActions.closeAllTabs()
                    props.setTabMenuTabId(null)
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
        <button type="button" class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground" onClick={() => props.splitChange('split-cols')}>Split V</button>
        <button type="button" class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground" onClick={() => props.splitChange('split-rows')}>Split H</button>
        <button type="button" class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground" onClick={() => props.splitChange('none')}>Single</button>
        <button type="button" class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-xs text-muted-foreground hover:text-foreground" aria-label="Close terminal drawer" onClick={() => terminalDrawerActions.toggle()}>Hide</button>
      </div>
    </div>
  )
}
