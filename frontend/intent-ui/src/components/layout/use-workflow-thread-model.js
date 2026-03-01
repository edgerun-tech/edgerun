import { createEffect, createMemo } from "solid-js";

function parseConversationTime(value) {
  const ts = new Date(value || 0).getTime();
  return Number.isFinite(ts) ? ts : 0;
}

export function useWorkflowThreadModel(options) {
  const {
    state,
    localMessagesByConversation,
    contactOnlyThreads,
    bridgeThreads,
    emailThreads,
    threadSourceFilter,
    setThreadSourceFilter,
    threadSearchQuery,
    selectedConversationId,
    setSelectedConversationId
  } = options;

  const aiConversation = createMemo(() => {
    const latestMessage = state().messages[state().messages.length - 1];
    return {
      id: "ai-active",
      kind: "ai",
      channel: "ai",
      title: state().prompt || "Active AI session",
      subtitle: state().provider || "opencode",
      sessionId: state().sessionId || "",
      updatedAt: latestMessage?.createdAt || (state().streaming ? new Date().toISOString() : ""),
      preview: (latestMessage?.text || "").trim() || (state().streaming ? "Streaming..." : "No response yet"),
      messages: (state().messages || []).map((message) => ({
        id: message.id,
        role: message.role === "user" ? "user" : "assistant",
        text: message.text || "",
        createdAt: message.createdAt,
        channel: "ai",
        author: message.role === "user" ? "You" : "Assistant"
      }))
    };
  });

  const callPendingThreads = createMemo(() => {
    const entries = Object.entries(localMessagesByConversation() || {});
    return entries
      .filter(([conversationId, messages]) => (
        String(conversationId || "").startsWith("call-link-")
        && Array.isArray(messages)
        && messages.length > 0
      ))
      .map(([conversationId, messages]) => {
        const last = messages[messages.length - 1] || {};
        const title = String(last.threadTitle || "").trim() || "Pending call";
        const subtitle = String(last.threadSubtitle || "").trim() || "Awaiting recipient";
        const updatedAt = String(last.createdAt || "").trim() || new Date().toISOString();
        const preview = String(last.text || "").trim() || "Call link copied";
        return {
          id: conversationId,
          kind: "call",
          channel: "call",
          title,
          subtitle,
          updatedAt,
          preview,
          messages: []
        };
      })
      .sort((a, b) => new Date(b.updatedAt || 0).getTime() - new Date(a.updatedAt || 0).getTime());
  });

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

  const allThreadConversations = createMemo(() => [
    ...callPendingThreads(),
    aiConversation(),
    ...sessionConversations(),
    ...bridgeThreads(),
    ...emailThreads(),
    ...contactOnlyThreads()
  ].map((thread) => {
    const localMessages = Array.isArray(localMessagesByConversation()[thread.id])
      ? localMessagesByConversation()[thread.id]
      : [];
    const localLast = localMessages[localMessages.length - 1] || null;
    const threadMessages = Array.isArray(thread.messages) ? thread.messages : [];
    const threadLast = threadMessages[threadMessages.length - 1] || null;
    const activityTs = Math.max(
      parseConversationTime(thread.updatedAt),
      parseConversationTime(threadLast?.createdAt),
      parseConversationTime(localLast?.createdAt)
    );
    return {
      ...thread,
      updatedAt: activityTs > 0 ? new Date(activityTs).toISOString() : (thread.updatedAt || ""),
      preview: String(localLast?.text || "").trim() || thread.preview || thread.subtitle || "No messages yet",
      activityTs
    };
  }).sort((a, b) => {
    if (b.activityTs !== a.activityTs) return b.activityTs - a.activityTs;
    return String(a.title || "").localeCompare(String(b.title || ""));
  }));

  const threadSourceOptions = createMemo(() => {
    const channels = new Set();
    for (const thread of allThreadConversations()) {
      const value = String(thread?.channel || "").trim();
      if (value) channels.add(value);
    }
    return ["all", ...Array.from(channels).sort((a, b) => a.localeCompare(b))];
  });

  const threadConversations = createMemo(() => {
    const filter = String(threadSourceFilter() || "all").trim().toLowerCase();
    const query = String(threadSearchQuery() || "").trim().toLowerCase();
    const sourceScoped = !filter || filter === "all"
      ? allThreadConversations()
      : allThreadConversations().filter((thread) => String(thread.channel || "").toLowerCase() === filter);
    if (!query) return sourceScoped;
    return sourceScoped.filter((thread) => {
      const haystack = [
        thread.title,
        thread.subtitle,
        thread.preview,
        thread.channel,
        Array.isArray(thread.participants) ? thread.participants.join(" ") : ""
      ].join(" ").toLowerCase();
      return haystack.includes(query);
    });
  });

  const hasConversationContent = createMemo(() => {
    const threads = allThreadConversations();
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

  createEffect(() => {
    const options = threadSourceOptions();
    if (!options.includes(threadSourceFilter())) {
      setThreadSourceFilter("all");
    }
  });

  createEffect(() => {
    const current = activeConversation();
    if (!current) return;
    if (selectedConversationId()) return;
    setSelectedConversationId(current.id);
  });

  createEffect(() => {
    const selectedId = selectedConversationId();
    if (!selectedId) return;
    const existsInFiltered = threadConversations().some((thread) => thread.id === selectedId);
    if (existsInFiltered) return;
    const fallback = threadConversations()[0];
    if (fallback?.id) {
      setSelectedConversationId(fallback.id);
    }
  });

  return {
    allThreadConversations,
    threadSourceOptions,
    threadConversations,
    hasConversationContent,
    activeConversation,
    activeConversationMessages
  };
}
