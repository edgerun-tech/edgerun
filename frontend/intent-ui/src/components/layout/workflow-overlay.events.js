import { publishEvent } from "../../stores/eventbus";

export function emitConversationChatHeadUpdated(conversationId, chatHead) {
  publishEvent("conversation.chat_head.updated", { conversationId, ...chatHead }, { source: "browser" });
}

export function emitConversationMessageSent(conversationId, text, channel) {
  publishEvent("conversation.message.sent", { conversationId, text, channel }, { source: "browser" });
}

export function emitClipboardHistoryCleared() {
  publishEvent("clipboard.history.cleared", { source: "drawer" }, { source: "browser" });
}
