import { For, Show, type Accessor } from 'solid-js'
import { getTerminalPaneSrc, type TerminalTab } from '../../lib/terminal-drawer-store'

type Props = {
  activeTab: Accessor<TerminalTab | undefined>
}

function splitClassName(tab: TerminalTab): string {
  if (tab.split === 'split-cols') return 'terminal-grid-split-cols'
  if (tab.split === 'split-rows') return 'terminal-grid-split-rows'
  return 'terminal-grid-split-none'
}

export function TerminalPanesView(props: Props) {
  return (
    <div class="min-h-0 p-2">
      <Show when={props.activeTab()}>
        {(tab) => (
          <div class={`grid h-full gap-2 ${splitClassName(tab())}`}>
            <For each={tab().panes}>{(pane) => {
              const src = getTerminalPaneSrc(pane.baseUrl, pane.id)
              return (
                <Show
                  when={src.length > 0}
                  fallback={
                    <div class="flex h-full min-h-0 items-center justify-center rounded-md border border-dashed border-border/70 bg-background/40 p-4 text-center text-xs text-muted-foreground">
                      Select a connected device to open this pane.
                    </div>
                  }
                >
                  <iframe
                    title={`Terminal ${pane.id}`}
                    class="h-full min-h-0 w-full rounded-md border border-border/70 bg-black"
                    src={src}
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
  )
}
