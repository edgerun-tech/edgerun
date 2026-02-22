import { Show } from 'solid-js'
import { TerminalDevicesPanel } from './terminal-devices-panel'
import { TerminalPanesView } from './terminal-panes-view'
import { TerminalTabsBar } from './terminal-tabs-bar'
import { useTerminalDrawerController } from './use-terminal-drawer-controller'

export function TerminalDrawer() {
  const controller = useTerminalDrawerController()

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
            onPointerDown={controller.startResize}
          >
            <span class="pointer-events-none mx-auto mt-1 block h-1 w-16 rounded-full bg-border/80 transition-colors group-hover:bg-primary/70" />
          </button>
        </Show>

        <div class="flex h-full min-h-0 flex-col">
          <TerminalTabsBar
            state={controller.state}
            tabMenuTabId={controller.tabMenuTabId}
            setTabMenuTabId={controller.setTabMenuTabId}
            hasTabsLeft={controller.hasTabsLeft}
            hasTabsRight={controller.hasTabsRight}
            hasOtherTabs={controller.hasOtherTabs}
            splitChange={controller.splitChange}
          />

          <Show when={controller.state().open} fallback={<div class="flex h-full items-center px-3 text-xs text-muted-foreground">Terminal hidden. Use the navbar toggle to open.</div>}>
            <div class="grid min-h-0 flex-1 grid-cols-[280px_minmax(0,1fr)] gap-0">
              <TerminalDevicesPanel
                state={controller.state}
                refreshDeviceStatus={controller.refreshDeviceStatus}
                importTailscaleDevices={controller.importTailscaleDevices}
                tailscaleImporting={controller.tailscaleImporting}
                tailscaleImportNote={controller.tailscaleImportNote}
                deviceNameInput={controller.deviceNameInput}
                setDeviceNameInput={controller.setDeviceNameInput}
                deviceUrlInput={controller.deviceUrlInput}
                setDeviceUrlInput={controller.setDeviceUrlInput}
                addDevice={controller.addDevice}
                qrDeviceUrl={controller.qrDeviceUrl}
                setQrDeviceUrl={controller.setQrDeviceUrl}
                qrImageDataUrl={controller.qrImageDataUrl}
              />
              <TerminalPanesView activeTab={controller.activeTab} />
            </div>
          </Show>
        </div>
      </section>
    </Show>
  )
}
