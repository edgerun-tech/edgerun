import { Show, For, createMemo, createSignal, createEffect, onMount, onCleanup } from "solid-js";
import { Motion } from "solid-motionone";
import { Portal } from "solid-js/web";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import { CodeDiffViewer } from "../results/CodeDiffViewer";
import ConversationsPanel from "./ConversationsPanel";
import DevicesPanel from "./DevicesPanel";
import LeftDrawer from "./LeftDrawer";
import {
  closeWorkflowDemo,
  openWorkflowIntegrations,
  startNewAssistantSession,
  setWorkflowCode,
  toggleWorkflowDrawer,
  triggerWorkflowCommit,
  useWorkflowSession,
  workflowUi
} from "../../stores/workflow-ui";
import { integrationStore } from "../../stores/integrations";
import { openWindow } from "../../stores/windows";
import { isOfficialBridgeId } from "../../lib/integrations/official-bridges";
import {
  CHAT_HEAD_PREFS_KEY,
  CHAT_HEAD_PRESET_COLORS,
  DRAWER_ICON_BUTTON_CLASS,
  DRAWER_SUGGESTION_ICON_BUTTON_CLASS,
  LEFT_DRAWER_PANEL_ITEMS,
  PANEL_SUGGESTION_TAGS,
  RIGHT_DRAWER_PANEL_ITEMS,
  THREAD_OVERSCAN,
  THREAD_PAGE_SIZE,
  THREAD_ROW_ESTIMATE
} from "./workflow-overlay.constants";
import {
  integrationIconForSuggestion,
} from "./workflow-overlay.utils";
import {
  emitConversationChatHeadUpdated,
  emitConversationMessageSent
} from "./workflow-overlay.events";
import { useWorkflowDeviceConnect } from "./use-workflow-device-connect";
import { useWorkflowConversationSources } from "./use-workflow-conversation-sources";
import { pushClipboardEntry } from "../../stores/clipboard-history";

const SUPER_V_SHORTCUT_EVENT = "intent-ui-super-v";

function cn(...classes) {
  return twMerge(clsx(classes));
}

function WorkflowOverlay() {
  if (typeof window === "undefined") return null;
  const LOCAL_CONVERSATION_MESSAGES_KEY = "intent-ui-local-conversation-messages-v1";
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
  const newestFirstMessages = createMemo(() => [...(state().messages || [])].reverse());
  const [conversationTab, setConversationTab] = createSignal("threads");
  const [selectedConversationId, setSelectedConversationId] = createSignal("");
  const [showConversationList, setShowConversationList] = createSignal(true);
  const [contactOnlyThreads, setContactOnlyThreads] = createSignal([]);
  const [loadedThreadCount, setLoadedThreadCount] = createSignal(THREAD_PAGE_SIZE);
  const [threadScrollTop, setThreadScrollTop] = createSignal(0);
  const [threadViewportHeight, setThreadViewportHeight] = createSignal(560);
  const [followThreadBottom, setFollowThreadBottom] = createSignal(true);
  const [showConversationSettings, setShowConversationSettings] = createSignal(false);
  const [showEmojiPalette, setShowEmojiPalette] = createSignal(false);
  const [draftMessage, setDraftMessage] = createSignal("");
  const [localMessagesByConversation, setLocalMessagesByConversation] = createSignal((() => {
    try {
      const parsed = JSON.parse(localStorage.getItem(LOCAL_CONVERSATION_MESSAGES_KEY) || "{}");
      return parsed && typeof parsed === "object" ? parsed : {};
    } catch {
      return {};
    }
  })());
  const [chatHeadPrefs, setChatHeadPrefs] = createSignal((() => {
    try {
      const parsed = JSON.parse(localStorage.getItem(CHAT_HEAD_PREFS_KEY) || "{}");
      return parsed && typeof parsed === "object" ? parsed : {};
    } catch {
      return {};
    }
  })());
  const aiConversation = createMemo(() => ({
    id: "ai-active",
    kind: "ai",
    channel: "ai",
    title: state().prompt || "Active AI session",
    subtitle: state().provider || "opencode",
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
      subtitle: session.provider || "opencode",
      sessionId: session.sessionId,
      updatedAt: session.updatedAt || "",
      preview: session.preview || "Open to load this thread",
      messages: []
    }))
  );
  const messageProviderIntegrations = createMemo(() => integrationStore.list().filter((integration) => (
    integration.id === "email" || isOfficialBridgeId(integration.id)
  )));
  const {
    emailThreads,
    bridgeThreads,
    contacts,
    contactsLoading
  } = useWorkflowConversationSources({
    messageProviderIntegrations,
    localMessagesByConversation
  });
  const threadConversations = createMemo(() => [
    aiConversation(),
    ...sessionConversations(),
    ...bridgeThreads(),
    ...emailThreads(),
    ...contactOnlyThreads()
  ]);
  const hasConversationContent = createMemo(() => {
    const threads = threadConversations();
    if (threads.length === 0) return false;
    return threads.some((thread) => {
      if (thread.id === "ai-active") return Boolean((thread.messages || []).length || state().prompt?.trim());
      return Boolean((thread.messages || []).length || thread.preview?.trim());
    });
  });
  const activeConversation = createMemo(() => {
    const selected = selectedConversationId();
    const all = threadConversations();
    if (!selected && all.length > 0) return all[0];
    return all.find((item) => item.id === selected) || all[0] || null;
  });
  const activeConversationMessages = createMemo(() => {
    const active = activeConversation();
    if (!active) return [];
    const local = localMessagesByConversation()[active.id] || [];
    return [...(active.messages || []), ...local];
  });
  const suggestIntegrationsForPanel = (panelId) => {
    const wantedTags = PANEL_SUGGESTION_TAGS[panelId] || [];
    if (wantedTags.length === 0) return [];
    return integrationStore.list()
      .map((integration) => {
        const integrationTags = Array.isArray(integration.tags) ? integration.tags : [];
        const overlap = integrationTags.filter((tag) => wantedTags.includes(tag)).length;
        return { integration, overlap };
      })
      .filter((item) => item.overlap > 0)
      .sort((a, b) => {
        if (a.integration.available !== b.integration.available) return a.integration.available ? 1 : -1;
        if (a.integration.connected !== b.integration.connected) return a.integration.connected ? 1 : -1;
        if (b.overlap !== a.overlap) return b.overlap - a.overlap;
        return a.integration.name.localeCompare(b.integration.name);
      })
      .slice(0, 4)
      .map((item) => item.integration);
  };
  const leftPanelSuggestions = createMemo(() => suggestIntegrationsForPanel(state().leftPanel));
  const rightPanelSuggestions = createMemo(() => suggestIntegrationsForPanel(state().rightPanel));
  const chatHeadForConversation = (conversation) => {
    if (!conversation) return { emoji: "💬", color: CHAT_HEAD_PRESET_COLORS[0], label: "C" };
    const pref = chatHeadPrefs()[conversation.id] || {};
    const fallbackLabel = String(conversation.title || "C").trim().slice(0, 1).toUpperCase() || "C";
    const fallbackEmoji = conversation.channel === "email" ? "📧" : conversation.channel === "ai" ? "🧠" : "💬";
    return {
      emoji: String(pref.emoji || fallbackEmoji),
      color: String(pref.color || CHAT_HEAD_PRESET_COLORS[0]),
      label: String(pref.label || fallbackLabel).slice(0, 2).toUpperCase()
    };
  };
  const fallbackChatHead = { emoji: "💬", color: CHAT_HEAD_PRESET_COLORS[0], label: "C" };
  const activeChatHead = createMemo(() => chatHeadForConversation(activeConversation()) || fallbackChatHead);
  const persistChatHeadPref = (conversationId, patch) => {
    if (!conversationId) return;
    setChatHeadPrefs((prev) => {
      const next = {
        ...prev,
        [conversationId]: {
          ...(prev[conversationId] || {}),
          ...patch
        }
      };
      try {
        localStorage.setItem(CHAT_HEAD_PREFS_KEY, JSON.stringify(next));
      } catch {
        // ignore storage failures
      }
      emitConversationChatHeadUpdated(conversationId, next[conversationId]);
      return next;
    });
  };
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
  const openIntegrationSuggestion = (integrationId) => {
    openWorkflowIntegrations(integrationId);
  };
  const DrawerIntegrationSuggestions = (props) => (
    <div class="shrink-0 border-t border-neutral-800 bg-[#0d0e12]/95 px-3 py-2" data-testid={`drawer-suggestions-${props.side}-${props.panel}`}>
      <div class="relative mb-1 h-7 overflow-hidden">
        <div class="pointer-events-none absolute inset-0 flex items-center gap-1 opacity-35" aria-hidden="true">
          <For each={props.items().slice(0, 8)}>
            {(integration) => {
              const Icon = integrationIconForSuggestion(integration.id);
              return <Icon size={12} class="text-neutral-500" />;
            }}
          </For>
        </div>
        <div class="relative z-10 flex h-full items-center">
          <p class="text-[10px] font-medium uppercase tracking-wide text-neutral-300">Suggested Integrations</p>
        </div>
      </div>
      <Show
        when={props.items().length > 0}
        fallback={<p class="px-1 py-1 text-[10px] text-neutral-500">No integration suggestions for this panel yet.</p>}
      >
        <div class="flex flex-wrap gap-1" data-testid={`drawer-suggestions-list-${props.side}-${props.panel}`}>
          <For each={props.items()}>
            {(integration) => {
              const Icon = integrationIconForSuggestion(integration.id);
              return (
                <button
                  type="button"
                  class={DRAWER_SUGGESTION_ICON_BUTTON_CLASS}
                  onClick={() => openIntegrationSuggestion(integration.id)}
                  data-testid={`drawer-suggestion-${props.panel}-${integration.id}`}
                  title={`${integration.name} · ${integration.available ? "available" : integration.availabilityReason || "not ready"}`}
                  aria-label={`Open integration ${integration.name}`}
                >
                  <Icon size={14} />
                </button>
              );
            }}
          </For>
        </div>
      </Show>
    </div>
  );
  const {
    selectedDeviceId,
    setSelectedDeviceId,
    fleetDevices,
    selectedDevice,
    connectPlatform,
    setConnectPlatform,
    pairingCodeInput,
    setPairingCodeInput,
    deviceConnectCopied,
    showDeviceConnectDialog,
    setShowDeviceConnectDialog,
    profilePublicKeyInput,
    setProfilePublicKeyInput,
    requestedLabelInput,
    setRequestedLabelInput,
    connectDomain,
    setConnectDomain,
    connectRegistrationToken,
    setConnectRegistrationToken,
    reserveBusy,
    reserveError,
    reserveStatus,
    pairingBusy,
    pairingError,
    pairingStatus,
    pairingExpiresAt,
    linuxConnectScript,
    copyConnectScript,
    issuePairingCode,
    reserveDomain
  } = useWorkflowDeviceConnect({ state });
  const sendDraftMessage = async () => {
    const text = draftMessage().trim();
    const conversation = activeConversation();
    if (!text || !conversation) return;
    const entry = {
      id: `local-${conversation.id}-${Date.now()}`,
      role: "user",
      text,
      createdAt: new Date().toISOString(),
      channel: conversation.channel || "chat",
      author: "You"
    };
    setLocalMessagesByConversation((prev) => ({
      ...prev,
      [conversation.id]: [...(prev[conversation.id] || []), entry]
    }));
    emitConversationMessageSent(conversation.id, text, conversation.channel || "chat");
    setDraftMessage("");
    setShowEmojiPalette(false);
  };
  let handleSuperVShortcut;
  onMount(() => {
    handleSuperVShortcut = () => {
      if (!(state().rightOpen && state().rightPanel === "conversations")) {
        toggleWorkflowDrawer({ side: "right", panel: "conversations" });
      }
      setConversationTab("threads");
      setShowConversationList(false);
      setShowConversationSettings(false);
      setShowEmojiPalette(true);
      setSelectedConversationId("ai-active");
      queueMicrotask(() => {
        const draftInput = document.querySelector('[data-testid="conversation-draft-input"]');
        if (draftInput instanceof HTMLElement) draftInput.focus();
      });
      if (navigator.clipboard?.readText) {
        void navigator.clipboard.readText().then((text) => {
          if (!String(text || "").trim()) return;
          pushClipboardEntry(text, "super-v");
        }).catch(() => {
          // ignore clipboard permission or availability failures
        });
      }
    };
    window.addEventListener(SUPER_V_SHORTCUT_EVENT, handleSuperVShortcut);
  });
  onCleanup(() => {
    if (handleSuperVShortcut) {
      window.removeEventListener(SUPER_V_SHORTCUT_EVENT, handleSuperVShortcut);
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
    const current = activeConversation();
    if (!current) return;
    if (selectedConversationId()) return;
    setSelectedConversationId(current.id);
  });
  createEffect(() => {
    if (showConversationList()) {
      setShowConversationSettings(false);
      setShowEmojiPalette(false);
    }
  });
  createEffect(() => {
    try {
      localStorage.setItem(LOCAL_CONVERSATION_MESSAGES_KEY, JSON.stringify(localMessagesByConversation()));
    } catch {
      // ignore storage failures
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
                <div class="flex h-full min-h-0 flex-col">
                  <LeftDrawer state={state} onOpenGuide={() => openWindow("guide")} />
                  <DrawerIntegrationSuggestions
                    side="left"
                    panel={state().leftPanel}
                    items={leftPanelSuggestions}
                  />
                </div>
              </div>
              <div class="hidden">
                <div class="flex h-full flex-col items-center justify-center gap-1">
                  <For each={LEFT_DRAWER_PANEL_ITEMS}>
                    {(item) => (
                      <button
                        type="button"
                        onClick={() => toggleWorkflowDrawer({ side: "left", panel: item.id })}
                        class={cn(DRAWER_ICON_BUTTON_CLASS, state().leftOpen && state().leftPanel === item.id && "text-[hsl(var(--primary))]")}
                        title={item.title}
                      >
                        <item.Icon size={16} />
                      </button>
                    )}
                  </For>
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
                  <For each={RIGHT_DRAWER_PANEL_ITEMS}>
                    {(item) => (
                      <button
                        type="button"
                        onClick={() => toggleWorkflowDrawer({ side: "right", panel: item.id })}
                        class={cn(DRAWER_ICON_BUTTON_CLASS, state().rightOpen && state().rightPanel === item.id && "text-[hsl(var(--primary))]")}
                        title={item.title}
                      >
                        <item.Icon size={16} />
                      </button>
                    )}
                  </For>
                </div>
              </div>
              <div class="min-w-0 flex-1 p-0">
                <div class="flex h-full min-h-0 flex-col">
                  <div class="min-h-0 flex-1 overflow-hidden">
                    <Show when={state().rightPanel === "conversations"}>
                      <ConversationsPanel
                        state={state}
                        cn={cn}
                        conversationTab={conversationTab}
                        setConversationTab={setConversationTab}
                        showConversationList={showConversationList}
                        setShowConversationList={setShowConversationList}
                        threadConversations={threadConversations}
                        activeConversation={activeConversation}
                        hasConversationContent={hasConversationContent}
                        messageProviderIntegrations={messageProviderIntegrations}
                        contacts={contacts}
                        contactsLoading={contactsLoading}
                        setContactOnlyThreads={setContactOnlyThreads}
                        setSelectedConversationId={setSelectedConversationId}
                        onSelectSessionThread={(sessionId) => {
                          const session = state().sessionHistory.find((item) => item.sessionId === sessionId);
                          if (session) useWorkflowSession(session);
                          setSelectedConversationId("ai-active");
                          setShowConversationList(false);
                        }}
                        onNewSession={startNewAssistantSession}
                        onOpenIntegrations={() => toggleWorkflowDrawer({ side: "left", panel: "integrations" })}
                        chatHeadForConversation={chatHeadForConversation}
                        fallbackChatHead={fallbackChatHead}
                        activeChatHead={activeChatHead}
                        showConversationSettings={showConversationSettings}
                        setShowConversationSettings={setShowConversationSettings}
                        persistChatHeadPref={persistChatHeadPref}
                        activeConversationMessages={activeConversationMessages}
                        loadedThreadCount={loadedThreadCount}
                        setLoadedThreadCount={setLoadedThreadCount}
                        visibleThreadMessages={visibleThreadMessages}
                        virtualTopPad={virtualTopPad}
                        virtualBottomPad={virtualBottomPad}
                        virtualThreadRows={virtualThreadRows}
                        setThreadViewportHeight={setThreadViewportHeight}
                        setThreadScrollTop={setThreadScrollTop}
                        followThreadBottom={followThreadBottom}
                        setFollowThreadBottom={setFollowThreadBottom}
                        showEmojiPalette={showEmojiPalette}
                        setShowEmojiPalette={setShowEmojiPalette}
                        draftMessage={draftMessage}
                        setDraftMessage={setDraftMessage}
                        sendDraftMessage={sendDraftMessage}
                      />
                    </Show>
                    <Show when={state().rightPanel === "devices"}>
                      <DevicesPanel
                        cn={cn}
                        showDeviceConnectDialog={showDeviceConnectDialog}
                        setShowDeviceConnectDialog={setShowDeviceConnectDialog}
                        connectPlatform={connectPlatform}
                        setConnectPlatform={setConnectPlatform}
                        profilePublicKeyInput={profilePublicKeyInput}
                        setProfilePublicKeyInput={setProfilePublicKeyInput}
                        requestedLabelInput={requestedLabelInput}
                        setRequestedLabelInput={setRequestedLabelInput}
                        reserveDomain={reserveDomain}
                        reserveBusy={reserveBusy}
                        reserveStatus={reserveStatus}
                        reserveError={reserveError}
                        connectDomain={connectDomain}
                        setConnectDomain={setConnectDomain}
                        connectRegistrationToken={connectRegistrationToken}
                        setConnectRegistrationToken={setConnectRegistrationToken}
                        issuePairingCode={issuePairingCode}
                        pairingBusy={pairingBusy}
                        pairingStatus={pairingStatus}
                        pairingError={pairingError}
                        pairingExpiresAt={pairingExpiresAt}
                        pairingCodeInput={pairingCodeInput}
                        setPairingCodeInput={setPairingCodeInput}
                        linuxConnectScript={linuxConnectScript}
                        copyConnectScript={copyConnectScript}
                        deviceConnectCopied={deviceConnectCopied}
                        fleetDevices={fleetDevices}
                        selectedDevice={selectedDevice}
                        selectedDeviceId={selectedDeviceId}
                        setSelectedDeviceId={setSelectedDeviceId}
                        onOpenTerminal={() => openWindow("terminal")}
                        onOpenFiles={() => openWindow("files")}
                      />
                    </Show>
                  </div>
                  <DrawerIntegrationSuggestions
                    side="right"
                    panel={state().rightPanel}
                    items={rightPanelSuggestions}
                  />
                </div>
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
            <For each={LEFT_DRAWER_PANEL_ITEMS}>
              {(item) => (
                <button
                  type="button"
                  onClick={() => toggleWorkflowDrawer({ side: "left", panel: item.id })}
                  class={cn(DRAWER_ICON_BUTTON_CLASS, state().leftOpen && state().leftPanel === item.id && "text-[hsl(var(--primary))]")}
                  title={item.title}
                >
                  <item.Icon size={16} />
                </button>
              )}
            </For>
          </div>
        </Motion.div>

        <Motion.div
          initial={{ x: 0 }}
          animate={{ x: state().rightOpen ? -360 : 0 }}
          transition={{ duration: 0.28, easing: [0.4, 0, 0.2, 1] }}
          class="fixed right-0 top-1/2 z-[10034] -translate-y-1/2 rounded-l-xl p-1"
        >
          <div class="flex flex-col items-center gap-1">
            <For each={RIGHT_DRAWER_PANEL_ITEMS}>
              {(item) => (
                <button
                  type="button"
                  onClick={() => toggleWorkflowDrawer({ side: "right", panel: item.id })}
                  class={cn(DRAWER_ICON_BUTTON_CLASS, state().rightOpen && state().rightPanel === item.id && "text-[hsl(var(--primary))]")}
                  title={item.title}
                >
                  <item.Icon size={16} />
                </button>
              )}
            </For>
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
