import { For, Show, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { IntentBar } from "./components/panels";
import { WindowManager, WorkflowOverlay } from "./components/layout";
import FloatingFeedPanel from "./components/layout/FloatingFeedPanel";
import WallpaperWidgets from "./components/layout/WallpaperWidgets";
import WallpaperMap from "./components/layout/WallpaperMap";
import ProfileBootstrapGate from "./components/onboarding/ProfileBootstrapGate";
import { localBridgeHttpUrl, localBridgeWsUrl } from "./lib/local-bridge-origin";
import { closeTopWindow, openWindow } from "./stores/windows";
import { closeWorkflowDemo, hydrateWorkflowUiFromStorage, openCodexResponse, startNewCodexSession, workflowUi } from "./stores/workflow-ui";
import { clearProfileRuntimeSession, hydrateProfileRuntime, profileRuntime } from "./stores/profile-runtime";
import { clearProfileSecretsContext } from "./stores/profile-secrets";
import { eventBusRuntime, eventTimeline, initializeBrowserEventBus, publishEvent, retryLocalBridgeConnection, subscribeEvent } from "./stores/eventbus";
import { UI_EVENT_TOPICS } from "./lib/ui-intents";
import { initializeUiActionBridge, toggleIntentBar } from "./stores/ui-actions";
import { pushClipboardEntry } from "./stores/clipboard-history";
import { Kbd, KbdGroup } from "./registry/ui/kbd";
import {
  TbOutlineCopy,
  TbOutlineClipboard,
  TbOutlineFileText,
  TbOutlineCode,
  TbOutlineTerminal,
  TbOutlineKey,
  TbOutlineApps,
  TbOutlineDeviceDesktop,
  TbOutlineHistory,
  TbOutlineRefresh,
  TbOutlineBook2,
  TbOutlineChevronDown
} from "solid-icons/tb";

function App() {
  const localBridgeWsEndpoint = localBridgeWsUrl("/v1/local/eventbus/ws");
  const [inputLayer, setInputLayer] = createSignal(1);
  const [isClient, setIsClient] = createSignal(false);
  const [showLayerIndicator, setShowLayerIndicator] = createSignal(false);
  const [menuOpen, setMenuOpen] = createSignal(false);
  const [menuPos, setMenuPos] = createSignal({ x: 0, y: 0 });
  const [clipboardText, setClipboardText] = createSignal("");
  const [copyText, setCopyText] = createSignal("");
  const [contextTarget, setContextTarget] = createSignal(null);
  const [showBootstrapGate, setShowBootstrapGate] = createSignal(false);
  const [accountMenuOpen, setAccountMenuOpen] = createSignal(false);
  const [registeredDomain, setRegisteredDomain] = createSignal("");
  const [dockerLogLines, setDockerLogLines] = createSignal([]);
  const canPaste = createMemo(() => clipboardText().trim().length > 0);
  const canCopy = createMemo(() => copyText().trim().length > 0);
  const latestEventBusItems = createMemo(() => (
    [...eventTimeline()].slice(-20).reverse()
  ));
  const latestDockerLogItems = createMemo(() => (
    [...dockerLogLines()].slice(0, 20)
  ));
  const localBridgeError = createMemo(() => {
    if (!isClient()) return "";
    if (eventBusRuntime().localBridgeConnected) return "";
    if (eventBusRuntime().localBridgeStatus !== "error") return "";
    return String(eventBusRuntime().localBridgeError || "Can't connect to local bridge, is it running?");
  });
  const sessionModeLabel = createMemo(() => (
    profileRuntime().mode === "profile" && profileRuntime().profileLoaded
      ? `profile (${profileRuntime().backend})`
      : "profile required"
  ));
  const shortProfileId = createMemo(() => {
    const id = String(profileRuntime().profileId || "").trim();
    if (!id) return "Not loaded";
    if (id.length <= 18) return id;
    return `${id.slice(0, 8)}...${id.slice(-6)}`;
  });
  const formatEventPayload = (payload) => {
    if (!payload || typeof payload !== "object" || Array.isArray(payload)) return "";
    const keys = Object.keys(payload);
    if (keys.length === 0) return "";
    const preview = {};
    for (const key of keys.slice(0, 3)) preview[key] = payload[key];
    try {
      return JSON.stringify(preview);
    } catch {
      return "";
    }
  };
  const formatEventAge = (createdAt) => {
    const raw = createdAt ?? Date.now();
    let ts = Number.NaN;
    if (typeof raw === "number") {
      ts = raw;
    } else if (typeof raw === "string" && /^\d+$/.test(raw.trim())) {
      const numeric = Number(raw.trim());
      ts = numeric < 1_000_000_000_000 ? numeric * 1000 : numeric;
    } else {
      ts = new Date(raw).getTime();
    }
    if (!Number.isFinite(ts)) return "just now";
    const delta = Math.max(0, Math.floor((Date.now() - ts) / 1000));
    if (delta < 5) return "just now";
    if (delta < 60) return `${delta}s ago`;
    const minutes = Math.floor(delta / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  };
  const defaultFloatingLayouts = createMemo(() => {
    const viewportWidth = typeof window === "undefined" ? 1360 : window.innerWidth;
    const viewportHeight = typeof window === "undefined" ? 900 : window.innerHeight;
    const panelWidth = 320;
    const panelHeight = 184;
    const gap = 12;
    return {
      eventBus: {
        x: Math.max(12, viewportWidth - panelWidth - 12),
        y: Math.max(12, viewportHeight - panelHeight - 96),
        width: panelWidth,
        height: panelHeight
      },
      dockerLogs: {
        x: Math.max(12, viewportWidth - (panelWidth * 2) - gap - 12),
        y: Math.max(12, viewportHeight - panelHeight - 96),
        width: panelWidth,
        height: panelHeight
      }
    };
  });
  const fetchDockerLogs = async () => {
    try {
      const response = await fetch(localBridgeHttpUrl("/v1/local/docker/logs?limit=120&tail_per_container=25"), {
        cache: "no-store"
      });
      const payload = await response.json().catch(() => ({}));
      if (!response.ok || !payload?.ok || !Array.isArray(payload?.lines)) return;
      const lines = payload.lines
        .filter((item) => item && typeof item === "object")
        .map((item, index) => ({
          id: `${item.container_id || item.containerId || "container"}-${item.timestamp || item.ts || Date.now()}-${index}`,
          containerName: String(item.container_name || item.containerName || item.container_id || item.containerId || "container"),
          message: String(item.message || "").trim(),
          timestamp: String(item.timestamp || item.ts || ""),
          stream: String(item.stream || item.level || "")
        }))
        .filter((item) => item.message.length > 0);
      setDockerLogLines(lines);
    } catch {
      // keep last known logs on transient bridge/network errors
    }
  };
  let handleGlobalHotkeys;
  let handleGlobalKeyUp;
  let handleWindowBlur;
  let handleGlobalContextMenu;
  let handleGlobalClick;
  let handleContextEscape;
  let handleOpenBootstrapGate;
  let handleStorageUpdate;
  let dockerLogsTimer;
  let layerIndicatorTimeout;
  let unsubscribeBootstrapOpen;

  const readRegisteredDomain = () => {
    if (typeof window === "undefined") return "";
    const keys = [
      "intent-ui-user-domain-v1",
      "intent-ui-domain-v1",
      "intent-ui-domain-reservation-v1",
      "edgerun_user_domain"
    ];
    for (const key of keys) {
      const raw = localStorage.getItem(key);
      if (!raw) continue;
      const trimmed = String(raw).trim();
      if (!trimmed) continue;
      if (trimmed.startsWith("{")) {
        try {
          const parsed = JSON.parse(trimmed);
          const value = String(parsed?.domain || parsed?.assignedDomain || parsed?.fqdn || "").trim();
          if (value) return value;
        } catch {
          // ignore parse failures
        }
      } else {
        return trimmed;
      }
    }
    return "";
  };

  const resolveCopyText = (target) => {
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
      const start = target.selectionStart ?? 0;
      const end = target.selectionEnd ?? start;
      if (end > start) return target.value.slice(start, end);
    }
    const selected = window.getSelection()?.toString() || "";
    return selected;
  };

  const pasteIntoTarget = (target, text) => {
    if (!target || !text) return false;
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
      const start = target.selectionStart ?? target.value.length;
      const end = target.selectionEnd ?? start;
      target.focus();
      target.setRangeText(text, start, end, "end");
      target.dispatchEvent(new Event("input", { bubbles: true }));
      return true;
    }
    if (target instanceof HTMLElement && target.isContentEditable) {
      target.focus();
      document.execCommand("insertText", false, text);
      return true;
    }
    return false;
  };

  const readClipboardText = async () => {
    if (!navigator.clipboard?.readText) {
      setClipboardText("");
      return;
    }
    try {
      const text = await navigator.clipboard.readText();
      setClipboardText(String(text || ""));
      pushClipboardEntry(text, "clipboard-read");
    } catch {
      setClipboardText("");
    }
  };

  const contextActions = [
    { label: "Open Files", icon: TbOutlineFileText, run: () => openWindow("files") },
    { label: "Open Editor", icon: TbOutlineCode, run: () => openWindow("editor") },
    { label: "Open Terminal", icon: TbOutlineTerminal, run: () => openWindow("terminal") },
    { label: "Open Credentials", icon: TbOutlineKey, run: () => openWindow("credentials") },
    { label: "Open ONVIF", icon: TbOutlineDeviceDesktop, run: () => openWindow("onvif") },
    { label: "Open Widgets", icon: TbOutlineApps, run: () => openWindow("widgets") },
    {
      label: "New Codex Session",
      icon: TbOutlineHistory,
      run: () => {
        startNewCodexSession();
        toggleIntentBar();
      }
    },
    { label: "Open Guide", icon: TbOutlineBook2, run: () => openWindow("guide") },
    { label: "Reload UI", icon: TbOutlineRefresh, run: () => window.location.reload() }
  ];
  const contextMenuActions = createMemo(() => {
    const actions = [];
    if (canCopy()) {
      actions.push({
        label: "Copy",
        icon: TbOutlineCopy,
        run: async () => {
          try {
            await navigator.clipboard?.writeText(copyText());
            pushClipboardEntry(copyText(), "copy-action");
            publishEvent("clipboard.copied", { text: copyText() }, { source: "intent-ui" });
          } catch {
            // ignore clipboard permission failures
          }
        }
      });
    }
    if (canPaste()) {
      actions.push({
        label: "Paste",
        icon: TbOutlineClipboard,
        run: async () => {
          const text = clipboardText();
          if (!text) return;
          const target = contextTarget() || document.activeElement;
          const inserted = pasteIntoTarget(target, text);
          pushClipboardEntry(text, "paste-action");
          publishEvent("clipboard.pasted", { text }, { source: "intent-ui" });
          if (!inserted && document.activeElement instanceof HTMLElement && document.activeElement.isContentEditable) {
            document.execCommand("insertText", false, text);
          }
        }
      });
    }
    if (actions.length > 0) {
      actions.push({ label: "__sep__", run: () => {} });
    }
    actions.push(...contextActions);
    return actions;
  });

  onMount(() => {
    hydrateProfileRuntime();
    hydrateWorkflowUiFromStorage();
    void initializeBrowserEventBus();
    initializeUiActionBridge();
    publishEvent("browser.runtime.started", { app: "intent-ui" }, { source: "browser" });
    void fetchDockerLogs();
    dockerLogsTimer = window.setInterval(() => {
      void fetchDockerLogs();
    }, 3000);
    setIsClient(true);
    const updateLayerFromEvent = (event) => {
      if (layerIndicatorTimeout) {
        clearTimeout(layerIndicatorTimeout);
        layerIndicatorTimeout = null;
      }
      if (event.altKey) {
        setInputLayer(2);
        setShowLayerIndicator(true);
        return;
      }
      if (event.ctrlKey) {
        setInputLayer(0);
        setShowLayerIndicator(true);
        return;
      }
      setInputLayer(1);
      setShowLayerIndicator(true);
      layerIndicatorTimeout = window.setTimeout(() => {
        setShowLayerIndicator(false);
        layerIndicatorTimeout = null;
      }, 1000);
    };

    handleGlobalHotkeys = (event) => {
      updateLayerFromEvent(event);
      const target = event.target;
      if (
        target instanceof HTMLElement &&
        (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable) &&
        event.key !== "Meta"
      ) {
        return;
      }

      const isSuperKey =
        (event.key === "Meta" || event.code === "MetaLeft" || event.code === "MetaRight") &&
        !event.repeat &&
        !event.shiftKey &&
        !event.ctrlKey &&
        !event.altKey;

      if (isSuperKey) {
        event.preventDefault();
        toggleIntentBar();
        return;
      }

      if (event.key === "Escape") {
        if (workflowUi().isOpen) {
          closeWorkflowDemo();
          return;
        }
        closeTopWindow();
      }
    };

    handleGlobalKeyUp = (event) => {
      updateLayerFromEvent(event);
    };
    handleWindowBlur = () => {
      setInputLayer(1);
      setShowLayerIndicator(false);
      if (layerIndicatorTimeout) {
        clearTimeout(layerIndicatorTimeout);
        layerIndicatorTimeout = null;
      }
    };
    handleGlobalContextMenu = (event) => {
      event.preventDefault();
    };
    handleGlobalClick = () => {
      setMenuOpen(false);
      setAccountMenuOpen(false);
    };
    handleContextEscape = (event) => {
      if (event.key === "Escape") {
        setMenuOpen(false);
      }
    };

    window.addEventListener("keydown", handleGlobalHotkeys);
    window.addEventListener("keyup", handleGlobalKeyUp);
    window.addEventListener("blur", handleWindowBlur);
    window.addEventListener("contextmenu", handleGlobalContextMenu, { capture: true });
    window.addEventListener("pointerdown", handleGlobalClick);
    window.addEventListener("keydown", handleContextEscape);
    handleOpenBootstrapGate = (event) => {
      if (!event?.payload?.manual) return;
      setShowBootstrapGate(true);
    };
    unsubscribeBootstrapOpen = subscribeEvent(UI_EVENT_TOPICS.action.profileBootstrapOpened, handleOpenBootstrapGate);
    handleStorageUpdate = () => setRegisteredDomain(readRegisteredDomain());
    window.addEventListener("storage", handleStorageUpdate);
    setRegisteredDomain(readRegisteredDomain());
    if (typeof window !== "undefined") {
      window.__intentDebug = window.__intentDebug || {};
      window.__intentDebug.openWindow = (id) => openWindow(id);
      window.__intentDebug.askAssistant = (prompt, options = {}) => openCodexResponse(prompt, options);
      window.__intentDebug.getWorkflowUi = () => workflowUi();
      window.__intentDebug.getEventBusRuntime = () => eventBusRuntime();
      window.__intentDebug.getEventBusTimeline = () => eventTimeline();
      window.__intentDebug.publishEvent = (topic, payload = {}) => publishEvent(topic, payload, { source: "intent-debug" });
    }
  });
  onCleanup(() => {
    if (handleGlobalHotkeys) window.removeEventListener("keydown", handleGlobalHotkeys);
    if (handleGlobalKeyUp) window.removeEventListener("keyup", handleGlobalKeyUp);
    if (handleWindowBlur) window.removeEventListener("blur", handleWindowBlur);
    if (handleGlobalContextMenu) window.removeEventListener("contextmenu", handleGlobalContextMenu, { capture: true });
    if (handleGlobalClick) window.removeEventListener("pointerdown", handleGlobalClick);
    if (handleContextEscape) window.removeEventListener("keydown", handleContextEscape);
    if (unsubscribeBootstrapOpen) unsubscribeBootstrapOpen();
    if (handleStorageUpdate) window.removeEventListener("storage", handleStorageUpdate);
    if (dockerLogsTimer) {
      window.clearInterval(dockerLogsTimer);
      dockerLogsTimer = null;
    }
    if (layerIndicatorTimeout) {
      clearTimeout(layerIndicatorTimeout);
      layerIndicatorTimeout = null;
    }
    if (typeof window !== "undefined" && window.__intentDebug?.openWindow) {
      delete window.__intentDebug.openWindow;
      delete window.__intentDebug.askAssistant;
      delete window.__intentDebug.getWorkflowUi;
      delete window.__intentDebug.getEventBusRuntime;
      delete window.__intentDebug.getEventBusTimeline;
      delete window.__intentDebug.publishEvent;
    }
  });

  const AppShell = () => (
    <div
      class="relative min-h-screen overflow-hidden bg-[#090909] text-foreground"
      data-input-layer={inputLayer()}
      onContextMenu={(event) => {
        event.preventDefault();
        event.stopPropagation();
        setContextTarget(event.target instanceof Element ? event.target : null);
        setCopyText(resolveCopyText(event.target));
        void readClipboardText();
        setMenuPos({ x: event.clientX, y: event.clientY });
        setMenuOpen(true);
      }}
    >
      <div class="pointer-events-none absolute inset-0 opacity-70" style={{
        background:
          "radial-gradient(1200px 700px at 20% -10%, rgba(38,78,125,0.24), transparent), radial-gradient(900px 560px at 88% 115%, rgba(64,42,101,0.2), transparent)"
      }} />
      <Show when={isClient()}>
        <WallpaperMap />
      </Show>
      <Show when={isClient()}>
        <WallpaperWidgets />
      </Show>
      <Show when={showLayerIndicator()}>
      <div class="pointer-events-none fixed bottom-3 left-3 z-[12000]">
        <div class="flex items-center gap-2 rounded-lg border border-neutral-800/80 bg-[#0f0f0f]/70 px-2 py-1.5 shadow-lg backdrop-blur-xl">
          <KbdGroup class="gap-1">
            <Kbd
              class={`transition ${inputLayer() === 0 ? "border-[hsl(var(--primary))]/60 bg-[hsl(var(--primary))]/20 text-[hsl(var(--primary))] opacity-100" : "border-neutral-700 text-neutral-300 opacity-50"}`}
            >
              0
            </Kbd>
            <Kbd
              class={`transition ${inputLayer() === 1 ? "border-[hsl(var(--primary))]/60 bg-[hsl(var(--primary))]/20 text-[hsl(var(--primary))] opacity-100" : "border-neutral-700 text-neutral-300 opacity-50"}`}
            >
              1
            </Kbd>
            <Kbd
              class={`transition ${inputLayer() === 2 ? "border-[hsl(var(--primary))]/60 bg-[hsl(var(--primary))]/20 text-[hsl(var(--primary))] opacity-100" : "border-neutral-700 text-neutral-300 opacity-50"}`}
            >
              2
            </Kbd>
          </KbdGroup>
          <span class="text-[10px] uppercase tracking-wide text-neutral-500">Layer</span>
        </div>
      </div>
      </Show>
      <Show when={profileRuntime().ready}>
        <div class="fixed right-3 top-3 z-[12000] flex items-start justify-end">
          <span class="hidden" data-testid="profile-runtime-mode">Session mode: {sessionModeLabel()}</span>
          <div class="relative">
            <button
              type="button"
              class="inline-flex h-10 items-center gap-2 rounded-full border border-neutral-800/80 bg-[#0f0f0f]/75 px-3 text-xs text-neutral-200 shadow-lg backdrop-blur-xl transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
              onPointerDown={(event) => event.stopPropagation()}
              onClick={() => setAccountMenuOpen((prev) => !prev)}
              data-testid="account-circle-trigger"
            >
              <span class="inline-flex h-6 w-6 items-center justify-center rounded-full border border-neutral-700 bg-neutral-900 text-[10px] font-semibold">ER</span>
              <span class="uppercase tracking-wide">Account</span>
              <TbOutlineChevronDown size={12} />
            </button>
            <Show when={accountMenuOpen()}>
              <div
                class="absolute right-0 top-full mt-2 w-72 rounded-xl border border-neutral-700 bg-[#101216]/95 p-3 shadow-2xl backdrop-blur-xl"
                onPointerDown={(event) => event.stopPropagation()}
                data-testid="account-circle-menu"
              >
                <p class="text-[10px] uppercase tracking-wide text-neutral-500">Session Mode</p>
                <p class="mt-1 text-xs text-neutral-200" data-testid="profile-runtime-mode-menu">{sessionModeLabel()}</p>
                <div class="mt-2 space-y-1 rounded-md border border-neutral-800 bg-neutral-900/60 p-2 text-[11px] text-neutral-400">
                  <p>Profile: <span class="text-neutral-200">{shortProfileId()}</span></p>
                  <p>Backend: <span class="text-neutral-200">{profileRuntime().backend || "not linked"}</span></p>
                  <p data-testid="account-domain-value">Domain: <span class="text-neutral-200">{registeredDomain() || "Not registered"}</span></p>
                </div>
                <div class="mt-3 grid grid-cols-1 gap-1.5">
                <button
                  type="button"
                  class="inline-flex items-center justify-center rounded border border-neutral-700 bg-neutral-900 px-2 py-1.5 text-[11px] text-neutral-200 hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
                  onClick={() => {
                    clearProfileRuntimeSession();
                    clearProfileSecretsContext();
                    setShowBootstrapGate(true);
                    setAccountMenuOpen(false);
                  }}
                  data-testid="account-reset-session"
                >
                  Reset session
                </button>
                </div>
              </div>
            </Show>
          </div>
        </div>
      </Show>
      <Show
        when={!localBridgeError()}
        fallback={
          <div class="fixed inset-0 z-[14000] flex items-center justify-center bg-[#070709]/95 px-4 py-6">
            <div class="w-full max-w-md rounded-xl border border-red-500/35 bg-[#16171d] p-4 shadow-2xl">
              <p class="text-[10px] uppercase tracking-wide text-red-300">Local Bridge Required</p>
              <h2 class="mt-1 text-base font-semibold text-neutral-100">{localBridgeError()}</h2>
              <p class="mt-2 text-xs text-neutral-400">
                Intent UI requires a local bridge at <code class="text-neutral-200">{localBridgeWsEndpoint}</code>.
              </p>
              <button
                type="button"
                class="mt-3 inline-flex items-center justify-center rounded border border-red-400/50 bg-red-500/10 px-3 py-1.5 text-xs text-red-200 hover:bg-red-500/20"
                onClick={() => retryLocalBridgeConnection()}
                data-testid="retry-local-bridge"
              >
                Retry bridge connection
              </button>
            </div>
          </div>
        }
      >
        <WindowManager />
        <FloatingFeedPanel
          panelId="event-bus"
          title="EVENT BUS"
          maxItems={20}
          minWidth={280}
          minHeight={150}
          defaultLayout={defaultFloatingLayouts().eventBus}
          entries={latestEventBusItems}
          emptyLabel="No bridge events yet."
          renderEntry={(event) => (
            <>
              <p class="truncate text-[10px] text-white">
                {event.topic || "event.unknown"} · {formatEventAge(event.createdAt)}
              </p>
              <Show when={formatEventPayload(event.payload)}>
                {(preview) => <p class="truncate font-mono text-[9px] text-white">{preview()}</p>}
              </Show>
            </>
          )}
        />
        <FloatingFeedPanel
          panelId="docker-logs"
          title="DOCKER LOGS"
          maxItems={20}
          minWidth={280}
          minHeight={150}
          defaultLayout={defaultFloatingLayouts().dockerLogs}
          entries={latestDockerLogItems}
          emptyLabel="No docker logs yet."
          renderEntry={(line) => (
            <p class="truncate text-[10px] text-white">
              [{line.containerName}] {line.message} · {formatEventAge(line.timestamp)}
            </p>
          )}
        />
        <IntentBar />
        <WorkflowOverlay />
      </Show>
    </div>
  );

  return (
    <>
      <AppShell />
      <Show when={isClient() && (!profileRuntime().ready || showBootstrapGate())}>
        <ProfileBootstrapGate
          allowDismiss={profileRuntime().ready}
          onDismiss={() => setShowBootstrapGate(false)}
          onComplete={() => {
            hydrateProfileRuntime();
            setShowBootstrapGate(false);
          }}
        />
      </Show>
      <Show when={isClient() && menuOpen()}>
        <div
          class="fixed z-[13000] w-56 overflow-hidden rounded-lg border border-neutral-700 bg-[#121218]/95 p-1 shadow-2xl backdrop-blur-xl"
          style={{ left: `${menuPos().x}px`, top: `${menuPos().y}px` }}
          onPointerDown={(event) => event.stopPropagation()}
        >
          <p class="px-2 py-1 text-[11px] uppercase tracking-wide text-neutral-500">IntentUI Actions</p>
          <div class="my-1 h-px bg-neutral-800" />
          <For each={contextMenuActions()}>
            {(action) => (
              <Show
                when={action.label !== "__sep__"}
                fallback={<div class="my-1 h-px bg-neutral-800" />}
              >
              <button
                type="button"
                class="flex w-full items-center rounded-md px-2 py-1.5 text-left text-sm text-neutral-200 transition-colors hover:bg-neutral-800"
                onClick={async () => {
                  setMenuOpen(false);
                  await action.run();
                }}
              >
                <Show when={action.icon}>
                  {(Icon) => <Icon size={14} class="mr-2 text-neutral-400" />}
                </Show>
                {action.label}
              </button>
              </Show>
            )}
          </For>
        </div>
      </Show>
    </>
  );
}

export default App;
