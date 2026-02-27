import { Show, For, createMemo, createSignal, createEffect, onCleanup } from "solid-js";
import { Motion } from "solid-motionone";
import { Portal } from "solid-js/web";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import {
  TbOutlineFileText,
  TbOutlineCloud,
  TbOutlineSettings,
  TbOutlineBook2,
  TbOutlineCommand,
  TbOutlinePlus,
  TbOutlineUser,
  TbOutlineDeviceDesktop,
  TbOutlineKey,
  TbOutlineServer,
  TbOutlineRefresh,
  TbOutlineWifi,
  TbOutlineWifiOff,
  TbOutlineBluetooth,
  TbOutlineNetwork,
  TbOutlineScan
} from "solid-icons/tb";
import { FiLink2 } from "solid-icons/fi";
import { CodeDiffViewer } from "../results/CodeDiffViewer";
import FileManager from "./FileManager";
import IntegrationsPanel from "./IntegrationsPanel";
import CloudPanel from "../panels/CloudPanel";
import CredentialsPanel from "../panels/CredentialsPanel";
import LauncherGuidePanel from "../panels/LauncherGuidePanel";
import SettingsPanel from "../panels/SettingsPanel";
import {
  closeWorkflowDemo,
  startNewCodexSession,
  setWorkflowCode,
  toggleWorkflowDrawer,
  triggerWorkflowCommit,
  useWorkflowSession,
  workflowUi
} from "../../stores/workflow-ui";
import {
  CURRENT_DEVICE_ID,
  knownDevices,
  runDiscoveryScan,
  scanSettings,
  scanState,
  setScannerEnabled
} from "../../stores/devices";
import { openWindow } from "../../stores/windows";

function cn(...classes) {
  return twMerge(clsx(classes));
}

function WorkflowOverlay() {
  if (typeof window === "undefined") return null;
  const THREAD_PAGE_SIZE = 80;
  const THREAD_ROW_ESTIMATE = 92;
  const THREAD_OVERSCAN = 6;
  const state = workflowUi;
  const isVisible = createMemo(() => state().visible);
  const unifiedStatus = createMemo(() => {
    const latest = state().statusEvents[state().statusEvents.length - 1];
    if (latest?.detail) return latest.detail;
    if (state().streaming) return "Streaming response...";
    if (state().messages.length > 0) return "Response ready.";
    return "Send a message in IntentBar to start this session.";
  });
  const leftInset = createMemo(() => isVisible() && state().leftOpen ? 340 : 44);
  const rightInset = createMemo(() => isVisible() && state().rightOpen ? 360 : 44);
  const topInset = createMemo(() => 40);
  const shortSessionId = createMemo(() => state().sessionId ? `${state().sessionId.slice(0, 8)}...` : "new");
  const [conversationTab, setConversationTab] = createSignal("threads");
  const [selectedConversationId, setSelectedConversationId] = createSignal("");
  const [showConversationList, setShowConversationList] = createSignal(true);
  const [emailThreads, setEmailThreads] = createSignal([]);
  const [contactOnlyThreads, setContactOnlyThreads] = createSignal([]);
  const [contacts, setContacts] = createSignal([]);
  const [contactsLoading, setContactsLoading] = createSignal(false);
  const [loadedThreadCount, setLoadedThreadCount] = createSignal(THREAD_PAGE_SIZE);
  const [threadScrollTop, setThreadScrollTop] = createSignal(0);
  const [threadViewportHeight, setThreadViewportHeight] = createSignal(560);
  const [followThreadBottom, setFollowThreadBottom] = createSignal(true);
  let threadScrollRef;
  let threadResizeObserver;
  const aiConversation = createMemo(() => ({
    id: "ai-active",
    kind: "ai",
    channel: "ai",
    title: state().prompt || "Active AI session",
    subtitle: state().provider || "codex",
    sessionId: state().sessionId || "",
    updatedAt: (state().messages[state().messages.length - 1]?.createdAt || new Date().toISOString()),
    preview: (state().messages[state().messages.length - 1]?.text || "").trim() || (state().streaming ? "Streaming..." : "No response yet"),
    messages: (state().messages || []).map((message) => ({
      id: message.id,
      role: message.role === "user" ? "user" : "assistant",
      text: message.text || "",
      createdAt: message.createdAt,
      channel: "ai",
      author: message.role === "user" ? "You" : "Assistant"
    }))
  }));
  const sessionConversations = createMemo(() => (state().sessionHistory || [])
    .filter((session) => session?.sessionId && session.sessionId !== state().sessionId)
    .map((session) => ({
      id: `session-${session.sessionId}`,
      kind: "session",
      channel: "ai",
      title: session.preview || "Codex session",
      subtitle: session.provider || "codex",
      sessionId: session.sessionId,
      updatedAt: session.updatedAt || "",
      preview: session.preview || "Open to load this thread",
      messages: []
    }))
  );
  const threadConversations = createMemo(() => [
    aiConversation(),
    ...sessionConversations(),
    ...emailThreads(),
    ...contactOnlyThreads()
  ]);
  const activeConversation = createMemo(() => {
    const selected = selectedConversationId();
    const all = threadConversations();
    if (!selected && all.length > 0) return all[0];
    return all.find((item) => item.id === selected) || all[0] || null;
  });
  const activeConversationMessages = createMemo(() => {
    const active = activeConversation();
    if (!active) return [];
    return active.messages || [];
  });
  const visibleThreadMessages = createMemo(() => {
    const all = activeConversationMessages();
    return all.slice(Math.max(0, all.length - loadedThreadCount()));
  });
  const virtualWindow = createMemo(() => {
    const count = visibleThreadMessages().length;
    const viewport = Math.max(threadViewportHeight(), 1);
    const start = Math.max(0, Math.floor(threadScrollTop() / THREAD_ROW_ESTIMATE) - THREAD_OVERSCAN);
    const end = Math.min(count, Math.ceil((threadScrollTop() + viewport) / THREAD_ROW_ESTIMATE) + THREAD_OVERSCAN);
    return { count, start, end };
  });
  const virtualThreadRows = createMemo(() => {
    const { start, end } = virtualWindow();
    return visibleThreadMessages().slice(start, end).map((message, index) => ({
      message,
      key: `${message?.id || "msg"}-${start + index}`
    }));
  });
  const virtualTopPad = createMemo(() => virtualWindow().start * THREAD_ROW_ESTIMATE);
  const virtualBottomPad = createMemo(() => Math.max(0, (virtualWindow().count - virtualWindow().end) * THREAD_ROW_ESTIMATE));
  const drawerPanelShellClass = "flex h-full min-h-0 flex-col";
  const drawerIconButtonClass = "inline-flex h-9 w-9 items-center justify-center rounded-md text-neutral-300 transition-colors hover:bg-neutral-800/35 hover:text-[hsl(var(--primary))]";
  const drawerSmallButtonClass = "inline-flex h-7 items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]";
  const drawerListRowClass = "w-full rounded-md border border-neutral-800 bg-neutral-900/70 px-2.5 py-2 text-left transition-colors hover:bg-neutral-800/80";
  const drawerStateBlockClass = "rounded-md border border-neutral-800 bg-neutral-900/55 px-2.5 py-2 text-xs text-neutral-500";
  const channelBadgeClass = (channel) => {
    if (channel === "ai") return "border-[hsl(var(--primary)/0.42)] bg-[hsl(var(--primary)/0.14)] text-[hsl(var(--primary))]";
    if (channel === "email") return "border-neutral-700 bg-neutral-800/80 text-neutral-300";
    if (channel === "contact") return "border-neutral-700 bg-neutral-800/70 text-neutral-400";
    return "border-neutral-700 bg-neutral-800/70 text-neutral-300";
  };
  const parseEmailAddress = (value) => {
    const raw = String(value || "").trim();
    const match = raw.match(/<([^>]+)>/);
    if (match) return match[1].trim().toLowerCase();
    const emailMatch = raw.match(/[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}/i);
    return emailMatch ? emailMatch[0].toLowerCase() : raw.toLowerCase();
  };
  const parseEmailName = (value) => {
    const raw = String(value || "").trim();
    const stripped = raw.replace(/<[^>]+>/g, "").replace(/"/g, "").trim();
    if (stripped) return stripped;
    return parseEmailAddress(raw);
  };
  const ensureThreadBottom = () => {
    if (!threadScrollRef) return;
    threadScrollRef.scrollTop = threadScrollRef.scrollHeight;
  };
  const [selectedDeviceId, setSelectedDeviceId] = createSignal("");
  const [devicesRefreshing, setDevicesRefreshing] = createSignal(false);
  const devices = createMemo(() => knownDevices());
  const fleetDevices = createMemo(() => {
    const grouped = new Map();
    for (const device of devices()) {
      const isHost = device.type === "host";
      const isLocalBrowser = device.id === CURRENT_DEVICE_ID;
      const key = isHost
        ? `host:${device.metadata?.host || device.ip || device.id}`
        : isLocalBrowser
          ? "local-browser"
          : `device:${device.id}`;
      if (!grouped.has(key)) {
        grouped.set(key, {
          id: key,
          name: isHost ? "Connected Host" : isLocalBrowser ? "This Device" : (device.name || device.id),
          type: isHost ? "host" : isLocalBrowser ? "local" : (device.type || "device"),
          online: Boolean(device.online),
          primary: device,
          members: [device],
          localAvailable: isLocalBrowser && Boolean(device.online)
        });
        continue;
      }
      const entry = grouped.get(key);
      entry.members.push(device);
      entry.online = entry.online || Boolean(device.online);
      entry.localAvailable = entry.localAvailable || (device.id === CURRENT_DEVICE_ID && Boolean(device.online));
      const prevSeen = new Date(entry.primary?.lastSeenAt || 0).getTime();
      const nextSeen = new Date(device.lastSeenAt || 0).getTime();
      const preferHost = entry.primary?.type !== "host" && device.type === "host";
      if (preferHost || nextSeen > prevSeen) entry.primary = device;
    }
    return Array.from(grouped.values()).sort((a, b) => {
      if (a.online !== b.online) return a.online ? -1 : 1;
      const aSeen = new Date(a.primary?.lastSeenAt || 0).getTime();
      const bSeen = new Date(b.primary?.lastSeenAt || 0).getTime();
      return bSeen - aSeen;
    });
  });
  const selectedDevice = createMemo(() => fleetDevices().find((item) => item.id === selectedDeviceId()) || null);
  const refreshHostDevice = async () => {
    setDevicesRefreshing(true);
    try {
      await fetch("/api/host/status", { cache: "no-store" });
      await runDiscoveryScan({ force: true });
    } catch {
      // ignore UI-side refresh failures
    } finally {
      setDevicesRefreshing(false);
    }
  };

  createEffect(() => {
    const list = fleetDevices();
    if (list.length === 0) return;
    if (!selectedDeviceId() || !list.some((item) => item.id === selectedDeviceId())) {
      setSelectedDeviceId(list[0].id);
    }
  });
  createEffect(() => {
    const total = activeConversationMessages().length;
    if (total <= THREAD_PAGE_SIZE) {
      setLoadedThreadCount(THREAD_PAGE_SIZE);
      return;
    }
    setLoadedThreadCount((prev) => Math.min(Math.max(prev, THREAD_PAGE_SIZE), total));
  });
  createEffect(() => {
    const messages = activeConversationMessages();
    const last = messages[messages.length - 1];
    const signature = `${messages.length}:${last?.id || ""}:${String(last?.text || "").length}:${state().streaming ? "1" : "0"}`;
    if (!signature) return;
    if (!state().rightOpen || state().rightPanel !== "conversations" || showConversationList()) return;
    if (!followThreadBottom()) return;
    queueMicrotask(() => ensureThreadBottom());
  });
  createEffect(() => {
    if (!state().rightOpen || state().rightPanel !== "conversations" || showConversationList()) return;
    queueMicrotask(() => {
      if (!threadScrollRef) return;
      setThreadViewportHeight(Math.max(1, threadScrollRef.clientHeight));
      if (followThreadBottom()) ensureThreadBottom();
    });
  });
  createEffect(() => {
    const current = activeConversation();
    if (!current) return;
    if (selectedConversationId()) return;
    setSelectedConversationId(current.id);
  });
  createEffect(() => {
    if (typeof window === "undefined") return;
    const loadConversationSources = async () => {
      const { getAllEmails } = await import("../../lib/db");
      const emails = await getAllEmails();
      const groups = new Map();
      for (const email of emails) {
        const senderEmail = parseEmailAddress(email.from || "");
        if (!senderEmail) continue;
        const senderName = parseEmailName(email.from || senderEmail);
        const groupId = `email-${senderEmail}`;
        if (!groups.has(groupId)) {
          groups.set(groupId, {
            id: groupId,
            kind: "email",
            channel: "email",
            title: senderName,
            subtitle: senderEmail,
            updatedAt: email.date || "",
            preview: email.snippet || "",
            messages: []
          });
        }
        const group = groups.get(groupId);
        const createdAt = email.date || new Date().toISOString();
        group.messages.push({
          id: email.id || `${groupId}-${group.messages.length}`,
          role: "contact",
          text: email.snippet || email.subject || "(No Subject)",
          createdAt,
          channel: "email",
          author: senderName
        });
        if (!group.updatedAt || new Date(createdAt).getTime() > new Date(group.updatedAt).getTime()) {
          group.updatedAt = createdAt;
          group.preview = email.snippet || email.subject || "(No Subject)";
        }
      }
      const sortedThreads = Array.from(groups.values())
        .map((group) => ({
          ...group,
          messages: group.messages.sort((a, b) => new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime())
        }))
        .sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime())
        .slice(0, 80);
      setEmailThreads(sortedThreads);

      const googleToken = window.localStorage.getItem("google_token");
      if (!googleToken) {
        setContacts([]);
        return;
      }
      setContactsLoading(true);
      try {
        const response = await fetch(`/api/google/contacts?limit=100&token=${encodeURIComponent(googleToken)}`);
        const payload = await response.json().catch(() => ({}));
        const items = Array.isArray(payload?.items) ? payload.items : [];
        setContacts(items.map((item, index) => {
          const rawEmailValue = Array.isArray(item.emails) ? item.emails[0] : item.email;
          const emailValue = typeof rawEmailValue === "object" && rawEmailValue
            ? (rawEmailValue.value || rawEmailValue.email || rawEmailValue.address || "")
            : rawEmailValue;
          const email = parseEmailAddress(emailValue || "");
          const id = email ? `contact-${email}` : `contact-${item.id || index}`;
          return {
            id,
            name: item.name || email || "Unnamed",
            email,
            source: "Google Contacts"
          };
        }));
      } catch {
        setContacts([]);
      } finally {
        setContactsLoading(false);
      }
    };
    loadConversationSources();
  });
  onCleanup(() => {
    if (threadResizeObserver) {
      threadResizeObserver.disconnect();
      threadResizeObserver = null;
    }
  });

  const workflowDiffResponse = createMemo(() => ({
    success: true,
    data: `diff --git a/src/lib/latency.ts b/src/lib/latency.ts
--- a/src/lib/latency.ts
+++ b/src/lib/latency.ts
@@ -1,4 +1,8 @@
 export function sumLatency(samples) {
-  if (!samples.length) return 0
-  return samples.reduce((total, current) => total + current, 0) / samples.length
+  if (!Array.isArray(samples) || samples.length === 0) return 0
+  const valid = samples.filter((value) => Number.isFinite(value))
+  if (!valid.length) return 0
+  const total = valid.reduce((acc, current) => acc + current, 0)
+  return Math.round((total / valid.length) * 100) / 100
 }`,
    ui: {
      viewType: "code-diff",
      title: "Proposed Patch",
      description: "AI-suggested patch for latency aggregation",
      metadata: {
        source: "Workflow Demo",
        timestamp: new Date().toISOString()
      }
    }
  }));

  return <>
      <Show when={state().isOpen}>
        <Motion.div
    initial={{ opacity: 0, scale: 0.98, y: 12 }}
    animate={{ opacity: isVisible() ? 1 : 0, scale: isVisible() ? 1 : 0.98, y: isVisible() ? 0 : 12 }}
    exit={{ opacity: 0, scale: 0.98, y: 12 }}
    class={cn("workflow-overlay-root fixed inset-0 z-[10020] flex items-center justify-center bg-black/45 backdrop-blur-sm", !isVisible() && "pointer-events-none")}
    style={{
      "padding-left": `${leftInset() + 16}px`,
      "padding-right": `${rightInset() + 16}px`,
      "padding-top": `${topInset() + 16}px`,
      "padding-bottom": "16px"
    }}
  >
          <div class="relative flex h-[min(88vh,900px)] w-[min(94vw,1300px)] flex-col overflow-hidden rounded-2xl border border-neutral-700 bg-[#141414] shadow-2xl">
            <div class="flex items-center justify-between border-b border-neutral-800 px-4 py-3">
              <div>
                <h3 class="text-sm font-semibold text-white">Code Edit Workflow</h3>
                <p class="text-xs text-neutral-400">Streaming assistant + diff review + commit</p>
              </div>
              <div class="flex items-center gap-2">
                <button
    type="button"
    onClick={closeWorkflowDemo}
    class="rounded-md border border-neutral-700 bg-neutral-800 px-2 py-1 text-xs text-neutral-300 transition-colors hover:bg-neutral-700"
  >
                  Close
                </button>
              </div>
            </div>

            <div class="border-b border-neutral-800 p-3">
              <h4 class="mb-2 text-xs font-medium uppercase tracking-wide text-neutral-400">Conversation</h4>
              <div class="h-40 w-full space-y-2 overflow-auto rounded-lg border border-neutral-800 bg-[#101010] p-3 text-xs leading-5 text-neutral-200">
                <Show when={state().messages.length > 0} fallback={<p class="text-neutral-500">Send a message in IntentBar to start this session.</p>}>
                  <For each={newestFirstMessages()}>
                    {(message) => (
                      <div class="rounded-md border border-neutral-800 bg-neutral-900/60 p-2">
                        <p class="mb-1 text-[10px] uppercase tracking-wide text-neutral-500">{message.role}</p>
                        <pre class="whitespace-pre-wrap font-mono text-[11px] text-neutral-200">{message.text || "..."}</pre>
                      </div>
                    )}
                  </For>
                </Show>
              </div>
            </div>

            <div class="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_360px] gap-0">
              <div class="flex min-h-0 flex-col border-r border-neutral-800 p-3">
                <div class="mb-2 flex items-center justify-between">
                  <h4 class="text-xs font-medium uppercase tracking-wide text-neutral-400">Editor + Diff</h4>
                </div>
                <textarea
    value={state().code}
    onInput={(e) => setWorkflowCode(e.currentTarget.value)}
    class="h-48 w-full resize-none rounded-lg border border-neutral-800 bg-[#0f0f0f] p-3 font-mono text-xs text-neutral-200 focus:outline-none focus:border-neutral-600"
  />
                <div class="mt-3 min-h-0 flex-1 overflow-hidden rounded-lg border border-neutral-800">
                  <CodeDiffViewer response={workflowDiffResponse()} />
                </div>
              </div>

              <div class="min-h-0 p-3">
                <h4 class="mb-2 text-xs font-medium uppercase tracking-wide text-neutral-400">Workflow Actions</h4>
                <div class="space-y-3">
                  <button
    type="button"
    onClick={triggerWorkflowCommit}
    class={cn(
      "w-full rounded-md border px-2.5 py-2 text-xs font-medium transition-colors",
      state().committed ? "border-emerald-500/60 bg-emerald-600/20 text-emerald-200" : state().commitPending ? "border-amber-500/60 bg-amber-600/20 text-amber-100" : "border-blue-500/60 bg-blue-600/25 text-blue-100 hover:bg-blue-600/35"
    )}
  >
                    {state().committed ? "Committed" : state().commitPending ? "Confirm Commit" : "Commit Changes"}
                  </button>
                  <div class="rounded-lg border border-neutral-800 bg-neutral-900/60 p-3 text-xs text-neutral-300">
                    <p class="font-medium text-neutral-200">Suggested Commit</p>
                    <p class="mt-1 font-mono text-[11px] text-neutral-400">fix(latency): harden averaging for invalid samples</p>
                  </div>
                  <div class="rounded-lg border border-neutral-800 bg-neutral-900/60 p-3 text-xs text-neutral-300">
                    <p class="font-medium text-neutral-200">Next</p>
                    <p class="mt-1">1. Run tests</p>
                    <p>2. Review rollout metrics</p>
                    <p>3. Push and open PR</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </Motion.div>
      </Show>

      <>
        <Motion.div
    data-layer-zone="drawer"
    initial={{ x: -340 }}
    animate={{ x: state().leftOpen ? 0 : -340 }}
    transition={{ duration: 0.28, easing: [0.4, 0, 0.2, 1] }}
    class="workflow-drawer fixed left-0 top-1/2 z-[10030] h-[80vh] w-[340px] -translate-y-1/2 overflow-hidden rounded-r-2xl border border-l-0 border-neutral-800/90 bg-[#101014]/92 shadow-[0_22px_44px_rgba(0,0,0,0.45)] backdrop-blur-xl"
  >
            <div class="flex h-full">
              <div class="min-w-0 flex-1 p-0">
                <Show when={state().leftPanel === "settings"}>
                  <div class={drawerPanelShellClass}>
                    <SettingsPanel compact />
                  </div>
                </Show>
                <Show when={state().leftPanel === "launcher"}>
                  <div class={drawerPanelShellClass}>
                    <div class="border-b border-neutral-800 px-3 py-2">
                      <div class="flex items-center justify-between gap-2">
                        <p class="text-xs font-medium uppercase tracking-wide text-neutral-300">Launcher</p>
                        <button
                          type="button"
                          class="rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-[10px] text-neutral-200 transition-colors hover:bg-neutral-800"
                          onClick={() => openWindow("guide")}
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
                <Show when={state().leftPanel === "files"}>
                  <div class={drawerPanelShellClass}>
                    <FileManager compact />
                  </div>
                </Show>
                <Show when={state().leftPanel === "cloud"}>
                  <div class={drawerPanelShellClass}>
                    <CloudPanel compact />
                  </div>
                </Show>
                <Show when={state().leftPanel === "integrations"}>
                  <div class={drawerPanelShellClass}>
                    <IntegrationsPanel compact preselectProviderId={state().selectedIntegrationId || ""} />
                  </div>
                </Show>
                <Show when={state().leftPanel === "credentials"}>
                  <div class={drawerPanelShellClass}>
                    <CredentialsPanel compact />
                  </div>
                </Show>
              </div>
              <div class="hidden">
                <div class="flex h-full flex-col items-center justify-center gap-1">
                  <button
    type="button"
    onClick={() => toggleWorkflowDrawer({ side: "left", panel: "launcher" })}
    class={cn(drawerIconButtonClass, state().leftOpen && state().leftPanel === "launcher" && "text-[hsl(var(--primary))]")}
    title="Launcher panel"
  >
                    <TbOutlineBook2 size={16} />
                  </button>
                  <button
    type="button"
    onClick={() => toggleWorkflowDrawer({ side: "left", panel: "files" })}
    class={cn(drawerIconButtonClass, state().leftOpen && state().leftPanel === "files" && "text-[hsl(var(--primary))]")}
    title="Files panel"
  >
                    <TbOutlineFileText size={16} />
                  </button>
                  <button
    type="button"
    onClick={() => toggleWorkflowDrawer({ side: "left", panel: "cloud" })}
    class={cn(drawerIconButtonClass, state().leftOpen && state().leftPanel === "cloud" && "text-[hsl(var(--primary))]")}
    title="Cloud panel"
  >
                    <TbOutlineCloud size={16} />
                  </button>
                  <button
    type="button"
    onClick={() => toggleWorkflowDrawer({ side: "left", panel: "integrations" })}
    class={cn(drawerIconButtonClass, state().leftOpen && state().leftPanel === "integrations" && "text-[hsl(var(--primary))]")}
    title="Integrations panel"
  >
                    <FiLink2 size={16} />
                  </button>
                  <button
    type="button"
    onClick={() => toggleWorkflowDrawer({ side: "left", panel: "credentials" })}
    class={cn(drawerIconButtonClass, state().leftOpen && state().leftPanel === "credentials" && "text-[hsl(var(--primary))]")}
    title="Credentials panel"
  >
                    <TbOutlineKey size={16} />
                  </button>
                  <button
    type="button"
    onClick={() => toggleWorkflowDrawer({ side: "left", panel: "settings" })}
    class={cn(drawerIconButtonClass, state().leftOpen && state().leftPanel === "settings" && "text-[hsl(var(--primary))]")}
    title="Settings panel"
  >
                    <TbOutlineSettings size={16} />
                  </button>
                </div>
              </div>
            </div>
        </Motion.div>

        <Motion.div
    data-layer-zone="drawer"
    initial={{ x: 360 }}
    animate={{ x: state().rightOpen ? 0 : 360 }}
    transition={{ duration: 0.28, easing: [0.4, 0, 0.2, 1] }}
    class="workflow-drawer fixed right-0 top-1/2 z-[10030] h-[80vh] w-[360px] -translate-y-1/2 overflow-hidden rounded-l-2xl border border-r-0 border-neutral-800/90 bg-[#101014]/92 shadow-[0_22px_44px_rgba(0,0,0,0.45)] backdrop-blur-xl"
  >
            <div class="flex h-full">
              <div class="hidden">
                <div class="flex h-full flex-col items-center justify-center gap-1">
                  <button
    type="button"
    onClick={() => toggleWorkflowDrawer({ side: "right", panel: "conversations" })}
    class={cn(drawerIconButtonClass, state().rightOpen && state().rightPanel === "conversations" && "text-[hsl(var(--primary))]")}
    title="Conversations"
  >
                    <TbOutlineCommand size={16} />
                  </button>
                  <button
    type="button"
    onClick={() => toggleWorkflowDrawer({ side: "right", panel: "devices" })}
    class={cn(drawerIconButtonClass, state().rightOpen && state().rightPanel === "devices" && "text-[hsl(var(--primary))]")}
    title="Devices panel"
  >
                    <TbOutlineDeviceDesktop size={16} />
                  </button>
                </div>
              </div>
              <div class="min-w-0 flex-1 p-0">
                <Show when={state().rightPanel === "conversations"}>
                  <div class={drawerPanelShellClass}>
                    <Show when={showConversationList()}>
                      <div class="border-b border-neutral-800 px-3 py-2">
                        <div class="flex items-center justify-between gap-2">
                          <p class="text-xs font-medium uppercase tracking-wide text-neutral-300">Conversations</p>
                          <button
                            type="button"
                            onClick={startNewCodexSession}
                            class={drawerSmallButtonClass}
                          >
                            <TbOutlinePlus size={11} />
                            New
                          </button>
                        </div>
                        <div class="mt-2 grid grid-cols-2 gap-1 rounded-md border border-neutral-800 bg-neutral-900/60 p-1">
                          <button
                            type="button"
                            onClick={() => setConversationTab("threads")}
                            class={cn(
                              "rounded px-2 py-1 text-[11px] transition-colors",
                              conversationTab() === "threads" ? "font-semibold text-[hsl(var(--primary))]" : "text-neutral-400 hover:bg-neutral-800"
                            )}
                          >
                            Threads
                          </button>
                          <button
                            type="button"
                            onClick={() => setConversationTab("contacts")}
                            class={cn(
                              "rounded px-2 py-1 text-[11px] transition-colors",
                              conversationTab() === "contacts" ? "font-semibold text-[hsl(var(--primary))]" : "text-neutral-400 hover:bg-neutral-800"
                            )}
                          >
                            Contacts
                          </button>
                        </div>
                      </div>
                      <div class="min-h-0 flex-1 overflow-auto p-3">
                        <Show when={conversationTab() === "threads"}>
                          <div class="space-y-1.5">
                            <For each={threadConversations()}>
                              {(thread) => (
                                <button
                                  type="button"
                                  onClick={() => {
                                    if (thread.kind === "session") {
                                      const session = state().sessionHistory.find((item) => item.sessionId === thread.sessionId);
                                      if (session) useWorkflowSession(session);
                                      setSelectedConversationId("ai-active");
                                      setShowConversationList(false);
                                      return;
                                    }
                                    setSelectedConversationId(thread.id);
                                    setShowConversationList(false);
                                  }}
                                  class={cn(
                                    cn(drawerListRowClass, "text-left"),
                                    activeConversation()?.id === thread.id
                                      ? "border-neutral-700 bg-neutral-900/85"
                                      : ""
                                  )}
                                >
                                  <div class="flex items-center justify-between gap-2">
                                    <p class={cn("truncate text-[11px] text-neutral-200", activeConversation()?.id === thread.id ? "font-semibold text-[hsl(var(--primary))]" : "font-medium")}>{thread.title}</p>
                                    <span class={cn("rounded border px-1.5 py-0.5 text-[9px] uppercase tracking-wide", channelBadgeClass(thread.channel))}>
                                      {thread.channel}
                                    </span>
                                  </div>
                                  <p class="mt-1 truncate text-[10px] text-neutral-500">{thread.preview || thread.subtitle || "No messages yet"}</p>
                                </button>
                              )}
                            </For>
                          </div>
                        </Show>
                        <Show when={conversationTab() === "contacts"}>
                          <div class="space-y-1.5">
                            <Show when={!contactsLoading()} fallback={<p class={drawerStateBlockClass}>Loading contacts...</p>}>
                              <For each={contacts()}>
                                {(contact) => (
                                  <button
                                    type="button"
                                    onClick={() => {
                                      const emailThreadId = contact.email ? `email-${contact.email}` : "";
                                      const existing = emailThreadId ? threadConversations().find((thread) => thread.id === emailThreadId) : null;
                                      if (existing) {
                                        setConversationTab("threads");
                                        setSelectedConversationId(existing.id);
                                        setShowConversationList(false);
                                        return;
                                      }
                                      const fallbackId = contact.email ? `contact-${contact.email}` : contact.id;
                                      setContactOnlyThreads((prev) => {
                                        if (prev.some((item) => item.id === fallbackId)) return prev;
                                        return [
                                          ...prev,
                                          {
                                            id: fallbackId,
                                            kind: "contact",
                                            channel: "contact",
                                            title: contact.name,
                                            subtitle: contact.email || "No email",
                                            updatedAt: new Date().toISOString(),
                                            preview: "No messages yet",
                                            messages: []
                                          }
                                        ];
                                      });
                                      setConversationTab("threads");
                                      setSelectedConversationId(fallbackId);
                                      setShowConversationList(false);
                                    }}
                                    class={drawerListRowClass}
                                  >
                                    <div class="flex items-center gap-2">
                                      <TbOutlineUser size={12} class="text-[hsl(var(--primary))]" />
                                      <p class="truncate text-[11px] font-medium text-neutral-200">{contact.name}</p>
                                    </div>
                                    <p class="mt-1 truncate text-[10px] text-neutral-500">{contact.email || "No email"}</p>
                                  </button>
                                )}
                              </For>
                              <Show when={contacts().length === 0}>
                                <p class={drawerStateBlockClass}>No contacts loaded.</p>
                              </Show>
                            </Show>
                          </div>
                        </Show>
                      </div>
                    </Show>
                    <Show when={!showConversationList()}>
                      <div class="flex items-center justify-between border-b border-neutral-800 px-3 py-2">
                        <button
                          type="button"
                          onClick={() => setShowConversationList(true)}
                          class={drawerSmallButtonClass}
                        >
                          Back
                        </button>
                        <p class="truncate text-[11px] font-medium text-neutral-200">{activeConversation()?.title || "Conversation"}</p>
                        <span class={cn("rounded border px-1.5 py-0.5 text-[9px] uppercase tracking-wide", channelBadgeClass(activeConversation()?.channel || "ai"))}>
                          {activeConversation()?.channel || "ai"}
                        </span>
                      </div>
                      <div
                        ref={(el) => {
                          threadScrollRef = el;
                          setThreadViewportHeight(Math.max(1, el.clientHeight));
                          if (typeof ResizeObserver !== "undefined") {
                            if (threadResizeObserver) threadResizeObserver.disconnect();
                            threadResizeObserver = new ResizeObserver(() => {
                              setThreadViewportHeight(Math.max(1, el.clientHeight));
                            });
                            threadResizeObserver.observe(el);
                          }
                        }}
                        class="min-h-0 flex-1 overflow-auto p-3"
                        onScroll={(event) => {
                          const target = event.currentTarget;
                          const scrollTop = target.scrollTop;
                          const scrollBottomGap = target.scrollHeight - target.clientHeight - scrollTop;
                          setThreadScrollTop(scrollTop);
                          setThreadViewportHeight(Math.max(1, target.clientHeight));
                          setFollowThreadBottom(scrollBottomGap < 80);
                          if (scrollTop < 160) {
                            const total = activeConversationMessages().length;
                            setLoadedThreadCount((prev) => Math.min(total, prev + THREAD_PAGE_SIZE));
                          }
                        }}
                      >
                        <Show
                          when={activeConversationMessages().length > 0}
                          fallback={<p class={drawerStateBlockClass}>{state().streaming ? "Streaming response..." : "No messages in this thread."}</p>}
                        >
                          <>
                            <Show when={loadedThreadCount() < activeConversationMessages().length}>
                              <p class="mb-2 px-1 text-[10px] uppercase tracking-wide text-neutral-500">
                                Scroll up to load older messages ({visibleThreadMessages().length}/{activeConversationMessages().length})
                              </p>
                            </Show>
                            <div style={{ height: `${virtualTopPad()}px` }} />
                            <For each={virtualThreadRows()}>
                              {(row) => (
                                <article
                                  class={cn(
                                    "mb-2 rounded-md border p-2",
                                    row.message?.role === "user"
                                      ? "ml-6 border-[hsl(var(--primary)/0.38)] bg-[hsl(var(--primary)/0.12)]"
                                      : "mr-6 border-neutral-700 bg-neutral-900/70"
                                  )}
                                >
                                  <div class="mb-1 flex items-center justify-between gap-2">
                                    <div class="flex items-center gap-1.5">
                                      <p class={cn("text-[10px] uppercase tracking-wide", row.message?.role === "user" ? "text-[hsl(var(--primary))]" : "text-neutral-300")}>
                                        {row.message?.author || (row.message?.role === "user" ? "You" : "Assistant")}
                                      </p>
                                      <span class={cn("rounded border px-1.5 py-0.5 text-[9px] uppercase tracking-wide", channelBadgeClass(row.message?.channel || "ai"))}>
                                        {row.message?.channel || "ai"}
                                      </span>
                                    </div>
                                    <p class="text-[10px] text-neutral-500">
                                      {row.message?.createdAt
                                        ? new Date(row.message.createdAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" })
                                        : ""}
                                    </p>
                                  </div>
                                  <p class="font-mono text-[11px] leading-5 text-neutral-200 whitespace-pre-wrap break-words [overflow-wrap:anywhere]">
                                    {row.message?.text || (state().streaming && row.message?.role !== "user" ? "..." : "")}
                                  </p>
                                </article>
                              )}
                            </For>
                            <div style={{ height: `${virtualBottomPad()}px` }} />
                          </>
                        </Show>
                      </div>
                    </Show>
                  </div>
                </Show>
                <Show when={state().rightPanel === "devices"}>
                  <div class={drawerPanelShellClass}>
                    <div class="border-b border-neutral-800 px-3 py-2">
                      <div class="flex items-center justify-between gap-2">
                        <p class="text-xs font-medium uppercase tracking-wide text-neutral-300">Devices</p>
                        <div class="flex items-center gap-1">
                          <button
                            type="button"
                            onClick={() => setScannerEnabled("wifi", !scanSettings().wifi)}
                            class={cn(
                              "inline-flex h-7 items-center rounded-md border px-2 text-[10px] transition-colors",
                              scanSettings().wifi ? "border-[hsl(var(--primary)/0.5)] bg-[hsl(var(--primary)/0.15)] text-[hsl(var(--primary))]" : "border-neutral-700 bg-neutral-900 text-neutral-400 hover:bg-neutral-800"
                            )}
                            title="Wi-Fi scanning"
                          >
                            <TbOutlineWifi size={12} />
                          </button>
                          <button
                            type="button"
                            onClick={() => setScannerEnabled("bluetooth", !scanSettings().bluetooth)}
                            class={cn(
                              "inline-flex h-7 items-center rounded-md border px-2 text-[10px] transition-colors",
                              scanSettings().bluetooth ? "border-[hsl(var(--primary)/0.5)] bg-[hsl(var(--primary)/0.15)] text-[hsl(var(--primary))]" : "border-neutral-700 bg-neutral-900 text-neutral-400 hover:bg-neutral-800"
                            )}
                            title="Bluetooth scanning"
                          >
                            <TbOutlineBluetooth size={12} />
                          </button>
                          <button
                            type="button"
                            onClick={() => setScannerEnabled("lan", !scanSettings().lan)}
                            class={cn(
                              "inline-flex h-7 items-center rounded-md border px-2 text-[10px] transition-colors",
                              scanSettings().lan ? "border-[hsl(var(--primary)/0.5)] bg-[hsl(var(--primary)/0.15)] text-[hsl(var(--primary))]" : "border-neutral-700 bg-neutral-900 text-neutral-400 hover:bg-neutral-800"
                            )}
                            title="LAN scanning"
                          >
                            <TbOutlineNetwork size={12} />
                          </button>
                          <button
                            type="button"
                            onClick={() => runDiscoveryScan({ force: true })}
                            class={drawerSmallButtonClass}
                            title="Run discovery scan now"
                          >
                            <TbOutlineScan size={12} class={scanState().running ? "animate-pulse" : ""} />
                          </button>
                        </div>
                      </div>
                      <p class="mt-1 text-[10px] text-neutral-500">
                        {scanState().running
                          ? "Scanning..."
                          : scanState().lastScanAt
                            ? `Last scan ${new Date(scanState().lastScanAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" })}`
                            : "Scanning is off"}
                      </p>
                      <Show when={scanState().error}>
                        <p class="mt-1 text-[10px] text-rose-300">{scanState().error}</p>
                      </Show>
                    </div>
                    <div class="min-h-0 flex-1 overflow-auto p-3">
                      <Show when={fleetDevices().length > 0} fallback={<p class={drawerStateBlockClass}>No connected devices yet.</p>}>
                        <div class="space-y-1.5">
                          <For each={fleetDevices()}>
                            {(device) => (
                              <button
                                type="button"
                                onClick={() => setSelectedDeviceId(device.id)}
                                class={cn(
                                  cn(drawerListRowClass, "text-left"),
                                  selectedDeviceId() === device.id
                                    ? "border-neutral-700 bg-neutral-900/85"
                                    : ""
                                )}
                              >
                                <div class="flex items-center justify-between gap-2">
                                  <div class="flex items-center gap-2 min-w-0">
                                    <Show
                                      when={device.type === "host"}
                                      fallback={
                                        <Show
                                          when={device.type === "wifi"}
                                          fallback={
                                            <Show
                                              when={device.type === "bluetooth"}
                                              fallback={
                                                <Show when={device.type === "lan"} fallback={<TbOutlineDeviceDesktop size={13} class="text-neutral-300" />}>
                                                  <TbOutlineNetwork size={13} class="text-[hsl(var(--primary))]" />
                                                </Show>
                                              }
                                            >
                                              <TbOutlineBluetooth size={13} class="text-[hsl(var(--primary))]" />
                                            </Show>
                                          }
                                        >
                                          <TbOutlineWifi size={13} class="text-[hsl(var(--primary))]" />
                                        </Show>
                                      }
                                    >
                                      <TbOutlineServer size={13} class="text-[hsl(var(--primary))]" />
                                    </Show>
                                    <p class={cn("truncate text-[11px] text-neutral-200", selectedDeviceId() === device.id ? "font-semibold text-[hsl(var(--primary))]" : "font-medium")}>{device.name || device.id}</p>
                                  </div>
                                  <div class="flex items-center gap-1">
                                    <Show when={device.localAvailable} fallback={<TbOutlineWifiOff size={12} class={device.online ? "text-[hsl(var(--primary))]" : "text-neutral-500"} />}>
                                      <TbOutlineWifi size={12} class="text-[hsl(var(--primary))]" />
                                    </Show>
                                    <span class={cn("inline-block h-2.5 w-2.5 rounded-full", device.online ? "bg-[hsl(var(--primary))]" : "bg-neutral-600")} />
                                  </div>
                                </div>
                                <p class="mt-1 truncate text-[10px] text-neutral-500">
                                  {device.primary?.ip || device.primary?.metadata?.host || "unknown"} · {device.members.length} source{device.members.length === 1 ? "" : "s"}
                                </p>
                              </button>
                            )}
                          </For>
                        </div>

                        <Show when={selectedDevice()}>
                          {(deviceAccessor) => {
                            const device = deviceAccessor();
                            const detail = device.primary || {};
                            return (
                              <div class="mt-3 rounded-md border border-neutral-800 bg-neutral-900/60 p-2 text-[11px] text-neutral-300">
                                <p class="mb-2 text-[10px] uppercase tracking-wide text-neutral-500">Details</p>
                                <div class="mb-2 flex flex-wrap items-center gap-2">
                                  <button
                                    type="button"
                                    class={drawerSmallButtonClass}
                                    onClick={refreshHostDevice}
                                  >
                                    <TbOutlineRefresh size={11} class={devicesRefreshing() ? "animate-spin" : ""} />
                                    {devicesRefreshing() ? "Refreshing" : "Refresh"}
                                  </button>
                                  <button
                                    type="button"
                                    class={drawerSmallButtonClass}
                                    onClick={() => openWindow("terminal")}
                                  >
                                    Open Terminal
                                  </button>
                                  <button
                                    type="button"
                                    class={drawerSmallButtonClass}
                                    onClick={() => openWindow("files")}
                                  >
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
                                  <p>
                                    <span class="text-neutral-500">Capabilities:</span>{" "}
                                    {detail.metadata.capabilities.shell ? "shell" : "no-shell"},{" "}
                                    {detail.metadata.capabilities.fileSystem ? "fs" : "no-fs"}
                                  </p>
                                </Show>
                              </div>
                            );
                          }}
                        </Show>
                      </Show>
                    </div>
                  </div>
                </Show>
              </div>
            </div>
        </Motion.div>

        <Motion.div
          initial={{ x: 0 }}
          animate={{ x: state().leftOpen ? 340 : 0 }}
          transition={{ duration: 0.28, easing: [0.4, 0, 0.2, 1] }}
          class="fixed left-0 top-1/2 z-[10034] -translate-y-1/2 rounded-r-xl p-1"
        >
          <div class="flex flex-col items-center gap-1">
            <button
              type="button"
              onClick={() => toggleWorkflowDrawer({ side: "left", panel: "launcher" })}
              class={cn(drawerIconButtonClass, state().leftOpen && state().leftPanel === "launcher" && "text-[hsl(var(--primary))]")}
              title="Launcher panel"
            >
              <TbOutlineBook2 size={16} />
            </button>
            <button
              type="button"
              onClick={() => toggleWorkflowDrawer({ side: "left", panel: "files" })}
              class={cn(drawerIconButtonClass, state().leftOpen && state().leftPanel === "files" && "text-[hsl(var(--primary))]")}
              title="Files panel"
            >
              <TbOutlineFileText size={16} />
            </button>
            <button
              type="button"
              onClick={() => toggleWorkflowDrawer({ side: "left", panel: "cloud" })}
              class={cn(drawerIconButtonClass, state().leftOpen && state().leftPanel === "cloud" && "text-[hsl(var(--primary))]")}
              title="Cloud panel"
            >
              <TbOutlineCloud size={16} />
            </button>
            <button
              type="button"
              onClick={() => toggleWorkflowDrawer({ side: "left", panel: "integrations" })}
              class={cn(drawerIconButtonClass, state().leftOpen && state().leftPanel === "integrations" && "text-[hsl(var(--primary))]")}
              title="Integrations panel"
            >
              <FiLink2 size={16} />
            </button>
            <button
              type="button"
              onClick={() => toggleWorkflowDrawer({ side: "left", panel: "credentials" })}
              class={cn(drawerIconButtonClass, state().leftOpen && state().leftPanel === "credentials" && "text-[hsl(var(--primary))]")}
              title="Credentials panel"
            >
              <TbOutlineKey size={16} />
            </button>
            <button
              type="button"
              onClick={() => toggleWorkflowDrawer({ side: "left", panel: "settings" })}
              class={cn(drawerIconButtonClass, state().leftOpen && state().leftPanel === "settings" && "text-[hsl(var(--primary))]")}
              title="Settings panel"
            >
              <TbOutlineSettings size={16} />
            </button>
          </div>
        </Motion.div>

        <Motion.div
          initial={{ x: 0 }}
          animate={{ x: state().rightOpen ? -360 : 0 }}
          transition={{ duration: 0.28, easing: [0.4, 0, 0.2, 1] }}
          class="fixed right-0 top-1/2 z-[10034] -translate-y-1/2 rounded-l-xl p-1"
        >
          <div class="flex flex-col items-center gap-1">
            <button
              type="button"
              onClick={() => toggleWorkflowDrawer({ side: "right", panel: "conversations" })}
              class={cn(drawerIconButtonClass, state().rightOpen && state().rightPanel === "conversations" && "text-[hsl(var(--primary))]")}
              title="Conversations"
            >
              <TbOutlineCommand size={16} />
            </button>
            <button
              type="button"
              onClick={() => toggleWorkflowDrawer({ side: "right", panel: "devices" })}
              class={cn(drawerIconButtonClass, state().rightOpen && state().rightPanel === "devices" && "text-[hsl(var(--primary))]")}
              title="Devices panel"
            >
              <TbOutlineDeviceDesktop size={16} />
            </button>
          </div>
        </Motion.div>

      </>

      <Portal mount={document.body}>
        <Show when={state().isOpen && state().leftOpen}>
          <div
            data-workflow-portal="left"
            class="pointer-events-none fixed left-12 top-1/2 z-[10036] h-[72vh] w-[280px] -translate-y-1/2 rounded-xl border border-dashed border-neutral-700/50 bg-neutral-900/20"
          />
        </Show>
        <Show when={state().isOpen && state().rightOpen}>
          <div
            data-workflow-portal="right"
            class="pointer-events-none fixed right-12 top-1/2 z-[10036] h-[72vh] w-[300px] -translate-y-1/2 rounded-xl border border-dashed border-neutral-700/50 bg-neutral-900/20"
          />
        </Show>
      </Portal>

    </>;
}

export default WorkflowOverlay;
