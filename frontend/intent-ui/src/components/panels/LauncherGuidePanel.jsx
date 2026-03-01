import { For, Show, createMemo, createSignal, onMount } from "solid-js";
import {
  TbOutlineApps,
  TbOutlineBook2,
  TbOutlineBrandGithub,
  TbOutlineCheck,
  TbOutlineCloud,
  TbOutlineCloudComputing,
  TbOutlineCommand,
  TbOutlineDeviceDesktop,
  TbOutlineFileText,
  TbOutlineKey,
  TbOutlineMail,
  TbOutlinePlayerPlay,
  TbOutlineSettings,
  TbOutlineTerminal2,
  TbOutlineUser
} from "solid-icons/tb";
import { openWindow } from "../../stores/windows";
import { toggleWorkflowDrawer, workflowUi, openWorkflowIntegrations } from "../../stores/workflow-ui";
import { knownDevices } from "../../stores/devices";
import { integrationStore } from "../../stores/integrations";
import { profileRuntime } from "../../stores/profile-runtime";
import { openProfileBootstrap, toggleIntentBar } from "../../stores/ui-actions";

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

const guideSteps = [
  { id: "open-intent", title: "Open command explorer", detail: "Type /help in IntentBar.", actionLabel: "Focus Intent", action: () => toggleIntentBar() },
  { id: "open-onboarding", title: "Review onboarding", detail: "Reopen profile onboarding and verify session mode.", actionLabel: "Open Onboarding", action: () => openProfileBootstrap() },
  { id: "connect-github", title: "Connect GitHub", detail: "Link GitHub token in Integrations.", actionLabel: "Open GitHub Link", action: () => openWorkflowIntegrations("github") },
  { id: "connect-assistant", title: "Connect assistant integration", detail: "Link OpenCode CLI before running assistant tasks.", actionLabel: "Open AI Integration", action: () => openWorkflowIntegrations("opencode_cli") },
  { id: "open-devices", title: "Check devices", detail: "Verify a connected device is online.", actionLabel: "Open Devices", action: () => toggleWorkflowDrawer({ side: "right", panel: "devices" }) },
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
  const activeProvider = createMemo(() => workflowUi().provider || "opencode");
  const assistantIntegrationId = createMemo(() => activeProvider() === "opencode" ? "opencode_cli" : "opencode_cli");
  const assistantIntegration = createMemo(() => integrationStore.get(assistantIntegrationId()));
  const anyConnectedDevice = createMemo(() => knownDevices().some((device) => Boolean(device?.online)));
  const hasConnectedToolingIntegration = createMemo(() => integrationStore.list()
    .filter((integration) => integration.id !== "opencode_cli")
    .some((integration) => integration.connected));

  const startupTasks = createMemo(() => {
    const onboardingDone = profileRuntime().ready;
    return [
      {
        id: "task-onboarding",
        title: "Onboarding access",
        detail: onboardingDone ? "Onboarding can be reopened from this launcher or top-right button." : "Finish onboarding to create or load your encrypted profile.",
        done: onboardingDone,
        actionLabel: "Open onboarding",
        action: () => openProfileBootstrap()
      },
      {
        id: "task-devices",
        title: "First device connection",
        detail: anyConnectedDevice()
          ? "At least one device is online."
          : "Open Devices, choose Linux, copy script, and run it on your target machine.",
        done: anyConnectedDevice(),
        actionLabel: "Open connect flow",
        action: () => toggleWorkflowDrawer({ side: "right", panel: "devices" })
      },
      {
        id: "task-integrations",
        title: "Tool integrations",
        detail: hasConnectedToolingIntegration() ? "One or more tooling integrations are connected." : "Connect GitHub/Cloud/other integrations for workflows.",
        done: hasConnectedToolingIntegration(),
        actionLabel: "Open integrations",
        action: () => openWindow("integrations")
      },
      {
        id: "task-assistant",
        title: "Assistant integration",
        detail: assistantIntegration()?.connected
          ? `${assistantIntegration()?.name || "Assistant"} integration connected.`
          : `Connect ${assistantIntegration()?.name || "assistant"} integration first.`,
        done: Boolean(assistantIntegration()?.connected),
        actionLabel: "Configure assistant",
        action: () => openWorkflowIntegrations(assistantIntegrationId())
      }
    ];
  });

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
            <p class="mb-2 text-[10px] uppercase tracking-wide text-neutral-500">Startup Tasks</p>
            <div class="space-y-1.5">
              <For each={startupTasks()}>
                {(task) => (
                  <div class="rounded border border-neutral-800 bg-neutral-900/70 px-2 py-1.5">
                    <div class="flex items-start justify-between gap-2">
                      <div class="min-w-0">
                        <p class="flex items-center gap-1 text-[11px] font-medium text-neutral-100">
                          <Show when={task.done} fallback={<TbOutlineUser size={12} class="text-neutral-500" />}>
                            <TbOutlineCheck size={12} class="text-[hsl(var(--primary))]" />
                          </Show>
                          {task.title}
                        </p>
                        <p class="mt-0.5 text-[10px] text-neutral-500">{task.detail}</p>
                      </div>
                      <button
                        type="button"
                        onClick={task.action}
                        class="shrink-0 rounded border border-neutral-800 bg-neutral-900 px-2 py-1 text-[10px] text-neutral-200 hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
                      >
                        {task.actionLabel}
                      </button>
                    </div>
                  </div>
                )}
              </For>
            </div>
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
