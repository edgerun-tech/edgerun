import { Show, For, createEffect, createMemo, onCleanup } from "solid-js";
import {
  TbOutlinePlus,
  TbOutlineUser,
  TbOutlineAdjustments,
  TbOutlineMoodSmile,
  TbOutlineClipboard,
  TbOutlineSend
} from "solid-icons/tb";
import {
  CHAT_HEAD_PRESET_COLORS,
  DRAWER_LIST_ROW_CLASS,
  DRAWER_PANEL_SHELL_CLASS,
  DRAWER_SMALL_BUTTON_CLASS,
  DRAWER_STATE_BLOCK_CLASS,
  EMOJI_QUICK_SET,
  THREAD_PAGE_SIZE
} from "./workflow-overlay.constants";
import { channelBadgeClass } from "./workflow-overlay.utils";
import { clipboardHistory, clearClipboardHistory } from "../../stores/clipboard-history";
import { emitClipboardHistoryCleared } from "./workflow-overlay.events";
import VirtualAnimatedList from "../common/VirtualAnimatedList";

export default function ConversationsPanel(props) {
  let threadScrollRef;
  let threadResizeObserver;
  const safeFallbackChatHead = createMemo(() => {
    const fallback = props.fallbackChatHead;
    if (fallback && typeof fallback === "object") {
      return {
        emoji: String(fallback.emoji || "💬"),
        color: String(fallback.color || CHAT_HEAD_PRESET_COLORS[0]),
        label: String(fallback.label || "C")
      };
    }
    return { emoji: "💬", color: CHAT_HEAD_PRESET_COLORS[0], label: "C" };
  });
  const safeActiveChatHead = createMemo(() => {
    const resolved = typeof props.activeChatHead === "function" ? props.activeChatHead() : null;
    if (resolved && typeof resolved === "object") {
      return {
        emoji: String(resolved.emoji || safeFallbackChatHead().emoji),
        color: String(resolved.color || safeFallbackChatHead().color),
        label: String(resolved.label || safeFallbackChatHead().label)
      };
    }
    return safeFallbackChatHead();
  });

  const ensureThreadBottom = () => {
    if (!threadScrollRef) return;
    threadScrollRef.scrollTop = threadScrollRef.scrollHeight;
  };

  createEffect(() => {
    const messages = props.activeConversationMessages();
    const last = messages[messages.length - 1];
    const signature = `${messages.length}:${last?.id || ""}:${String(last?.text || "").length}:${props.state().streaming ? "1" : "0"}`;
    if (!signature) return;
    if (!props.state().rightOpen || props.state().rightPanel !== "conversations" || props.showConversationList()) return;
    if (!props.followThreadBottom()) return;
    queueMicrotask(() => ensureThreadBottom());
  });

  createEffect(() => {
    if (!props.state().rightOpen || props.state().rightPanel !== "conversations" || props.showConversationList()) return;
    queueMicrotask(() => {
      if (!threadScrollRef) return;
      props.setThreadViewportHeight(Math.max(1, threadScrollRef.clientHeight));
      if (props.followThreadBottom()) ensureThreadBottom();
    });
  });

  onCleanup(() => {
    if (threadResizeObserver) {
      threadResizeObserver.disconnect();
      threadResizeObserver = null;
    }
  });

  return (
    <div class={DRAWER_PANEL_SHELL_CLASS}>
      <Show when={props.showConversationList()}>
        <div class="border-b border-neutral-800 px-3 py-2">
          <div class="flex items-center justify-between gap-2">
            <p class="text-xs font-medium uppercase tracking-wide text-neutral-300">Conversations</p>
            <button type="button" onClick={props.onNewSession} class={DRAWER_SMALL_BUTTON_CLASS}>
              <TbOutlinePlus size={11} />
              New
            </button>
          </div>
          <div class="mt-2 grid grid-cols-2 gap-1 rounded-md border border-neutral-800 bg-neutral-900/60 p-1">
            <button
              type="button"
              onClick={() => props.setConversationTab("threads")}
              class={props.cn(
                "rounded px-2 py-1 text-[11px] transition-colors",
                props.conversationTab() === "threads" ? "font-semibold text-[hsl(var(--primary))]" : "text-neutral-400 hover:bg-neutral-800"
              )}
            >
              Threads
            </button>
            <button
              type="button"
              onClick={() => props.setConversationTab("contacts")}
              class={props.cn(
                "rounded px-2 py-1 text-[11px] transition-colors",
                props.conversationTab() === "contacts" ? "font-semibold text-[hsl(var(--primary))]" : "text-neutral-400 hover:bg-neutral-800"
              )}
            >
              Contacts
            </button>
          </div>
        </div>
        <div class="min-h-0 flex-1 overflow-auto p-3">
          <Show when={props.conversationTab() === "threads"}>
            <div class="space-y-1.5">
              <Show when={props.threadSourceOptions().length > 1}>
                <div class="mb-1.5 flex flex-wrap gap-1.5" data-testid="conversation-thread-source-filter">
                  <For each={props.threadSourceOptions()}>
                    {(source) => (
                      <button
                        type="button"
                        onClick={() => props.setThreadSourceFilter(source)}
                        class={props.cn(
                          "rounded border px-2 py-0.5 text-[10px] uppercase tracking-wide transition-colors",
                          props.threadSourceFilter() === source
                            ? "border-[hsl(var(--primary)/0.5)] bg-[hsl(var(--primary)/0.12)] text-[hsl(var(--primary))]"
                            : "border-neutral-700 bg-neutral-900/50 text-neutral-400 hover:bg-neutral-800"
                        )}
                        data-testid={`conversation-thread-source-option-${source}`}
                      >
                        {source === "all" ? "All" : source}
                      </button>
                    )}
                  </For>
                </div>
              </Show>
              <VirtualAnimatedList
                items={props.threadConversations}
                estimateSize={52}
                overscan={5}
                animateRows
                renderItem={(thread) => (
                  <button
                    type="button"
                    onClick={() => {
                      if (thread.kind === "session") {
                        props.onSelectSessionThread(thread.sessionId);
                        return;
                      }
                      props.setSelectedConversationId(thread.id);
                      props.setShowConversationList(false);
                    }}
                    class={props.cn(
                      props.cn(DRAWER_LIST_ROW_CLASS, "text-left"),
                      props.activeConversation()?.id === thread.id ? "border-neutral-700 bg-neutral-900/85" : ""
                    )}
                    data-testid="conversation-thread-item"
                    data-conversation-channel={thread.channel}
                  >
                    <div class="flex items-center justify-between gap-2">
                      <div class="flex min-w-0 items-center gap-2">
                        <Show
                          when={typeof thread.avatarUrl === "string" && thread.avatarUrl.trim()}
                          fallback={(
                            <span
                              class="inline-flex h-6 w-6 items-center justify-center rounded-full border border-neutral-700 text-[11px]"
                              style={{
                                "background-color": `${(props.chatHeadForConversation(thread)?.color || safeFallbackChatHead().color)}33`,
                                color: props.chatHeadForConversation(thread)?.color || safeFallbackChatHead().color
                              }}
                            >
                              {props.chatHeadForConversation(thread)?.emoji || props.chatHeadForConversation(thread)?.label || safeFallbackChatHead().label}
                            </span>
                          )}
                        >
                          {(avatarUrl) => (
                            <img
                              src={avatarUrl()}
                              alt={thread.title || "Conversation avatar"}
                              class="h-6 w-6 rounded-full border border-neutral-700 object-cover"
                              loading="lazy"
                              referrerPolicy="no-referrer"
                            />
                          )}
                        </Show>
                        <p class={props.cn("truncate text-[11px] text-neutral-200", props.activeConversation()?.id === thread.id ? "font-semibold text-[hsl(var(--primary))]" : "font-medium")}>{thread.title}</p>
                      </div>
                      <span class={props.cn("rounded border px-1.5 py-0.5 text-[9px] uppercase tracking-wide", channelBadgeClass(thread.channel))}>
                        {thread.channel}
                      </span>
                    </div>
                    <p class="mt-1 truncate text-[10px] text-neutral-500">{thread.preview || thread.subtitle || "No messages yet"}</p>
                  </button>
                )}
              />
              <Show when={!props.hasConversationContent()}>
                <div class={DRAWER_STATE_BLOCK_CLASS} data-testid="conversations-empty-state">
                  <p class="text-neutral-300">This is where all your conversations will be available.</p>
                  <p class="mt-1">Connect message provider integrations to unlock threads.</p>
                  <div class="mt-2 space-y-1">
                    <VirtualAnimatedList
                      items={props.messageProviderIntegrations}
                      estimateSize={28}
                      overscan={3}
                      animateRows
                      renderItem={(provider) => (
                        <button
                          type="button"
                          onClick={props.onOpenIntegrations}
                          class="flex w-full items-center justify-between rounded border border-neutral-800 bg-neutral-900/70 px-2 py-1 text-[10px] text-neutral-200 hover:bg-neutral-800"
                          data-testid={`conversation-provider-${provider.id}`}
                        >
                          <span>{provider.name}</span>
                          <span class={provider.available ? "text-emerald-300" : "text-amber-300"}>
                            {provider.available ? "available" : "not ready"}
                          </span>
                        </button>
                      )}
                    />
                  </div>
                </div>
              </Show>
              <Show when={props.hasConversationContent() && props.threadConversations().length === 0}>
                <p class={DRAWER_STATE_BLOCK_CLASS}>No threads match this source filter.</p>
              </Show>
            </div>
          </Show>
          <Show when={props.conversationTab() === "contacts"}>
            <div class="space-y-1.5">
              <Show when={!props.contactsLoading()} fallback={<p class={DRAWER_STATE_BLOCK_CLASS}>Loading contacts...</p>}>
                <VirtualAnimatedList
                  items={props.contacts}
                  estimateSize={60}
                  overscan={4}
                  animateRows
                  renderItem={(contact) => (
                    <button
                      type="button"
                      onClick={() => {
                        const emailThreadId = contact.email ? `email-${contact.email}` : "";
                        const allThreads = props.allThreadConversations();
                        const existing = emailThreadId ? allThreads.find((thread) => thread.id === emailThreadId) : null;
                        const linkedThreadId = Array.isArray(contact.threadIds)
                          ? contact.threadIds.find((threadId) => allThreads.some((thread) => thread.id === threadId))
                          : "";
                        const linkedThread = linkedThreadId
                          ? allThreads.find((thread) => thread.id === linkedThreadId)
                          : null;
                        if (existing) {
                          props.setConversationTab("threads");
                          props.setSelectedConversationId(existing.id);
                          props.setShowConversationList(false);
                          return;
                        }
                        if (linkedThread) {
                          props.setConversationTab("threads");
                          props.setSelectedConversationId(linkedThread.id);
                          props.setShowConversationList(false);
                          return;
                        }
                        const fallbackId = contact.email ? `contact-${contact.email}` : contact.id;
                        props.setContactOnlyThreads((prev) => {
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
                        props.setConversationTab("threads");
                        props.setSelectedConversationId(fallbackId);
                        props.setShowConversationList(false);
                      }}
                      class={DRAWER_LIST_ROW_CLASS}
                    >
                      <div class="flex items-center gap-2">
                        <TbOutlineUser size={12} class="text-[hsl(var(--primary))]" />
                        <p class="truncate text-[11px] font-medium text-neutral-200">{contact.name}</p>
                      </div>
                      <p class="mt-1 truncate text-[10px] text-neutral-500">{contact.email || "No email"}</p>
                      <Show when={Array.isArray(contact.channels) && contact.channels.length > 0}>
                        <p class="mt-1 truncate text-[10px] text-neutral-500">
                          Channels: {contact.channels.join(", ")}
                        </p>
                      </Show>
                    </button>
                  )}
                />
                <Show when={props.contacts().length === 0}>
                  <p class={DRAWER_STATE_BLOCK_CLASS}>No contacts loaded.</p>
                </Show>
              </Show>
            </div>
          </Show>
        </div>
      </Show>

      <Show when={!props.showConversationList()}>
        <div class="flex items-center justify-between border-b border-neutral-800 px-3 py-2">
          <div class="flex items-center gap-2">
            <button type="button" onClick={() => props.setShowConversationList(true)} class={DRAWER_SMALL_BUTTON_CLASS}>
              Back
            </button>
            <button
              type="button"
              onClick={() => props.setShowConversationSettings((prev) => !prev)}
              class={DRAWER_SMALL_BUTTON_CLASS}
              data-testid="conversation-settings-toggle"
            >
              <TbOutlineAdjustments size={11} />
              Settings
            </button>
          </div>
          <div class="flex min-w-0 items-center gap-2">
            <span class="inline-flex h-6 w-6 items-center justify-center rounded-full border border-neutral-700 text-[11px]" style={{ "background-color": `${safeActiveChatHead().color}33`, color: safeActiveChatHead().color }}>
              {safeActiveChatHead().emoji || safeActiveChatHead().label}
            </span>
            <p class="truncate text-[11px] font-medium text-neutral-200">{props.activeConversation()?.title || "Conversation"}</p>
            <span class={props.cn("rounded border px-1.5 py-0.5 text-[9px] uppercase tracking-wide", channelBadgeClass(props.activeConversation()?.channel || "ai"))}>
              {props.activeConversation()?.channel || "ai"}
            </span>
          </div>
        </div>

        <Show when={props.showConversationSettings()}>
          <div class="space-y-2 border-b border-neutral-800 bg-neutral-950/40 px-3 py-2" data-testid="conversation-settings-popup">
            <p class="text-[10px] uppercase tracking-wide text-neutral-500">Message Providers</p>
            <div class="space-y-1">
              <VirtualAnimatedList
                items={props.messageProviderIntegrations}
                estimateSize={28}
                overscan={3}
                animateRows
                renderItem={(provider) => (
                  <button
                    type="button"
                    class="flex w-full items-center justify-between rounded border border-neutral-800 bg-neutral-900/70 px-2 py-1 text-[10px] text-neutral-200 hover:bg-neutral-800"
                    onClick={props.onOpenIntegrations}
                    data-testid={`conversation-settings-provider-${provider.id}`}
                  >
                    <span>{provider.name}</span>
                    <span class={provider.available ? "text-emerald-300" : "text-amber-300"}>
                      {provider.available ? "available" : provider.availabilityReason}
                    </span>
                  </button>
                )}
              />
            </div>
            <p class="pt-1 text-[10px] uppercase tracking-wide text-neutral-500">Chat Head</p>
            <div class="grid grid-cols-6 gap-1">
              <For each={CHAT_HEAD_PRESET_COLORS}>
                {(color) => (
                  <button
                    type="button"
                    class={props.cn("h-6 rounded border", safeActiveChatHead().color === color ? "border-[hsl(var(--primary))]" : "border-neutral-700")}
                    style={{ "background-color": color }}
                    onClick={() => props.persistChatHeadPref(props.activeConversation()?.id, { color })}
                    data-testid={`chat-head-color-${color.replace("#", "")}`}
                  />
                )}
              </For>
            </div>
            <div class="flex flex-wrap gap-1">
              <For each={EMOJI_QUICK_SET.slice(0, 8)}>
                {(emoji) => (
                  <button
                    type="button"
                    class="inline-flex h-7 w-7 items-center justify-center rounded border border-neutral-700 bg-neutral-900 text-sm hover:border-[hsl(var(--primary)/0.45)]"
                    onClick={() => props.persistChatHeadPref(props.activeConversation()?.id, { emoji })}
                  >
                    {emoji}
                  </button>
                )}
              </For>
            </div>
          </div>
        </Show>

        <div
          ref={(el) => {
            threadScrollRef = el;
            props.setThreadViewportHeight(Math.max(1, el.clientHeight));
            if (typeof ResizeObserver !== "undefined") {
              if (threadResizeObserver) threadResizeObserver.disconnect();
              threadResizeObserver = new ResizeObserver(() => {
                props.setThreadViewportHeight(Math.max(1, el.clientHeight));
              });
              threadResizeObserver.observe(el);
            }
          }}
          class="min-h-0 flex-1 overflow-auto p-3"
          data-testid="conversation-thread-scroll"
          onScroll={(event) => {
            const target = event.currentTarget;
            const scrollTop = target.scrollTop;
            const scrollBottomGap = target.scrollHeight - target.clientHeight - scrollTop;
            props.setThreadScrollTop(scrollTop);
            props.setThreadViewportHeight(Math.max(1, target.clientHeight));
            props.setFollowThreadBottom(scrollBottomGap < 80);
            if (scrollTop < 160) {
              const previousHeight = target.scrollHeight;
              const total = props.activeConversationMessages().length;
              props.setLoadedThreadCount((prev) => {
                const next = Math.min(total, prev + THREAD_PAGE_SIZE);
                if (next === prev) return prev;
                queueMicrotask(() => {
                  if (!threadScrollRef) return;
                  const delta = threadScrollRef.scrollHeight - previousHeight;
                  if (delta <= 0) return;
                  threadScrollRef.scrollTop += delta;
                  props.setThreadScrollTop(threadScrollRef.scrollTop);
                });
                return next;
              });
            }
          }}
        >
          <Show
            when={props.activeConversationMessages().length > 0}
            fallback={<p class={DRAWER_STATE_BLOCK_CLASS}>{props.state().streaming ? "Streaming response..." : "No messages in this thread."}</p>}
          >
            <>
              <Show when={props.loadedThreadCount() < props.activeConversationMessages().length}>
                <p class="mb-2 px-1 text-[10px] uppercase tracking-wide text-neutral-500">
                  Scroll up to load older messages ({props.visibleThreadMessages().length}/{props.activeConversationMessages().length})
                </p>
              </Show>
              <VirtualAnimatedList
                items={props.virtualThreadRows}
                estimateSize={112}
                overscan={6}
                scrollTop={props.threadScrollTop}
                viewportHeight={props.threadViewportHeight}
                animateRows
                renderItem={(row) => (
                  <article
                    class={props.cn(
                      "mb-2 rounded-md border p-2",
                      row.message?.role === "user"
                        ? "ml-6 border-[hsl(var(--primary)/0.38)] bg-[hsl(var(--primary)/0.12)]"
                        : "mr-6 border-neutral-700 bg-neutral-900/70"
                    )}
                    data-testid="conversation-thread-message"
                    onContextMenu={(event) => {
                      event.preventDefault();
                      props.onOpenChatBubble?.(row.message);
                    }}
                  >
                    <div class="mb-1 flex items-center justify-between gap-2">
                      <div class="flex items-center gap-1.5">
                        <p class={props.cn("text-[10px] uppercase tracking-wide", row.message?.role === "user" ? "text-[hsl(var(--primary))]" : "text-neutral-300")}>
                          {row.message?.author || (row.message?.role === "user" ? "You" : "Assistant")}
                        </p>
                        <span class={props.cn("rounded border px-1.5 py-0.5 text-[9px] uppercase tracking-wide", channelBadgeClass(row.message?.channel || "ai"))}>
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
                      {row.message?.text || (props.state().streaming && row.message?.role !== "user" ? "..." : "")}
                    </p>
                    <Show when={typeof row.message?.attachmentUrl === "string" && row.message.attachmentUrl.trim()}>
                      {(attachmentUrl) => (
                        <div class="mt-2 overflow-hidden rounded border border-neutral-700/80 bg-black/25">
                          <Show
                            when={String(row.message?.attachmentType || "").toLowerCase() === "photo" || String(row.message?.attachmentType || "").toLowerCase() === "image"}
                            fallback={
                              <a
                                class="block truncate px-2 py-1.5 text-[10px] text-[hsl(var(--primary))] underline-offset-2 hover:underline"
                                href={attachmentUrl()}
                                target="_blank"
                                rel="noopener noreferrer"
                              >
                                Open attachment
                              </a>
                            }
                          >
                            <img
                              src={attachmentUrl()}
                              alt={String(row.message?.attachmentType || "Attachment")}
                              class="max-h-56 w-full object-contain"
                              loading="lazy"
                              referrerPolicy="no-referrer"
                            />
                          </Show>
                        </div>
                      )}
                    </Show>
                  </article>
                )}
              />
            </>
          </Show>
        </div>

        <div class="border-t border-neutral-800 px-3 py-2">
          <div class="mb-1 flex items-center justify-between">
            <div class="flex items-center gap-1.5">
              <button
                type="button"
                class={DRAWER_SMALL_BUTTON_CLASS}
                onClick={() => props.setShowEmojiPalette((prev) => !prev)}
                data-testid="conversation-emoji-toggle"
              >
                <TbOutlineMoodSmile size={11} />
                Emoji
              </button>
              <button
                type="button"
                class={DRAWER_SMALL_BUTTON_CLASS}
                onClick={() => {
                  const clip = clipboardHistory()[0];
                  if (!clip?.text) return;
                  props.setDraftMessage((prev) => `${prev}${prev ? "\n" : ""}${clip.text}`);
                }}
                data-testid="conversation-clipboard-insert"
              >
                <TbOutlineClipboard size={11} />
                Clipboard
              </button>
            </div>
            <button type="button" class={DRAWER_SMALL_BUTTON_CLASS} onClick={props.sendDraftMessage} data-testid="conversation-send-message">
              <TbOutlineSend size={11} />
              Send
            </button>
          </div>
          <Show when={props.showEmojiPalette()}>
            <div class="mb-1 flex flex-wrap gap-1 rounded border border-neutral-800 bg-neutral-900/60 p-1" data-testid="conversation-emoji-palette">
              <For each={EMOJI_QUICK_SET}>
                {(emoji) => (
                  <button
                    type="button"
                    class="inline-flex h-7 w-7 items-center justify-center rounded border border-neutral-700 bg-neutral-900 text-sm hover:border-[hsl(var(--primary)/0.45)]"
                    onClick={() => props.setDraftMessage((prev) => `${prev}${emoji}`)}
                  >
                    {emoji}
                  </button>
                )}
              </For>
            </div>
          </Show>
          <textarea
            ref={props.conversationDraftInputRef}
            value={props.draftMessage()}
            onInput={(event) => props.setDraftMessage(event.currentTarget.value)}
            onKeyDown={(event) => {
              if (event.isComposing) return;
              if (event.key !== "Enter") return;
              if (event.shiftKey) return;
              event.preventDefault();
              props.sendDraftMessage();
            }}
            placeholder="Type a message..."
            class="h-20 w-full resize-none rounded border border-neutral-700 bg-neutral-900 px-2 py-1.5 text-[11px] text-neutral-200 placeholder:text-neutral-500 focus:border-neutral-600 focus:outline-none"
            data-testid="conversation-draft-input"
          />
          <Show when={clipboardHistory().length > 0}>
            <div class="mt-1 flex items-center justify-between gap-2 rounded border border-neutral-800 bg-neutral-900/50 px-2 py-1 text-[10px] text-neutral-400">
              <span class="truncate">Clipboard history: {clipboardHistory()[0]?.text || ""}</span>
              <button
                type="button"
                class="rounded border border-neutral-700 px-1.5 py-0.5 text-neutral-300 hover:border-[hsl(var(--primary)/0.45)]"
                onClick={() => {
                  clearClipboardHistory();
                  emitClipboardHistoryCleared();
                }}
              >
                Clear
              </button>
            </div>
          </Show>
        </div>
      </Show>
    </div>
  );
}
