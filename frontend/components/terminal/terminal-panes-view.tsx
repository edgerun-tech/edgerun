// SPDX-License-Identifier: Apache-2.0
import { For, Show } from 'solid-js'
import { type TerminalTab } from '../../lib/terminal-drawer-store'
import { parseRouteDeviceId } from '../../lib/webrtc-route-client'
import { RoutedTerminalPane } from './routed-terminal-pane'
import type { TerminalPanesController } from './use-terminal-drawer-controller'

type Props = {
  controller: TerminalPanesController
}

function splitClassName(tab: TerminalTab): string {
  if (tab.split === 'split-cols') return 'terminal-grid-split-cols'
  if (tab.split === 'split-rows') return 'terminal-grid-split-rows'
  return 'terminal-grid-split-none'
}

export function TerminalPanesView(props: Props) {
  const panes = () => props.controller
  return (
    <div class="min-h-0 p-2">
      <Show when={panes().activeTab()}>
        {(tab) => (
          <div class={`grid h-full gap-2 ${splitClassName(tab())}`}>
            <For each={tab().panes.map((pane) => pane.id)}>{(paneId) => {
              const pane = () => tab().panes.find((entry) => entry.id === paneId)
              const target = pane()?.baseUrl.trim() || ''
              const routeDeviceId = parseRouteDeviceId(target)
              if (routeDeviceId) {
                return <RoutedTerminalPane paneId={paneId} routeDeviceId={routeDeviceId} />
              }
              return (
                <Show
                  when={target.length > 0}
                  fallback={
                    <div class="flex h-full min-h-0 items-center justify-center rounded-md border border-dashed border-border/70 bg-background/40 p-4 text-center text-xs text-muted-foreground">
                      Select a connected device to open this pane.
                    </div>
                  }
                >
                  <div
                    class="flex h-full min-h-0 flex-col items-center justify-center rounded-md border border-amber-500/40 bg-amber-500/5 p-4 text-center"
                    data-testid="terminal-nonroute-disabled"
                  >
                    <p class="text-sm font-semibold text-amber-300">Web iframe terminal embed is disabled.</p>
                    <p class="mt-2 text-xs text-muted-foreground">Use a routed target (`route://&lt;device-id&gt;`) to open an in-app terminal session.</p>
                    <p class="mt-2 break-all font-mono text-[11px] text-muted-foreground">{target}</p>
                  </div>
                </Show>
              )
            }}</For>
          </div>
        )}
      </Show>
    </div>
  )
}
