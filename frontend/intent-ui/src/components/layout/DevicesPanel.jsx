import { Show, For } from "solid-js";
import {
  TbOutlinePlus,
  TbOutlineClipboard,
  TbOutlineDeviceDesktop,
  TbOutlineServer,
  TbOutlineWifi,
  TbOutlineWifiOff
} from "solid-icons/tb";
import {
  DRAWER_LIST_ROW_CLASS,
  DRAWER_PANEL_SHELL_CLASS,
  DRAWER_SMALL_BUTTON_CLASS,
  DRAWER_STATE_BLOCK_CLASS,
  LOCAL_BRIDGE_LISTEN
} from "./workflow-overlay.constants";

export default function DevicesPanel(props) {
  return (
    <div class={DRAWER_PANEL_SHELL_CLASS}>
      <div class="border-b border-neutral-800 px-3 py-2">
        <p class="text-xs font-medium uppercase tracking-wide text-neutral-300">Devices</p>
        <p class="mt-1 text-[10px] text-neutral-500">
          Browser runtime mode reports only real, active browser-connected devices.
        </p>
      </div>
      <div class="min-h-0 flex-1 overflow-auto p-3">
        <div class="mb-3 flex items-center justify-between gap-2 rounded-md border border-neutral-800 bg-neutral-900/45 p-2.5">
          <div class="min-w-0">
            <p class="text-[10px] uppercase tracking-wide text-neutral-500">Device onboarding</p>
            <p class="mt-1 truncate text-[10px] text-neutral-500">Add a machine and generate its connect command.</p>
          </div>
          <button
            type="button"
            class={DRAWER_SMALL_BUTTON_CLASS}
            onClick={() => props.setShowDeviceConnectDialog(true)}
            data-testid="device-open-connect-dialog"
          >
            <TbOutlinePlus size={11} />
            Add device
          </button>
        </div>

        <Show when={props.showDeviceConnectDialog()}>
          <div
            class="fixed inset-0 z-[10040] flex items-center justify-center bg-black/50 px-4"
            data-testid="device-connect-dialog-backdrop"
            onClick={(event) => {
              if (event.target === event.currentTarget) props.setShowDeviceConnectDialog(false);
            }}
          >
            <div
              class="w-full max-w-xl rounded-xl border border-neutral-700 bg-[#101216] p-3 shadow-2xl"
              data-testid="device-connect-dialog"
              onClick={(event) => event.stopPropagation()}
            >
              <div class="mb-2 flex items-start justify-between gap-2">
                <div>
                  <p class="text-[11px] font-semibold uppercase tracking-wide text-neutral-200">Connect device</p>
                  <p class="mt-1 text-[10px] text-neutral-500">Choose platform and run the generated command on the target machine.</p>
                </div>
                <button
                  type="button"
                  class={DRAWER_SMALL_BUTTON_CLASS}
                  onClick={() => props.setShowDeviceConnectDialog(false)}
                  data-testid="device-connect-dialog-close"
                >
                  Close
                </button>
              </div>
              <div class="max-h-[72vh] overflow-auto pr-1">
                <div class="rounded-md border border-neutral-800 bg-neutral-900/60 p-2.5" data-testid="device-connect-block">
                  <div class="flex items-center gap-1.5">
                    <button
                      type="button"
                      class={props.cn(
                        DRAWER_SMALL_BUTTON_CLASS,
                        props.connectPlatform() === "linux" && "border-[hsl(var(--primary)/0.45)] text-[hsl(var(--primary))]"
                      )}
                      onClick={() => props.setConnectPlatform("linux")}
                      data-testid="device-platform-linux"
                    >
                      Linux
                    </button>
                    <button
                      type="button"
                      class={props.cn(DRAWER_SMALL_BUTTON_CLASS, "opacity-60")}
                      disabled
                      data-testid="device-platform-macos"
                    >
                      macOS (soon)
                    </button>
                    <button
                      type="button"
                      class={props.cn(DRAWER_SMALL_BUTTON_CLASS, "opacity-60")}
                      disabled
                      data-testid="device-platform-windows"
                    >
                      Windows (soon)
                    </button>
                  </div>
                  <Show when={props.connectPlatform() === "linux"}>
                    <label class="mt-2 block text-[10px] text-neutral-500">
                      Profile public key (base64url)
                      <input
                        type="text"
                        value={props.profilePublicKeyInput()}
                        onInput={(event) => props.setProfilePublicKeyInput(event.currentTarget.value)}
                        placeholder="paste profile public key"
                        class="mt-1 h-8 w-full rounded border border-neutral-700 bg-neutral-900 px-2 text-[11px] text-neutral-200 placeholder:text-neutral-500 focus:border-neutral-600 focus:outline-none"
                        data-testid="device-profile-public-key-input"
                      />
                    </label>
                    <label class="mt-2 block text-[10px] text-neutral-500">
                      Requested label (optional)
                      <input
                        type="text"
                        value={props.requestedLabelInput()}
                        onInput={(event) => props.setRequestedLabelInput(event.currentTarget.value)}
                        placeholder="alice"
                        class="mt-1 h-8 w-full rounded border border-neutral-700 bg-neutral-900 px-2 text-[11px] text-neutral-200 placeholder:text-neutral-500 focus:border-neutral-600 focus:outline-none"
                        data-testid="device-requested-label-input"
                      />
                    </label>
                    <div class="mt-2 flex items-center gap-1.5">
                      <button
                        type="button"
                        class={DRAWER_SMALL_BUTTON_CLASS}
                        onClick={props.reserveDomain}
                        disabled={props.reserveBusy()}
                        data-testid="device-reserve-domain"
                      >
                        {props.reserveBusy() ? "Reserving..." : "Reserve domain"}
                      </button>
                      <Show when={props.reserveStatus()}>
                        <span class="text-[10px] text-[hsl(var(--primary))]" data-testid="device-reserve-status">{props.reserveStatus()}</span>
                      </Show>
                    </div>
                    <Show when={props.reserveError()}>
                      <p class="mt-1 text-[10px] text-red-300" data-testid="device-reserve-error">{props.reserveError()}</p>
                    </Show>
                    <label class="mt-2 block text-[10px] text-neutral-500">
                      Domain
                      <input
                        type="text"
                        value={props.connectDomain()}
                        onInput={(event) => props.setConnectDomain(event.currentTarget.value)}
                        placeholder="alice.users.edgerun.tech"
                        class="mt-1 h-8 w-full rounded border border-neutral-700 bg-neutral-900 px-2 text-[11px] text-neutral-200 placeholder:text-neutral-500 focus:border-neutral-600 focus:outline-none"
                        data-testid="device-domain-input"
                      />
                    </label>
                    <label class="mt-2 block text-[10px] text-neutral-500">
                      Registration token
                      <input
                        type="text"
                        value={props.connectRegistrationToken()}
                        onInput={(event) => props.setConnectRegistrationToken(event.currentTarget.value)}
                        placeholder="paste registration token"
                        class="mt-1 h-8 w-full rounded border border-neutral-700 bg-neutral-900 px-2 text-[11px] text-neutral-200 placeholder:text-neutral-500 focus:border-neutral-600 focus:outline-none"
                        data-testid="device-registration-token-input"
                      />
                    </label>
                    <div class="mt-2 flex items-center gap-1.5">
                      <button
                        type="button"
                        class={DRAWER_SMALL_BUTTON_CLASS}
                        onClick={props.issuePairingCode}
                        disabled={props.pairingBusy()}
                        data-testid="device-issue-pairing-code"
                      >
                        {props.pairingBusy() ? "Issuing..." : "Issue pairing code"}
                      </button>
                      <Show when={props.pairingStatus()}>
                        <span class="text-[10px] text-[hsl(var(--primary))]" data-testid="device-pairing-status">{props.pairingStatus()}</span>
                      </Show>
                    </div>
                    <Show when={props.pairingError()}>
                      <p class="mt-1 text-[10px] text-red-300" data-testid="device-pairing-error">{props.pairingError()}</p>
                    </Show>
                    <Show when={props.pairingExpiresAt()}>
                      <p class="mt-1 text-[10px] text-neutral-500" data-testid="device-pairing-expiry">Expires: {props.pairingExpiresAt()}</p>
                    </Show>
                    <label class="mt-2 block text-[10px] text-neutral-500">
                      Pairing code
                      <input
                        type="text"
                        value={props.pairingCodeInput()}
                        onInput={(event) => props.setPairingCodeInput(event.currentTarget.value)}
                        placeholder="paste pairing code"
                        class="mt-1 h-8 w-full rounded border border-neutral-700 bg-neutral-900 px-2 text-[11px] text-neutral-200 placeholder:text-neutral-500 focus:border-neutral-600 focus:outline-none"
                        data-testid="device-pairing-code-input"
                      />
                    </label>
                    <pre
                      class="mt-2 overflow-x-auto rounded border border-neutral-800 bg-[#0c0c12] p-2 text-[10px] text-neutral-200"
                      data-testid="device-linux-script"
                    >
{props.linuxConnectScript()}
                    </pre>
                    <div class="mt-2 flex items-center gap-1.5">
                      <button
                        type="button"
                        class={DRAWER_SMALL_BUTTON_CLASS}
                        onClick={props.copyConnectScript}
                        data-testid="device-copy-script"
                      >
                        <TbOutlineClipboard size={11} />
                        {props.deviceConnectCopied() ? "Copied" : "Copy script"}
                      </button>
                      <span class="text-[10px] text-neutral-500">Local bridge: {LOCAL_BRIDGE_LISTEN}</span>
                    </div>
                  </Show>
                </div>
              </div>
            </div>
          </div>
        </Show>

        <Show when={props.fleetDevices().length > 0} fallback={<p class={DRAWER_STATE_BLOCK_CLASS}>No connected devices yet.</p>}>
          <div class="space-y-1.5">
            <For each={props.fleetDevices()}>
              {(device) => (
                <button
                  type="button"
                  onClick={() => props.setSelectedDeviceId(device.id)}
                  class={props.cn(
                    props.cn(DRAWER_LIST_ROW_CLASS, "text-left"),
                    props.selectedDeviceId() === device.id
                      ? "border-neutral-700 bg-neutral-900/85"
                      : ""
                  )}
                >
                  <div class="flex items-center justify-between gap-2">
                    <div class="flex items-center gap-2 min-w-0">
                      <Show
                        when={device.type === "host"}
                        fallback={<TbOutlineDeviceDesktop size={13} class="text-neutral-300" />}
                      >
                        <TbOutlineServer size={13} class="text-[hsl(var(--primary))]" />
                      </Show>
                      <p class={props.cn("truncate text-[11px] text-neutral-200", props.selectedDeviceId() === device.id ? "font-semibold text-[hsl(var(--primary))]" : "font-medium")}>{device.name || device.id}</p>
                    </div>
                    <div class="flex items-center gap-1">
                      <Show when={device.localAvailable} fallback={<TbOutlineWifiOff size={12} class={device.online ? "text-[hsl(var(--primary))]" : "text-neutral-500"} />}>
                        <TbOutlineWifi size={12} class="text-[hsl(var(--primary))]" />
                      </Show>
                      <span class={props.cn("inline-block h-2.5 w-2.5 rounded-full", device.online ? "bg-[hsl(var(--primary))]" : "bg-neutral-600")} />
                    </div>
                  </div>
                  <p class="mt-1 truncate text-[10px] text-neutral-500">
                    {device.primary?.ip || device.primary?.metadata?.host || "unknown"} · {device.members.length} source{device.members.length === 1 ? "" : "s"}
                  </p>
                </button>
              )}
            </For>
          </div>

          <Show when={props.selectedDevice()}>
            {(deviceAccessor) => {
              const device = deviceAccessor();
              const detail = device.primary || {};
              return (
                <div class="mt-3 rounded-md border border-neutral-800 bg-neutral-900/60 p-2 text-[11px] text-neutral-300">
                  <p class="mb-2 text-[10px] uppercase tracking-wide text-neutral-500">Details</p>
                  <div class="mb-2 flex flex-wrap items-center gap-2">
                    <button type="button" class={DRAWER_SMALL_BUTTON_CLASS} onClick={props.onOpenTerminal}>
                      Open Terminal
                    </button>
                    <button type="button" class={DRAWER_SMALL_BUTTON_CLASS} onClick={props.onOpenFiles}>
                      Open Files
                    </button>
                  </div>
                  <p><span class="text-neutral-500">ID:</span> {detail.id || device.id}</p>
                  <p><span class="text-neutral-500">Type:</span> {device.type || detail.type || "unknown"}</p>
                  <p><span class="text-neutral-500">OS:</span> {detail.os || "unknown"}</p>
                  <p><span class="text-neutral-500">IP:</span> {detail.ip || "unknown"}</p>
                  <p><span class="text-neutral-500">Connected:</span> {detail.connectedAt || "unknown"}</p>
                  <p><span class="text-neutral-500">Last seen:</span> {detail.lastSeenAt || "unknown"}</p>
                  <Show when={detail.metadata?.viewport}>
                    <p><span class="text-neutral-500">Viewport:</span> {detail.metadata.viewport}</p>
                  </Show>
                  <Show when={detail.metadata?.resources?.cpu}>
                    <p>
                      <span class="text-neutral-500">CPU:</span>{" "}
                      {detail.metadata.resources.cpu.cores || 0} cores · load{" "}
                      {(detail.metadata.resources.cpu.loadAvg || []).join(" / ")}
                    </p>
                  </Show>
                  <Show when={detail.metadata?.resources?.memory}>
                    <p>
                      <span class="text-neutral-500">Memory:</span>{" "}
                      {Math.round((Number(detail.metadata.resources.memory.used || 0) / 1024 / 1024 / 1024) * 10) / 10}G /{" "}
                      {Math.round((Number(detail.metadata.resources.memory.total || 0) / 1024 / 1024 / 1024) * 10) / 10}G
                    </p>
                  </Show>
                  <Show when={detail.metadata?.resources?.disk?.total}>
                    <p>
                      <span class="text-neutral-500">Disk:</span>{" "}
                      {Math.round((Number(detail.metadata.resources.disk.used || 0) / 1024 / 1024 / 1024) * 10) / 10}G /{" "}
                      {Math.round((Number(detail.metadata.resources.disk.total || 0) / 1024 / 1024 / 1024) * 10) / 10}G
                    </p>
                  </Show>
                  <Show when={detail.metadata?.capabilities}>
                    <div class="mt-2">
                      <p class="mb-1"><span class="text-neutral-500">Capabilities:</span></p>
                      <div class="flex flex-wrap gap-1">
                        <For each={Object.entries(detail.metadata.capabilities).filter(([, enabled]) => Boolean(enabled))}>
                          {([name]) => (
                            <span class="inline-flex items-center rounded border border-neutral-700 bg-neutral-800/70 px-1.5 py-0.5 text-[10px] text-neutral-200">
                              {name}
                            </span>
                          )}
                        </For>
                        <Show when={Object.entries(detail.metadata.capabilities).filter(([, enabled]) => Boolean(enabled)).length === 0}>
                          <span class="text-[10px] text-neutral-500">none</span>
                        </Show>
                      </div>
                    </div>
                  </Show>
                </div>
              );
            }}
          </Show>
        </Show>
      </div>
    </div>
  );
}
