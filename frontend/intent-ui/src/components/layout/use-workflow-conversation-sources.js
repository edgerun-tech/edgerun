import { createEffect, createSignal } from "solid-js";
import { parseEmailAddress, parseEmailName } from "./workflow-overlay.utils";

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
        .filter((provider) => provider.id !== "email")
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
              ? "Connected via Matrix bridge"
              : (provider.availabilityReason || "Not connected"),
            updatedAt: last?.createdAt || "",
            preview: last?.text || (provider.available
              ? "Bridge connected. Send a message to start this thread."
              : "Connect this provider in Integrations to enable unified messaging."),
            messages: []
          };
        })
        .sort((a, b) => a.title.localeCompare(b.title));
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

  return {
    emailThreads,
    bridgeThreads,
    contacts,
    contactsLoading
  };
}
