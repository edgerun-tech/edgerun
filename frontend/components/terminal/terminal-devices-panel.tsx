// SPDX-License-Identifier: Apache-2.0
import { For, Show } from 'solid-js'
import { terminalDrawerActions, type DeviceStatus } from '../../lib/terminal-drawer-store'
import type { TerminalDevicesController } from './use-terminal-drawer-controller'

type Props = {
  controller: TerminalDevicesController
}

function statusBadge(status: DeviceStatus): string {
  if (status === 'online') return 'text-emerald-400'
  if (status === 'offline') return 'text-rose-400'
  return 'text-muted-foreground'
}

export function TerminalDevicesPanel(props: Props) {
  const devices = () => props.controller
  return (
    <aside class="border-r border-border/70 bg-card/30 p-3">
      <div class="mb-2 flex items-center justify-between">
        <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Connected Devices</p>
        <button
          type="button"
          class="rounded-md border border-border/70 bg-card/60 px-2 py-1 text-[11px] text-muted-foreground hover:text-foreground"
          onClick={() => void devices().refreshDeviceStatus()}
        >
          Refresh
        </button>
      </div>
      <div class="mb-3 space-y-2">
        <button
          type="button"
          class="h-8 w-full rounded-md border border-border/70 bg-card/70 px-2 text-xs text-muted-foreground hover:text-foreground disabled:opacity-60"
          disabled={devices().ownerImporting() || devices().walletAddress().length === 0}
          onClick={() => void devices().syncMyDevices()}
        >
          {devices().ownerImporting() ? 'Syncing devices...' : 'Sync My Devices'}
        </button>
        <Show when={devices().walletAddress().length > 0}>
          <p class="truncate font-mono text-[11px] text-muted-foreground">Wallet owner: {devices().walletAddress()}</p>
        </Show>
        <input
          type="text"
          value={devices().ownerPubkeyInput()}
          placeholder="Owner pubkey override (optional)"
          aria-label="Owner pubkey override"
          class="h-8 w-full rounded-md border border-border bg-background/80 px-2 font-mono text-xs text-foreground"
          onInput={(ev) => devices().setOwnerPubkeyInput(ev.currentTarget.value)}
        />
        <button
          type="button"
          class="h-8 w-full rounded-md border border-border/70 bg-card/70 px-2 text-xs text-muted-foreground hover:text-foreground disabled:opacity-60"
          disabled={devices().ownerImporting()}
          onClick={() => void devices().importOwnerDevices()}
        >
          {devices().ownerImporting() ? 'Importing owner...' : 'Import Owner Routes'}
        </button>
        <Show when={devices().ownerImportNote().length > 0}>
          <p class="text-[11px] text-muted-foreground">{devices().ownerImportNote()}</p>
        </Show>
      </div>

      <div class="mb-3 space-y-2">
        <input
          type="text"
          value={devices().deviceNameInput()}
          placeholder="Device name"
          aria-label="Device name"
          data-testid="terminal-device-name-input"
          class="h-8 w-full rounded-md border border-border bg-background/80 px-2 text-xs text-foreground"
          onInput={(ev) => devices().setDeviceNameInput(ev.currentTarget.value)}
        />
        <input
          type="text"
          value={devices().deviceUrlInput()}
          placeholder="route://device-id or https://host"
          aria-label="Device route URL"
          data-testid="terminal-device-url-input"
          class="h-8 w-full rounded-md border border-border bg-background/80 px-2 font-mono text-xs text-foreground"
          onInput={(ev) => devices().setDeviceUrlInput(ev.currentTarget.value)}
        />
        <button
          type="button"
          class="h-8 w-full rounded-md border border-border/70 bg-card/70 px-2 text-xs text-muted-foreground hover:text-foreground"
          onClick={() => devices().addDevice()}
        >
          Add Device
        </button>
      </div>

      <div class="space-y-2 overflow-y-auto pr-1">
        <Show when={devices().state().devices.length > 0} fallback={<p class="text-xs text-muted-foreground">No devices yet. Add a `route://` target, a term-server URL, or import owner routes, then connect a tab.</p>}>
          <For each={devices().state().devices}>{(device) => (
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
                  onClick={() => void devices().connectDevice(device)}
                >
                  Connect
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
    </aside>
  )
}
