import { createSignal } from "solid-js";
import { UI_EVENT_TOPICS, UI_INTENT_TOPICS, uiIntentMeta } from "../lib/ui-intents";
import { publishEvent, subscribeEvent } from "./eventbus";

const CLIPBOARD_HISTORY_KEY = "intent-ui-clipboard-history-v1";
const CLIPBOARD_HISTORY_MAX = 48;
let clipboardSubscriptionsInitialized = false;

function safeParse(raw) {
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function readHistory() {
  if (typeof window === "undefined") return [];
  const parsed = safeParse(localStorage.getItem(CLIPBOARD_HISTORY_KEY) || "");
  return Array.isArray(parsed) ? parsed : [];
}

function persistHistory(entries) {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(CLIPBOARD_HISTORY_KEY, JSON.stringify(entries.slice(0, CLIPBOARD_HISTORY_MAX)));
  } catch {
    // ignore storage failures
  }
}

const [clipboardHistory, setClipboardHistory] = createSignal(readHistory());

function applyClipboardPush(text, source = "unknown") {
  const value = String(text || "").trim();
  if (!value) return;
  setClipboardHistory((prev) => {
    const deduped = prev.filter((entry) => String(entry?.text || "") !== value);
    const next = [{
      id: `clip-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      text: value,
      source,
      createdAt: new Date().toISOString()
    }, ...deduped].slice(0, CLIPBOARD_HISTORY_MAX);
    persistHistory(next);
    return next;
  });
  publishEvent(UI_EVENT_TOPICS.clipboard.updated, { text: value, source }, uiIntentMeta("clipboard.reducer"));
  publishEvent("clipboard.updated", { text: value, source }, { scope: "browser" });
}

function applyClipboardClear() {
  setClipboardHistory([]);
  persistHistory([]);
  publishEvent(UI_EVENT_TOPICS.clipboard.cleared, {}, uiIntentMeta("clipboard.reducer"));
  publishEvent("clipboard.cleared", {}, { scope: "browser" });
}

function ensureClipboardIntentSubscriptions() {
  if (clipboardSubscriptionsInitialized) return;
  clipboardSubscriptionsInitialized = true;

  subscribeEvent(UI_INTENT_TOPICS.clipboard.push, (event) => {
    applyClipboardPush(event?.payload?.text, event?.payload?.source || "unknown");
  });

  subscribeEvent(UI_INTENT_TOPICS.clipboard.clear, () => {
    applyClipboardClear();
  });
}

ensureClipboardIntentSubscriptions();

function pushClipboardEntry(text, source = "unknown") {
  publishEvent(UI_INTENT_TOPICS.clipboard.push, { text, source }, uiIntentMeta("clipboard.store"));
}

function clearClipboardHistory() {
  publishEvent(UI_INTENT_TOPICS.clipboard.clear, {}, uiIntentMeta("clipboard.store"));
}

export {
  clipboardHistory,
  clearClipboardHistory,
  pushClipboardEntry
};
