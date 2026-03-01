import { createEffect, createSignal } from "solid-js";
import { parseEmailAddress, parseEmailName } from "./workflow-overlay.utils";

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
