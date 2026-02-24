// SPDX-License-Identifier: Apache-2.0
import { Show } from 'solid-js'
import { terminalDrawerActions } from '../../lib/terminal-drawer-store'
import { TerminalDevicesPanel } from './terminal-devices-panel'
import { TerminalPanesView } from './terminal-panes-view'
import { TerminalTabsBar } from './terminal-tabs-bar'
import { useTerminalDrawerController } from './use-terminal-drawer-controller'

const DRAWER_MIN_RATIO = 0.2
const DRAWER_MAX_RATIO = 0.85
const DRAWER_KEYBOARD_STEP = 0.05

export function TerminalDrawer() {
  const controller = useTerminalDrawerController()

  function setDrawerRatio(next: number): void {
    const clamped = Math.max(DRAWER_MIN_RATIO, Math.min(DRAWER_MAX_RATIO, next))
    terminalDrawerActions.setHeightRatio(clamped)
  }

  function onResizeHandleKeyDown(event: KeyboardEvent): void {
    const current = controller.state().heightRatio
    if (event.key === 'ArrowUp') {
      event.preventDefault()
      setDrawerRatio(current + DRAWER_KEYBOARD_STEP)
      return
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      setDrawerRatio(current - DRAWER_KEYBOARD_STEP)
      return
    }
    if (event.key === 'Home') {
      event.preventDefault()
      setDrawerRatio(DRAWER_MIN_RATIO)
      return
    }
    if (event.key === 'End') {
      event.preventDefault()
      setDrawerRatio(DRAWER_MAX_RATIO)
    }
  }

  return (
    <Show when={controller.walletConnected()}>
      <section
        id="edgerun-terminal-drawer"
        aria-label="Terminal Drawer"
        aria-hidden={!controller.state().open}
        data-open={controller.state().open ? 'true' : 'false'}
        class="terminal-drawer fixed inset-x-0 bottom-0 z-[70] border-t border-border/80 bg-black/90 backdrop-blur"
      >
        <Show when={controller.state().open}>
          <button
            type="button"
            class="group absolute inset-x-0 -top-1 z-10 h-5 touch-none cursor-row-resize border-0 bg-transparent"
            aria-label="Resize terminal drawer"
            aria-keyshortcuts="ArrowUp ArrowDown Home End"
            onPointerDown={controller.startResize}
            onKeyDown={onResizeHandleKeyDown}
          >
            <span class="pointer-events-none mx-auto mt-1 block h-1 w-16 rounded-full bg-border/80 transition-colors group-hover:bg-primary/70" />
          </button>
        </Show>

        <div class="flex h-full min-h-0 flex-col">
          <TerminalTabsBar
            controller={controller.tabs}
          />

          <Show when={controller.state().open} fallback={<div class="flex h-full items-center px-3 text-xs text-muted-foreground">Terminal hidden. Use the navbar toggle to open.</div>}>
            <div class="grid min-h-0 flex-1 grid-cols-[280px_minmax(0,1fr)] gap-0">
              <TerminalDevicesPanel
                controller={controller.devices}
              />
              <TerminalPanesView controller={controller.panes} />
            </div>
          </Show>
        </div>
      </section>
    </Show>
  )
}
