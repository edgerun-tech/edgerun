import { Show } from "solid-js";
import FileManager from "./FileManager";
import IntegrationsPanel from "./IntegrationsPanel";
import CloudPanel from "../panels/CloudPanel";
import CredentialsPanel from "../panels/CredentialsPanel";
import LauncherGuidePanel from "../panels/LauncherGuidePanel";
import SettingsPanel from "../panels/SettingsPanel";
import { DRAWER_PANEL_SHELL_CLASS } from "./workflow-overlay.constants";

export default function LeftDrawer(props) {
  return (
    <div class="min-h-0 flex-1 overflow-hidden">
      <Show when={props.state().leftPanel === "settings"}>
        <div class={DRAWER_PANEL_SHELL_CLASS}>
          <SettingsPanel compact />
        </div>
      </Show>
      <Show when={props.state().leftPanel === "launcher"}>
        <div class={DRAWER_PANEL_SHELL_CLASS}>
          <div class="border-b border-neutral-800 px-3 py-2">
            <div class="flex items-center justify-between gap-2">
              <p class="text-xs font-medium uppercase tracking-wide text-neutral-300">Launcher</p>
              <button
                type="button"
                class="rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-[10px] text-neutral-200 transition-colors hover:bg-neutral-800"
                onClick={props.onOpenGuide}
              >
                Open Guide
              </button>
            </div>
          </div>
          <div class="min-h-0 flex-1 overflow-auto p-2">
            <LauncherGuidePanel compact />
          </div>
        </div>
      </Show>
      <Show when={props.state().leftPanel === "files"}>
        <div class={DRAWER_PANEL_SHELL_CLASS}>
          <FileManager compact />
        </div>
      </Show>
      <Show when={props.state().leftPanel === "cloud"}>
        <div class={DRAWER_PANEL_SHELL_CLASS}>
          <CloudPanel compact />
        </div>
      </Show>
      <Show when={props.state().leftPanel === "integrations"}>
        <div class={DRAWER_PANEL_SHELL_CLASS}>
          <IntegrationsPanel compact preselectProviderId={props.state().selectedIntegrationId || ""} />
        </div>
      </Show>
      <Show when={props.state().leftPanel === "credentials"}>
        <div class={DRAWER_PANEL_SHELL_CLASS}>
          <CredentialsPanel compact />
        </div>
      </Show>
    </div>
  );
}
