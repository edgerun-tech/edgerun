import { createSignal } from "solid-js";
import { UI_EVENT_TOPICS, UI_INTENT_TOPICS, uiIntentMeta } from "../lib/ui-intents";
import { publishEvent, subscribeEvent } from "./eventbus";

/**
 * @typedef {"editor" | "files" | "integrations" | "github" | "email" | "settings" | "widgets" | "onvif" | "terminal" | "cloud" | "call" | "cloudflare" | "drive" | "calendar" | "browser" | "photos" | "credentials" | "guide"} WindowId
 */

/**
 * @typedef {object} WindowState
 * @property {string} title
 * @property {boolean} isOpen
 * @property {boolean=} isMinimized
 * @property {boolean=} isMaximized
 * @property {{x: number, y: number}=} position
 * @property {{width: number, height: number}=} size
 * @property {string=} workspaceId
 * @property {number=} zIndex
 */

const WINDOW_PRESETS = {
  editor: { width: 980, height: 680 },
  files: { width: 920, height: 640 },
  integrations: { width: 860, height: 620 },
  github: { width: 980, height: 700 },
  email: { width: 940, height: 700 },
  settings: { width: 820, height: 620 },
  widgets: { width: 560, height: 460 },
  onvif: { width: 980, height: 700 },
  terminal: { width: 940, height: 620 },
  cloud: { width: 980, height: 700 },
  call: { width: 980, height: 700 },
  cloudflare: { width: 980, height: 700 },
  drive: { width: 940, height: 680 },
  calendar: { width: 940, height: 680 },
  browser: { width: 1020, height: 760 },
  photos: { width: 1020, height: 760 },
  credentials: { width: 920, height: 700 },
  guide: { width: 900, height: 680 }
};

const [windows, setWindows] = createSignal({});
const [activeWindowId, setActiveWindowId] = createSignal(null);
const [zCounter, setZCounter] = createSignal(1);
const [windowLayerOffset, setWindowLayerOffset] = createSignal({ x: 0, y: 0 });

let subscriptionsInitialized = false;

const defaultTitle = (id) => id.charAt(0).toUpperCase() + id.slice(1);

function getActiveWindowId() {
  return activeWindowId;
}

function getWindowLayerOffset() {
  return windowLayerOffset;
}

function shiftWindowLayer(dx, dy) {
  setWindowLayerOffset((prev) => ({ x: prev.x + dx, y: prev.y + dy }));
}

function resetWindowLayer() {
  setWindowLayerOffset({ x: 0, y: 0 });
}

function nextZ() {
  const value = zCounter() + 1;
  setZCounter(value);
  return 1000 + value;
}

function getOpenWindowsSorted() {
  return Object.entries(windows())
    .filter(([_, state]) => state?.isOpen)
    .sort((a, b) => (a[1].zIndex ?? 0) - (b[1].zIndex ?? 0));
}

function getTopWindowId() {
  const sorted = getOpenWindowsSorted();
  const top = sorted[sorted.length - 1];
  return top ? /** @type {WindowId} */ (top[0]) : null;
}

function initializeDefaultWindows() {
  return;
}

function getViewportSize() {
  if (typeof window === "undefined") {
    return { width: 1440, height: 900 };
  }
  return { width: window.innerWidth, height: window.innerHeight };
}

function getCirclePosition(index, size, total) {
  const viewport = getViewportSize();
  const offset = windowLayerOffset();
  const centerX = viewport.width / 2;
  const centerY = viewport.height / 2 - 34;
  const safeTotal = Math.max(total, 1);
  const radius = 220 + Math.min(180, safeTotal * 14);
  const angle = -Math.PI / 2 + (index / safeTotal) * Math.PI * 2;
  const raw = {
    x: Math.round(centerX + Math.cos(angle) * radius - size.width / 2 - offset.x),
    y: Math.round(centerY + Math.sin(angle) * radius - size.height / 2 - offset.y)
  };
  const margin = 24;
  return {
    x: Math.min(
      Math.max(raw.x, margin - offset.x),
      Math.max(margin - offset.x, viewport.width - size.width - margin - offset.x)
    ),
    y: Math.min(
      Math.max(raw.y, margin - offset.y),
      Math.max(margin - offset.y, viewport.height - size.height - margin - offset.y)
    )
  };
}

function emitWindowIntent(topic, payload = {}) {
  publishEvent(topic, payload, uiIntentMeta("windows.store"));
}

function applyBringToFront(id) {
  const z = nextZ();
  setWindows((prev) => ({
    ...prev,
    [id]: {
      ...prev[id],
      title: prev[id]?.title ?? defaultTitle(id),
      isOpen: true,
      isMinimized: false,
      workspaceId: prev[id]?.workspaceId ?? "default",
      zIndex: z
    }
  }));
  setActiveWindowId(id);
}

function applyOpenWindow(id) {
  const current = windows();
  const openCount = Object.values(current).filter((w) => w?.isOpen).length;
  const existing = current[id];
  const idealSize = existing?.size ?? WINDOW_PRESETS[id] ?? { width: 920, height: 640 };
  const nextPosition = existing?.position ?? getCirclePosition(openCount, idealSize, openCount + 1);
  const z = nextZ();

  setWindows((prev) => ({
    ...prev,
    [id]: {
      title: prev[id]?.title ?? defaultTitle(id),
      isOpen: true,
      isMinimized: false,
      isMaximized: false,
      workspaceId: "default",
      position: nextPosition,
      size: idealSize,
      zIndex: z
    }
  }));
  setActiveWindowId(id);
}

function applyCloseWindow(id) {
  setWindows((prev) => ({
    ...prev,
    [id]: {
      ...prev[id] ?? { title: defaultTitle(id) },
      isOpen: false,
      isMinimized: false
    }
  }));
  queueMicrotask(() => {
    const nextTop = getTopWindowId();
    if (nextTop) {
      setActiveWindowId(nextTop);
    }
  });
}

function applyMinimizeWindow(id) {
  setWindows((prev) => ({
    ...prev,
    [id]: {
      ...prev[id] ?? { title: defaultTitle(id), isOpen: true },
      isOpen: true,
      isMinimized: true
    }
  }));
  queueMicrotask(() => {
    const nextTop = getTopWindowId();
    if (nextTop) {
      setActiveWindowId(nextTop);
    }
  });
}

function applyMaximizeWindow(id) {
  applyBringToFront(id);
  setWindows((prev) => ({
    ...prev,
    [id]: {
      ...prev[id] ?? { title: defaultTitle(id), isOpen: true },
      isOpen: true,
      isMaximized: true,
      isMinimized: false
    }
  }));
}

function applyRestoreWindow(id) {
  applyBringToFront(id);
  setWindows((prev) => ({
    ...prev,
    [id]: {
      ...prev[id] ?? { title: defaultTitle(id), isOpen: true },
      isOpen: true,
      isMaximized: false,
      isMinimized: false
    }
  }));
}

function applyUpdateWindowPosition(id, position) {
  setWindows((prev) => ({
    ...prev,
    [id]: {
      ...prev[id] ?? { title: defaultTitle(id), isOpen: true },
      position
    }
  }));
}

function applyUpdateWindowSize(id, size) {
  setWindows((prev) => ({
    ...prev,
    [id]: {
      ...prev[id] ?? { title: defaultTitle(id), isOpen: true },
      size
    }
  }));
}

function applyCloseAllWindows() {
  setWindows((prev) => {
    const next = { ...prev };
    for (const id of Object.keys(next)) {
      next[id] = {
        ...next[id],
        isOpen: false,
        isMinimized: false
      };
    }
    return next;
  });
  setActiveWindowId(null);
}

function ensureWindowIntentSubscriptions() {
  if (subscriptionsInitialized) return;
  subscriptionsInitialized = true;

  subscribeEvent(UI_INTENT_TOPICS.window.open, (event) => {
    const id = String(event?.payload?.windowId || "").trim();
    if (!id) return;
    applyOpenWindow(id);
    publishEvent(UI_EVENT_TOPICS.window.opened, { windowId: id }, uiIntentMeta("windows.reducer"));
  });

  subscribeEvent(UI_INTENT_TOPICS.window.close, (event) => {
    const id = String(event?.payload?.windowId || "").trim();
    if (!id) return;
    applyCloseWindow(id);
    publishEvent(UI_EVENT_TOPICS.window.closed, { windowId: id }, uiIntentMeta("windows.reducer"));
  });

  subscribeEvent(UI_INTENT_TOPICS.window.closeTop, () => {
    const id = getTopWindowId();
    if (!id) return;
    applyCloseWindow(id);
    publishEvent(UI_EVENT_TOPICS.window.closed, { windowId: id }, uiIntentMeta("windows.reducer"));
  });

  subscribeEvent(UI_INTENT_TOPICS.window.closeAll, () => {
    applyCloseAllWindows();
    publishEvent(UI_EVENT_TOPICS.window.closedAll, {}, uiIntentMeta("windows.reducer"));
  });

  subscribeEvent(UI_INTENT_TOPICS.window.minimize, (event) => {
    const id = String(event?.payload?.windowId || "").trim();
    if (!id) return;
    applyMinimizeWindow(id);
    publishEvent(UI_EVENT_TOPICS.window.minimized, { windowId: id }, uiIntentMeta("windows.reducer"));
  });

  subscribeEvent(UI_INTENT_TOPICS.window.maximize, (event) => {
    const id = String(event?.payload?.windowId || "").trim();
    if (!id) return;
    applyMaximizeWindow(id);
    publishEvent(UI_EVENT_TOPICS.window.maximized, { windowId: id }, uiIntentMeta("windows.reducer"));
  });

  subscribeEvent(UI_INTENT_TOPICS.window.restore, (event) => {
    const id = String(event?.payload?.windowId || "").trim();
    if (!id) return;
    applyRestoreWindow(id);
    publishEvent(UI_EVENT_TOPICS.window.restored, { windowId: id }, uiIntentMeta("windows.reducer"));
  });

  subscribeEvent(UI_INTENT_TOPICS.window.focus, (event) => {
    const id = String(event?.payload?.windowId || "").trim();
    if (!id) return;
    applyBringToFront(id);
    publishEvent(UI_EVENT_TOPICS.window.focused, { windowId: id }, uiIntentMeta("windows.reducer"));
  });

  subscribeEvent(UI_INTENT_TOPICS.window.move, (event) => {
    const id = String(event?.payload?.windowId || "").trim();
    const position = event?.payload?.position;
    if (!id || !position) return;
    applyUpdateWindowPosition(id, position);
    publishEvent(UI_EVENT_TOPICS.window.moved, { windowId: id, position }, uiIntentMeta("windows.reducer"));
  });

  subscribeEvent(UI_INTENT_TOPICS.window.resize, (event) => {
    const id = String(event?.payload?.windowId || "").trim();
    const size = event?.payload?.size;
    if (!id || !size) return;
    applyUpdateWindowSize(id, size);
    publishEvent(UI_EVENT_TOPICS.window.resized, { windowId: id, size }, uiIntentMeta("windows.reducer"));
  });
}

ensureWindowIntentSubscriptions();

function bringWindowToFront(id) {
  emitWindowIntent(UI_INTENT_TOPICS.window.focus, { windowId: id });
}

function openWindow(id) {
  emitWindowIntent(UI_INTENT_TOPICS.window.open, { windowId: id });
}

function closeWindow(id) {
  emitWindowIntent(UI_INTENT_TOPICS.window.close, { windowId: id });
}

function closeTopWindow() {
  emitWindowIntent(UI_INTENT_TOPICS.window.closeTop, {});
}

function minimizeWindow(id) {
  emitWindowIntent(UI_INTENT_TOPICS.window.minimize, { windowId: id });
}

function maximizeWindow(id) {
  emitWindowIntent(UI_INTENT_TOPICS.window.maximize, { windowId: id });
}

function restoreWindow(id) {
  emitWindowIntent(UI_INTENT_TOPICS.window.restore, { windowId: id });
}

function updateWindowPosition(id, position) {
  emitWindowIntent(UI_INTENT_TOPICS.window.move, { windowId: id, position });
}

function updateWindowSize(id, size) {
  emitWindowIntent(UI_INTENT_TOPICS.window.resize, { windowId: id, size });
}

function closeAllWindows() {
  emitWindowIntent(UI_INTENT_TOPICS.window.closeAll, {});
}

export {
  bringWindowToFront,
  closeAllWindows,
  closeTopWindow,
  closeWindow,
  getActiveWindowId,
  getTopWindowId,
  getWindowLayerOffset,
  initializeDefaultWindows,
  maximizeWindow,
  minimizeWindow,
  openWindow,
  resetWindowLayer,
  restoreWindow,
  shiftWindowLayer,
  updateWindowPosition,
  updateWindowSize,
  windows
};
