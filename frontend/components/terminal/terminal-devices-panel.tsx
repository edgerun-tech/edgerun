import { For, Show, type Accessor, type Setter } from 'solid-js'
import { terminalDrawerActions, type DeviceStatus, type TerminalDrawerState } from '../../lib/terminal-drawer-store'

type Props = {
  state: Accessor<TerminalDrawerState>
  refreshDeviceStatus: () => Promise<void> | void
  importTailscaleDevices: (opts?: { silent?: boolean }) => Promise<void>
  tailscaleImporting: Accessor<boolean>
  tailscaleImportNote: Accessor<string>
  deviceNameInput: Accessor<string>
  setDeviceNameInput: Setter<string>
  deviceUrlInput: Accessor<string>
  setDeviceUrlInput: Setter<string>
  addDevice: () => void
  qrDeviceUrl: Accessor<string>
  setQrDeviceUrl: Setter<string>
  qrImageDataUrl: Accessor<string>
}

function statusBadge(status: DeviceStatus): string {
  if (status === 'online') return 'text-emerald-400'
  if (status === 'offline') return 'text-rose-400'
  return 'text-muted-foreground'
}

export function TerminalDevicesPanel(props: Props) {
  return (
    <aside class="border-r border-border/70 bg-card/30 p-3">
      <div class="mb-2 flex items-center justify-between">
        <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Connected Devices</p>
        <div class="flex items-center gap-1">
          <button
            type="button"
            class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-[11px] text-muted-foreground hover:text-foreground"
            onClick={() => void props.refreshDeviceStatus()}
          >
            Refresh
          </button>
          <button
            type="button"
            class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-[11px] text-muted-foreground hover:text-foreground disabled:opacity-60"
            disabled={props.tailscaleImporting()}
            onClick={() => void props.importTailscaleDevices({ silent: false })}
          >
            {props.tailscaleImporting() ? 'Importing...' : 'Import TS'}
          </button>
        </div>
      </div>
      <label class="mb-2 flex items-center gap-2 text-[11px] text-muted-foreground">
        <input
          type="checkbox"
          checked={props.state().autoImportTailscale}
          onChange={(event) => terminalDrawerActions.setAutoImportTailscale((event.currentTarget as HTMLInputElement).checked)}
        />
        Auto import Tailscale on wallet connect
      </label>
      <Show when={props.tailscaleImportNote().length > 0}>
        <p class="mb-2 text-[11px] text-muted-foreground">{props.tailscaleImportNote()}</p>
      </Show>

      <div class="mb-3 space-y-2">
        <input
          type="text"
          value={props.deviceNameInput()}
          placeholder="Device name"
          class="h-8 w-full rounded-md border border-border bg-background/80 px-2 text-xs text-foreground"
          onInput={(ev) => props.setDeviceNameInput(ev.currentTarget.value)}
        />
        <input
          type="text"
          value={props.deviceUrlInput()}
          placeholder="https://device.edgerun.tech"
          class="h-8 w-full rounded-md border border-border bg-background/80 px-2 font-mono text-xs text-foreground"
          onInput={(ev) => props.setDeviceUrlInput(ev.currentTarget.value)}
        />
        <button
          type="button"
          class="h-8 w-full rounded-md border border-border/70 bg-card/70 px-2 text-xs text-muted-foreground hover:text-foreground"
          onClick={() => props.addDevice()}
        >
          Add Device
        </button>
      </div>

      <div class="space-y-2 overflow-y-auto pr-1">
        <Show when={props.state().devices.length > 0} fallback={<p class="text-xs text-muted-foreground">No devices yet. Add a relay URL, then connect a tab.</p>}>
          <For each={props.state().devices}>{(device) => (
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
                  onClick={() => props.setQrDeviceUrl(device.baseUrl)}
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

      <Show when={props.qrImageDataUrl().length > 0}>
        <div class="mt-3 rounded-md border border-border/70 bg-background/40 p-2">
          <p class="mb-2 text-[11px] uppercase tracking-wide text-muted-foreground">Device QR</p>
          <img src={props.qrImageDataUrl()} alt="Device URL QR code" class="mx-auto h-40 w-40 rounded border border-border/70 bg-white p-1" />
          <p class="mt-2 truncate font-mono text-[10px] text-muted-foreground">{props.qrDeviceUrl()}</p>
          <div class="mt-2 flex gap-1">
            <button
              type="button"
              class="flex-1 rounded-md border border-border/70 bg-card/60 px-2 py-1 text-[11px] text-muted-foreground hover:text-foreground"
              onClick={async () => {
                try {
                  await navigator.clipboard.writeText(props.qrDeviceUrl())
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
              onClick={() => props.setQrDeviceUrl('')}
            >
              Close
            </button>
          </div>
        </div>
      </Show>
    </aside>
  )
}
