import { createEffect, createSignal } from "solid-js";
import { parseEmailAddress, parseEmailName } from "./workflow-overlay.utils";
import { integrationStore } from "../../stores/integrations";

function firstNonEmptyString(values = []) {
  for (const value of values) {
    if (typeof value === "string" && value.trim()) return value.trim();
  }
  return "";
}

function normalizeGoogleContact(item, index) {
  const names = Array.isArray(item?.names) ? item.names : [];
  const emailAddresses = Array.isArray(item?.emailAddresses) ? item.emailAddresses : [];
  const emails = Array.isArray(item?.emails) ? item.emails : [];
  const explicitEmail = typeof item?.email === "string" ? item.email : "";
  const email = parseEmailAddress(firstNonEmptyString([
    explicitEmail,
    emailAddresses[0]?.value,
    emailAddresses[0]?.email,
    emails[0]?.value,
    emails[0]?.email,
    typeof emails[0] === "string" ? emails[0] : ""
  ]));
  const name = firstNonEmptyString([
    typeof item?.name === "string" ? item.name : "",
    names[0]?.displayName,
    names[0]?.unstructuredName,
    names[0]?.givenName,
    names[0]?.familyName,
    email,
    "Unnamed"
  ]);
  const rawId = firstNonEmptyString([
    typeof item?.resourceName === "string" ? item.resourceName : "",
    typeof item?.id === "string" ? item.id : "",
    email
  ]) || `google-contact-${index}`;

  return {
    id: `contact-${rawId.replace(/[^a-zA-Z0-9:_-]/g, "-")}`,
    name,
    email,
    source: "Google Contacts"
  };
}

function beeperMediaProxyUrl(uri) {
  const value = String(uri || "").trim();
  if (!value) return "";
  if (/^https?:\/\//i.test(value) || /^data:image\//i.test(value)) return value;
  if (value.startsWith("file://") || value.startsWith("mxc://")) {
    return `/api/beeper/media?uri=${encodeURIComponent(value)}`;
  }
  return "";
}

function normalizeBeeperChat(chat, index) {
  const chatId = firstNonEmptyString([
    typeof chat?.id === "string" ? chat.id : "",
    typeof chat?.chatID === "string" ? chat.chatID : "",
    typeof chat?.localChatID === "string" ? chat.localChatID : ""
  ]) || `beeper-chat-${index}`;
  const title = firstNonEmptyString([
    typeof chat?.title === "string" ? chat.title : "",
    typeof chat?.user?.displayText === "string" ? chat.user.displayText : "",
    typeof chat?.user?.fullName === "string" ? chat.user.fullName : "",
    "Beeper chat"
  ]);
  const network = firstNonEmptyString([
    typeof chat?.network === "string" ? chat.network : "",
    typeof chat?.accountID === "string" ? chat.accountID : "",
    "Beeper"
  ]);
  const previewType = String(chat?.preview?.type || "").trim().toUpperCase();
  const attachmentCount = Array.isArray(chat?.preview?.attachments) ? chat.preview.attachments.length : 0;
  const previewTextRaw = firstNonEmptyString([
    typeof chat?.preview?.text === "string" ? chat.preview.text : "",
    typeof chat?.preview?.body === "string" ? chat.preview.body : "",
    typeof chat?.preview?.caption === "string" ? chat.preview.caption : ""
  ]);
  const senderPrefix = typeof chat?.preview?.senderName === "string" && chat.preview.senderName.trim()
    ? `${chat.preview.senderName.trim()}: `
    : "";
  const mediaFallback = attachmentCount > 0
    ? `${senderPrefix}[${previewType === "IMAGE" ? "Photo" : (previewType || "Attachment")}]`
    : "";
  const previewText = firstNonEmptyString([
    previewTextRaw ? `${senderPrefix}${previewTextRaw}` : "",
    mediaFallback,
    typeof chat?.preview?.type === "string" ? `${senderPrefix}${chat.preview.type}` : "",
    "Open in Beeper Desktop to continue."
  ]);
  const updatedAt = firstNonEmptyString([
    typeof chat?.lastActivity === "string" ? chat.lastActivity : "",
    typeof chat?.preview?.timestamp === "string" ? chat.preview.timestamp : ""
  ]);
  const participants = Array.isArray(chat?.participants?.items) ? chat.participants.items : [];
  const nonSelfParticipants = participants.filter((item) => !item?.isSelf);
  const singleChat = String(chat?.type || "").toLowerCase() === "single";
  const candidateAvatar = firstNonEmptyString([
    singleChat && nonSelfParticipants.length === 1 && typeof nonSelfParticipants[0]?.imgURL === "string"
      ? nonSelfParticipants[0].imgURL
      : "",
    singleChat && typeof chat?.user?.imgURL === "string" ? chat.user.imgURL : ""
  ]);
  const avatarUrl = beeperMediaProxyUrl(candidateAvatar);
  const previewMessageId = firstNonEmptyString([
    typeof chat?.preview?.id === "string" ? chat.preview.id : "",
    `${chatId}-preview`
  ]);
  const previewTimestamp = firstNonEmptyString([
    typeof chat?.preview?.timestamp === "string" ? chat.preview.timestamp : "",
    updatedAt,
    new Date().toISOString()
  ]);
  const previewMessages = previewText
    ? [{
      id: previewMessageId,
      role: "contact",
      text: previewText,
      createdAt: previewTimestamp,
      channel: "beeper",
      author: firstNonEmptyString([
        typeof chat?.preview?.senderName === "string" ? chat.preview.senderName : "",
        title
      ])
    }]
    : [];
  return {
    id: `bridge-beeper-${chatId}`,
    kind: "bridge",
    channel: "beeper",
    title,
    subtitle: network,
    updatedAt,
    avatarUrl,
    sourceChatId: chatId,
    preview: previewText,
    messages: previewMessages
  };
}

function normalizeBeeperMessage(message = {}, fallbackChannel = "beeper") {
  const attachment = Array.isArray(message?.attachments) ? message.attachments[0] : null;
  const attachmentUrlRaw = firstNonEmptyString([
    typeof attachment?.srcURL === "string" ? attachment.srcURL : "",
    typeof attachment?.url === "string" ? attachment.url : ""
  ]);
  const attachmentUrl = beeperMediaProxyUrl(attachmentUrlRaw);
  const attachmentTypeRaw = firstNonEmptyString([
    typeof attachment?.type === "string" ? attachment.type : "",
    typeof message?.type === "string" ? message.type : "",
    "attachment"
  ]).toLowerCase();
  const attachmentType = attachmentTypeRaw === "image" || attachmentTypeRaw === "img"
    ? "Photo"
    : attachmentTypeRaw === "video"
      ? "Video"
      : attachmentTypeRaw === "audio"
        ? "Audio"
        : attachmentTypeRaw === "file"
          ? "File"
          : (attachmentTypeRaw || "Attachment");
  const text = firstNonEmptyString([
    typeof message?.text === "string" ? message.text : "",
    typeof message?.body === "string" ? message.body : "",
    typeof message?.caption === "string" ? message.caption : "",
    attachmentUrl ? `[${attachmentType}]\n${attachmentUrl}` : `[${attachmentType}]`
  ]);
  return {
    id: firstNonEmptyString([
      typeof message?.id === "string" ? message.id : "",
      typeof message?.sortKey === "string" ? message.sortKey : "",
      `${Date.now()}-${Math.random().toString(16).slice(2, 8)}`
    ]),
    role: message?.isSender ? "user" : "contact",
    text,
    createdAt: firstNonEmptyString([
      typeof message?.timestamp === "string" ? message.timestamp : "",
      new Date().toISOString()
    ]),
    channel: fallbackChannel,
    author: firstNonEmptyString([
      typeof message?.senderName === "string" ? message.senderName : "",
      message?.isSender ? "You" : "Contact"
    ]),
    attachmentUrl,
    attachmentType
  };
}

export function useWorkflowConversationSources({ messageProviderIntegrations, localMessagesByConversation }) {
  const [emailThreads, setEmailThreads] = createSignal([]);
  const [bridgeThreads, setBridgeThreads] = createSignal([]);
  const [contacts, setContacts] = createSignal([]);
  const [contactsLoading, setContactsLoading] = createSignal(false);

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

      const providerThreads = messageProviderIntegrations()
        .filter((provider) => provider.id !== "email" && provider.id !== "beeper")
        .map((provider) => {
          const conversationId = `bridge-${provider.id}`;
          const history = Array.isArray(localMessagesByConversation()[conversationId])
            ? localMessagesByConversation()[conversationId]
            : [];
          const last = history[history.length - 1] || null;
          return {
            id: conversationId,
            kind: "bridge",
            channel: provider.id,
            title: provider.name,
            subtitle: provider.available
              ? "Connected via integration"
              : (provider.availabilityReason || "Not connected"),
            updatedAt: last?.createdAt || "",
            preview: last?.text || (provider.available
              ? "Integration connected. Send a message to start this thread."
              : "Connect this provider in Integrations to enable unified messaging."),
            messages: []
          };
        })
        .sort((a, b) => a.title.localeCompare(b.title));

      const beeperIntegration = messageProviderIntegrations().find((provider) => provider.id === "beeper");
      if (beeperIntegration && beeperIntegration.available) {
        const beeperToken = integrationStore.getToken("beeper");
        if (beeperToken) {
          try {
            const beeperResponse = await fetch(`/api/beeper/chats?limit=200&token=${encodeURIComponent(beeperToken)}`);
            const beeperPayload = await beeperResponse.json().catch(() => ({}));
            if (beeperResponse.ok && beeperPayload?.ok !== false) {
              const beeperItems = Array.isArray(beeperPayload?.items) ? beeperPayload.items : [];
              const normalizedBeeperThreads = beeperItems
                .map((item, index) => normalizeBeeperChat(item, index))
                .sort((a, b) => new Date(b.updatedAt || 0).getTime() - new Date(a.updatedAt || 0).getTime());

              const threadMessages = new Map();
              await Promise.all(normalizedBeeperThreads.slice(0, 30).map(async (thread) => {
                if (!thread.sourceChatId) return;
                try {
                  const messageResponse = await fetch(`/api/beeper/messages?chat_id=${encodeURIComponent(thread.sourceChatId)}&limit=40&token=${encodeURIComponent(beeperToken)}`);
                  const messagePayload = await messageResponse.json().catch(() => ({}));
                  if (!messageResponse.ok || messagePayload?.ok === false) return;
                  const items = Array.isArray(messagePayload?.items) ? messagePayload.items : [];
                  const normalized = items
                    .map((item) => normalizeBeeperMessage(item, "beeper"))
                    .filter((item) => item.text)
                    .sort((a, b) => new Date(a.createdAt || 0).getTime() - new Date(b.createdAt || 0).getTime());
                  if (normalized.length > 0) threadMessages.set(thread.sourceChatId, normalized);
                } catch {
                  // ignore per-thread message fetch failures
                }
              }));

              providerThreads.push(...normalizedBeeperThreads.map((thread) => {
                const messages = threadMessages.get(thread.sourceChatId) || thread.messages || [];
                const last = messages[messages.length - 1] || null;
                const previewLine = String(last?.text || "").split("\n")[0] || "";
                return {
                  ...thread,
                  preview: previewLine
                    ? `${String(last?.author || "Contact")}: ${previewLine}`
                    : (thread.preview || ""),
                  updatedAt: last?.createdAt || thread.updatedAt,
                  messages
                };
              }));
            } else {
              providerThreads.push({
                id: "bridge-beeper",
                kind: "bridge",
                channel: "beeper",
                title: "Beeper",
                subtitle: "Desktop API unreachable",
                updatedAt: "",
                preview: beeperPayload?.error || `Beeper chats failed (${beeperResponse.status})`,
                messages: []
              });
            }
          } catch {
            providerThreads.push({
              id: "bridge-beeper",
              kind: "bridge",
              channel: "beeper",
              title: "Beeper",
              subtitle: "Desktop API unreachable",
              updatedAt: "",
              preview: "Beeper Desktop API is not reachable from this runtime.",
              messages: []
            });
          }
        }

        try {
          const importedResponse = await fetch("/api/beeper/imported?limit_threads=300&limit_messages=150");
          const importedPayload = await importedResponse.json().catch(() => ({}));
          if (importedResponse.ok && importedPayload?.ok !== false) {
            const importedItems = Array.isArray(importedPayload?.items) ? importedPayload.items : [];
            const existingIds = new Set(providerThreads.map((thread) => thread.id));
            providerThreads.push(...importedItems
              .filter((thread) => thread && typeof thread === "object" && !existingIds.has(String(thread.id || "")))
              .map((thread) => ({
                id: String(thread.id || `bridge-beeper-import-${Math.random().toString(16).slice(2, 8)}`),
                kind: "bridge",
                channel: "beeper",
                title: String(thread.title || "Imported Thread"),
                subtitle: String(thread.subtitle || "Imported from Facebook export"),
                updatedAt: String(thread.updatedAt || ""),
                preview: String(thread.preview || "Imported history"),
                messages: Array.isArray(thread.messages) ? thread.messages : []
              }))
              .sort((a, b) => new Date(b.updatedAt || 0).getTime() - new Date(a.updatedAt || 0).getTime()));
          }
        } catch {
          // ignore imported dataset fetch failures
        }
      }
      setBridgeThreads(providerThreads);

      const googleToken = window.localStorage.getItem("google_token");
      if (!googleToken) {
        setContacts([]);
        return;
      }

      setContactsLoading(true);
      try {
        const response = await fetch(`/api/google/contacts?limit=100&token=${encodeURIComponent(googleToken)}`);
        const payload = await response.json().catch(() => ({}));
        if (!response.ok || payload?.ok === false) {
          throw new Error(payload?.error || `Google contacts request failed (${response.status})`);
        }
        const items = Array.isArray(payload?.items) ? payload.items : [];
        const normalized = items
          .map((item, index) => normalizeGoogleContact(item, index))
          .filter((contact) => contact.name || contact.email)
          .sort((a, b) => a.name.localeCompare(b.name));
        setContacts(normalized);
      } catch {
        setContacts([]);
      } finally {
        setContactsLoading(false);
      }
    };

    loadConversationSources();
  });

  return {
    emailThreads,
    bridgeThreads,
    contacts,
    contactsLoading
  };
}
