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
  const previewText = firstNonEmptyString([
    typeof chat?.preview?.body === "string" ? chat.preview.body : "",
    typeof chat?.preview?.senderName === "string" ? `${chat.preview.senderName}: ${chat.preview.type || ""}` : "",
    typeof chat?.preview?.type === "string" ? chat.preview.type : "",
    "Open in Beeper Desktop to continue."
  ]);
  const updatedAt = firstNonEmptyString([
    typeof chat?.lastActivity === "string" ? chat.lastActivity : "",
    typeof chat?.preview?.timestamp === "string" ? chat.preview.timestamp : ""
  ]);
  return {
    id: `bridge-beeper-${chatId}`,
    kind: "bridge",
    channel: "beeper",
    title,
    subtitle: network,
    updatedAt,
    preview: previewText,
    messages: []
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
            const beeperResponse = await fetch(`/api/beeper/chats?limit=80&token=${encodeURIComponent(beeperToken)}`);
            const beeperPayload = await beeperResponse.json().catch(() => ({}));
            if (beeperResponse.ok && beeperPayload?.ok !== false) {
              const beeperItems = Array.isArray(beeperPayload?.items) ? beeperPayload.items : [];
              providerThreads.push(...beeperItems
                .map((item, index) => normalizeBeeperChat(item, index))
                .sort((a, b) => new Date(b.updatedAt || 0).getTime() - new Date(a.updatedAt || 0).getTime()));
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
