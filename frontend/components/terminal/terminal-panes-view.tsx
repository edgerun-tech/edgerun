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

function toTermWebSrc(target: string, paneId: string): string {
  const raw = String(target || '').trim()
  if (!raw) return ''
  if (parseRouteDeviceId(raw)) return ''
  const withScheme = /^[a-zA-Z][a-zA-Z\d+\-.]*:\/\//.test(raw) ? raw : `http://${raw}`
  let url: URL
  try {
    url = new URL(withScheme)
  } catch {
    return ''
  }
  if (url.protocol !== 'http:' && url.protocol !== 'https:') return ''
  const path = url.pathname.replace(/\/+$/, '')
  if (!path || path === '/') {
    url.pathname = '/term'
  } else if (!/\/term$/i.test(path)) {
    url.pathname = `${path}/term`
  }
  url.searchParams.set('sid', paneId)
  return url.toString()
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
              const target = () => pane()?.baseUrl.trim() || ''
              const routeDeviceId = () => parseRouteDeviceId(target())
              const termWebSrc = () => toTermWebSrc(target(), paneId)
              return (
                <Show when={routeDeviceId()} fallback={
                  <Show
                    when={target().length > 0}
                    fallback={
                      <div
                        class="flex h-full min-h-0 items-center justify-center rounded-md border border-dashed border-border/70 bg-background/40 p-4 text-center text-xs text-muted-foreground"
                      >
                        Select a connected device to open this pane.
                      </div>
                    }
                  >
                    <Show
                      when={termWebSrc().length > 0}
                      fallback={
                        <div
                          class="flex h-full min-h-0 flex-col items-center justify-center rounded-md border border-amber-500/40 bg-amber-500/5 p-4 text-center"
                          data-testid="terminal-nonroute-disabled"
                        >
                          <p class="text-sm font-semibold text-amber-300">Invalid terminal target URL.</p>
                          <p class="mt-2 text-xs text-muted-foreground">Use `http(s)://host[:port]` for term-web or `route://&lt;device-id&gt;` for routed sessions.</p>
                          <p class="mt-2 break-all font-mono text-[11px] text-muted-foreground">{target()}</p>
                        </div>
                      }
                    >
                      <iframe
                        src={termWebSrc()}
                        class="h-full min-h-0 w-full rounded-md border border-border/70 bg-black"
                        title="Terminal web session"
                        loading="lazy"
                        referrerPolicy="no-referrer"
                        sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-downloads"
                        data-testid="terminal-web-iframe"
                      />
                    </Show>
                  </Show>
                }>
                  {(routeDevice) => <RoutedTerminalPane paneId={paneId} routeDeviceId={routeDevice()} />}
                </Show>
              )
            }}</For>
          </div>
        )}
      </Show>
    </div>
  )
}
