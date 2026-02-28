import { createSignal } from "solid-js";
import { publishEvent } from "./eventbus";

const CLIPBOARD_HISTORY_KEY = "intent-ui-clipboard-history-v1";
const CLIPBOARD_HISTORY_MAX = 48;

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

function pushClipboardEntry(text, source = "unknown") {
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
  publishEvent("clipboard.updated", { text: value, source }, { scope: "browser" });
}

function clearClipboardHistory() {
  setClipboardHistory([]);
  persistHistory([]);
  publishEvent("clipboard.cleared", {}, { scope: "browser" });
}

export {
  clipboardHistory,
  clearClipboardHistory,
  pushClipboardEntry
};
