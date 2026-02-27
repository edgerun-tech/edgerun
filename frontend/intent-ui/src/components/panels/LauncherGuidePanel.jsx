import { For, Show, createMemo, createSignal, onMount } from "solid-js";
import {
  TbOutlineApps,
  TbOutlineBook2,
  TbOutlineBrandGithub,
  TbOutlineCloud,
  TbOutlineCloudComputing,
  TbOutlineCommand,
  TbOutlineDeviceDesktop,
  TbOutlineFileText,
  TbOutlineKey,
  TbOutlineMail,
  TbOutlinePlayerPlay,
  TbOutlineSettings,
  TbOutlineTerminal2
} from "solid-icons/tb";
import { openWindow } from "../../stores/windows";
import { openWorkflowIntegrations, setAssistantProvider, toggleWorkflowDrawer, workflowUi } from "../../stores/workflow-ui";

const GUIDE_KEY = "intent-ui-guide-progress-v1";

const launchItems = [
  { id: "editor", label: "Editor", icon: TbOutlineCommand, run: () => openWindow("editor") },
  { id: "files", label: "Files", icon: TbOutlineFileText, run: () => openWindow("files") },
  { id: "terminal", label: "Terminal", icon: TbOutlineTerminal2, run: () => openWindow("terminal") },
  { id: "integrations", label: "Integrations", icon: TbOutlineCloudComputing, run: () => openWindow("integrations") },
  { id: "credentials", label: "Credentials", icon: TbOutlineKey, run: () => openWindow("credentials") },
  { id: "github", label: "GitHub Browser", icon: TbOutlineBrandGithub, run: () => openWindow("github") },
  { id: "email", label: "Email", icon: TbOutlineMail, run: () => openWindow("email") },
  { id: "cloud", label: "Cloud", icon: TbOutlineCloud, run: () => openWindow("cloud") },
  { id: "cloudflare", label: "Cloudflare", icon: TbOutlineCloudComputing, run: () => openWindow("cloudflare") },
  { id: "drive", label: "Drive", icon: TbOutlineCloud, run: () => openWindow("drive") },
  { id: "calendar", label: "Calendar", icon: TbOutlineBook2, run: () => openWindow("calendar") },
  { id: "browser", label: "Browser", icon: TbOutlinePlayerPlay, run: () => openWindow("browser") },
  { id: "call", label: "Call", icon: TbOutlineDeviceDesktop, run: () => openWindow("call") },
  { id: "settings", label: "Settings", icon: TbOutlineSettings, run: () => openWindow("settings") }
];

const drawerItems = [
  { id: "drawer-launcher", label: "Drawer Launcher", run: () => toggleWorkflowDrawer({ side: "left", panel: "launcher" }) },
  { id: "drawer-files", label: "Drawer Files", run: () => toggleWorkflowDrawer({ side: "left", panel: "files" }) },
  { id: "drawer-cloud", label: "Drawer Cloud", run: () => toggleWorkflowDrawer({ side: "left", panel: "cloud" }) },
  { id: "drawer-integrations", label: "Drawer Integrations", run: () => toggleWorkflowDrawer({ side: "left", panel: "integrations" }) },
  { id: "drawer-credentials", label: "Drawer Credentials", run: () => toggleWorkflowDrawer({ side: "left", panel: "credentials" }) },
  { id: "drawer-settings", label: "Drawer Settings", run: () => toggleWorkflowDrawer({ side: "left", panel: "settings" }) },
  { id: "drawer-conversations", label: "Drawer Conversations", run: () => toggleWorkflowDrawer({ side: "right", panel: "conversations" }) },
  { id: "drawer-devices", label: "Drawer Devices", run: () => toggleWorkflowDrawer({ side: "right", panel: "devices" }) }
];

const guideSteps = [
  { id: "open-intent", title: "Open command explorer", detail: "Type /help in IntentBar.", actionLabel: "Focus Intent", action: () => window.dispatchEvent(new CustomEvent("intentbar:toggle")) },
  { id: "connect-github", title: "Connect GitHub", detail: "Link GitHub token in Integrations.", actionLabel: "Open GitHub Link", action: () => openWorkflowIntegrations("github") },
  { id: "connect-google", title: "Connect Google", detail: "Enable Drive + Gmail + Calendar access.", actionLabel: "Open Google Link", action: () => openWorkflowIntegrations("google") },
  { id: "open-files", title: "Browse files", detail: "Open File Manager and pick a filesystem.", actionLabel: "Open Files", action: () => openWindow("files") },
  { id: "open-devices", title: "Check devices", detail: "Open Devices drawer and verify host telemetry.", actionLabel: "Open Devices", action: () => toggleWorkflowDrawer({ side: "right", panel: "devices" }) },
  { id: "open-terminal", title: "Run commands", detail: "Open terminal and run a quick command.", actionLabel: "Open Terminal", action: () => openWindow("terminal") }
];

function readGuideProgress() {
  if (typeof window === "undefined") return {};
  try {
    const parsed = JSON.parse(localStorage.getItem(GUIDE_KEY) || "{}");
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

function persistGuideProgress(value) {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(GUIDE_KEY, JSON.stringify(value));
  } catch {
    // ignore local storage failures
  }
}

function LauncherGuidePanel(props) {
  const [tab, setTab] = createSignal("launcher");
  const [progress, setProgress] = createSignal({});
  const compact = () => Boolean(props.compact);
  const completedCount = createMemo(() => guideSteps.filter((step) => Boolean(progress()[step.id])).length);
  const activeProvider = createMemo(() => workflowUi().provider || "codex");

  const toggleStep = (id) => {
    setProgress((prev) => {
      const next = { ...prev, [id]: !prev[id] };
      persistGuideProgress(next);
      return next;
    });
  };

  const resetGuide = () => {
    const empty = {};
    setProgress(empty);
    persistGuideProgress(empty);
  };

  onMount(() => {
    setProgress(readGuideProgress());
    if (props.initialTab === "guide") setTab("guide");
  });

  return (
    <div class={`flex h-full min-h-0 flex-col text-sm text-neutral-300 ${compact() ? "" : "p-2"}`}>
      <div class="mb-2 grid grid-cols-2 gap-1 rounded-md border border-neutral-800 bg-neutral-900/60 p-1">
        <button
          type="button"
          onClick={() => setTab("launcher")}
          class={`rounded px-2 py-1 text-[11px] transition-colors ${tab() === "launcher" ? "font-semibold text-[hsl(var(--primary))]" : "text-neutral-400 hover:bg-neutral-800"}`}
        >
          <span class="inline-flex items-center gap-1"><TbOutlineApps size={12} /> Launcher</span>
        </button>
        <button
          type="button"
          onClick={() => setTab("guide")}
          class={`rounded px-2 py-1 text-[11px] transition-colors ${tab() === "guide" ? "font-semibold text-[hsl(var(--primary))]" : "text-neutral-400 hover:bg-neutral-800"}`}
        >
          <span class="inline-flex items-center gap-1"><TbOutlineBook2 size={12} /> Guide</span>
        </button>
      </div>

      <Show when={tab() === "launcher"}>
        <div class="min-h-0 flex-1 overflow-auto space-y-1.5 pr-1">
          <div class="rounded-md border border-neutral-800 bg-neutral-900/55 p-2.5">
            <p class="mb-2 text-[10px] uppercase tracking-wide text-neutral-500">Assistant Provider</p>
            <div class="mb-2 grid grid-cols-2 gap-1">
              <button
                type="button"
                class={`inline-flex h-7 items-center justify-center rounded border px-2 text-[10px] transition-colors ${activeProvider() === "codex" ? "border-neutral-800 bg-neutral-900/70 font-semibold text-[hsl(var(--primary))]" : "border-neutral-800 bg-neutral-900/70 text-neutral-300 hover:bg-neutral-800"}`}
                onClick={() => setAssistantProvider("codex")}
              >
                Codex
              </button>
              <button
                type="button"
                class={`inline-flex h-7 items-center justify-center rounded border px-2 text-[10px] transition-colors ${activeProvider() === "qwen" ? "border-neutral-800 bg-neutral-900/70 font-semibold text-[hsl(var(--primary))]" : "border-neutral-800 bg-neutral-900/70 text-neutral-300 hover:bg-neutral-800"}`}
                onClick={() => setAssistantProvider("qwen")}
              >
                Qwen
              </button>
            </div>
            <button
              type="button"
              class="w-full rounded border border-neutral-800 bg-neutral-900/70 px-2 py-1.5 text-left text-[10px] text-neutral-200 transition-colors hover:bg-neutral-800"
              onClick={() => openWindow("integrations")}
            >
              Open AI Integrations
            </button>
          </div>

          <div class="rounded-md border border-neutral-800 bg-neutral-900/55 p-2.5">
            <p class="mb-2 text-[10px] uppercase tracking-wide text-neutral-500">Apps</p>
            <div class="grid grid-cols-2 gap-1.5">
              <For each={launchItems}>
                {(item) => (
                  <button
                    type="button"
                    onClick={item.run}
                    class="rounded border border-neutral-800 bg-neutral-900/70 px-2 py-1.5 text-left text-[10px] text-neutral-200 transition-colors hover:bg-neutral-800"
                  >
                    <span class="inline-flex items-center gap-1.5">
                      <item.icon size={12} class="text-neutral-400" />
                      {item.label}
                    </span>
                  </button>
                )}
              </For>
            </div>
          </div>

          <div class="rounded-md border border-neutral-800 bg-neutral-900/55 p-2.5">
            <p class="mb-2 text-[10px] uppercase tracking-wide text-neutral-500">Drawers</p>
            <div class="grid grid-cols-2 gap-1.5">
              <For each={drawerItems}>
                {(item) => (
                  <button
                    type="button"
                    onClick={item.run}
                    class="rounded border border-neutral-800 bg-neutral-900/70 px-2 py-1.5 text-left text-[10px] text-neutral-200 transition-colors hover:bg-neutral-800"
                  >
                    {item.label}
                  </button>
                )}
              </For>
            </div>
          </div>
        </div>
      </Show>

      <Show when={tab() === "guide"}>
        <div class="min-h-0 flex-1 overflow-auto space-y-1.5 pr-1">
          <div class="rounded-md border border-neutral-800 bg-neutral-900/55 p-2.5">
            <div class="mb-2 flex items-center justify-between gap-2">
              <p class="text-[10px] uppercase tracking-wide text-neutral-500">Interactive Onboarding</p>
              <button
                type="button"
                class="inline-flex h-6 items-center rounded border border-neutral-700 px-2 text-[10px] text-neutral-300 hover:bg-neutral-800"
                onClick={resetGuide}
              >
                Reset
              </button>
            </div>
            <p class="text-xs text-neutral-400">{completedCount()} / {guideSteps.length} completed</p>
          </div>

          <For each={guideSteps}>
            {(step) => (
              <div class="rounded-md border border-neutral-800 bg-neutral-900/60 p-2.5">
                <label class="flex items-start gap-2">
                  <input
                    type="checkbox"
                    class="mt-0.5"
                    style={{ "accent-color": "hsl(var(--primary))" }}
                    checked={Boolean(progress()[step.id])}
                    onInput={() => toggleStep(step.id)}
                  />
                  <div class="min-w-0">
                    <p class="text-[11px] font-medium text-neutral-100">{step.title}</p>
                    <p class="text-[10px] text-neutral-500">{step.detail}</p>
                  </div>
                </label>
                <button
                  type="button"
                  onClick={() => {
                    step.action();
                    toggleStep(step.id);
                  }}
                  class="mt-2 inline-flex h-7 items-center rounded border border-neutral-700 bg-neutral-900/70 px-2 text-[10px] text-neutral-200 hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
                >
                  {step.actionLabel}
                </button>
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}

export default LauncherGuidePanel;
