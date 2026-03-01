import { onCleanup, onMount } from "solid-js";

const SUPER_V_SHORTCUT_EVENT = "intent-ui-super-v";
const CALL_LINK_READY_EVENT = "intent-ui-call-link-ready";

export function useWorkflowOverlayEvents(options) {
  const {
    state,
    activeBubbleDrag,
    setChatBubbles,
    setActiveBubbleDrag,
    clampBubblePosition,
    toggleConversationsPanel,
    setConversationTab,
    setShowConversationList,
    setShowConversationSettings,
    setShowEmojiPalette,
    setSelectedConversationId,
    conversationDraftInputRef,
    threadSearchInputRef,
    setLocalMessagesByConversation,
    onSuperVPaste
  } = options;

  let handleSuperVShortcut;
  let handleCallLinkReady;
  let handleBubblePointerMove;
  let handleBubblePointerUp;
  let handleThreadSearchShortcut;

  onMount(() => {
    handleSuperVShortcut = () => {
      if (!(state().rightOpen && state().rightPanel === "conversations")) {
        toggleConversationsPanel();
      }
      setConversationTab("threads");
      setShowConversationList(false);
      setShowConversationSettings(false);
      setShowEmojiPalette(true);
      setSelectedConversationId("ai-active");
      queueMicrotask(() => {
        const input = conversationDraftInputRef();
        if (input instanceof HTMLElement) input.focus();
      });
      onSuperVPaste();
    };

    handleBubblePointerMove = (event) => {
      const drag = activeBubbleDrag();
      if (!drag) return;
      const nextX = event.clientX - drag.offsetX;
      const nextY = event.clientY - drag.offsetY;
      const clamped = clampBubblePosition(nextX, nextY);
      setChatBubbles((prev) => prev.map((bubble) => bubble.id === drag.id
        ? { ...bubble, x: clamped.x, y: clamped.y, updatedAt: Date.now() }
        : bubble
      ));
    };

    handleBubblePointerUp = () => {
      setActiveBubbleDrag(null);
    };

    handleCallLinkReady = (event) => {
      const detail = event?.detail || {};
      const threadId = String(detail.threadId || "").trim();
      const link = String(detail.link || "").trim();
      const title = String(detail.title || "Pending call").trim();
      const subtitle = String(detail.subtitle || "Awaiting recipient").trim();
      if (!threadId || !link) return;
      const now = new Date().toISOString();
      const pendingEntry = {
        id: `local-${threadId}-${Date.now()}`,
        role: "assistant",
        text: `Pending call link copied: ${link}`,
        createdAt: now,
        channel: "call",
        author: "Call Studio",
        threadTitle: title,
        threadSubtitle: subtitle,
        callStatus: "pending"
      };
      setLocalMessagesByConversation((prev) => ({
        ...prev,
        [threadId]: [...(prev[threadId] || []), pendingEntry]
      }));
      setSelectedConversationId(threadId);
    };

    handleThreadSearchShortcut = (event) => {
      if (event.defaultPrevented || event.key !== "/") return;
      const target = event.target;
      const isInputTarget = target instanceof HTMLElement
        && (target.closest("input, textarea, [contenteditable='true']") || target.isContentEditable);
      if (isInputTarget) return;
      if (!(state().rightOpen && state().rightPanel === "conversations")) {
        toggleConversationsPanel();
      }
      setConversationTab("threads");
      setShowConversationList(true);
      event.preventDefault();
      queueMicrotask(() => {
        const input = threadSearchInputRef();
        if (input instanceof HTMLElement) input.focus();
      });
    };

    window.addEventListener(SUPER_V_SHORTCUT_EVENT, handleSuperVShortcut);
    window.addEventListener(CALL_LINK_READY_EVENT, handleCallLinkReady);
    window.addEventListener("pointermove", handleBubblePointerMove);
    window.addEventListener("pointerup", handleBubblePointerUp);
    window.addEventListener("keydown", handleThreadSearchShortcut);
  });

  onCleanup(() => {
    if (handleSuperVShortcut) {
      window.removeEventListener(SUPER_V_SHORTCUT_EVENT, handleSuperVShortcut);
    }
    if (handleBubblePointerMove) {
      window.removeEventListener("pointermove", handleBubblePointerMove);
    }
    if (handleCallLinkReady) {
      window.removeEventListener(CALL_LINK_READY_EVENT, handleCallLinkReady);
    }
    if (handleBubblePointerUp) {
      window.removeEventListener("pointerup", handleBubblePointerUp);
    }
    if (handleThreadSearchShortcut) {
      window.removeEventListener("keydown", handleThreadSearchShortcut);
    }
  });
}
