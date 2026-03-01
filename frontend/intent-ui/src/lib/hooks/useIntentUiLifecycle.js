import { onCleanup, onMount } from "solid-js";
import { closeTopWindow, openWindow } from "../../stores/windows";
import {
  closeWorkflowDemo,
  hydrateWorkflowUiFromStorage,
  openAssistantResponse,
  switchWorkflowSession,
  workflowUi
} from "../../stores/workflow-ui";
import {
  eventTimeline,
  initializeBrowserEventBus,
  publishEvent,
  subscribeEvent
} from "../../stores/eventbus";
import { UI_EVENT_TOPICS } from "../ui-intents";
import { initializeUiActionBridge, toggleIntentBar } from "../../stores/ui-actions";
import { hydrateProfileRuntime } from "../../stores/profile-runtime";

function readRegisteredDomain() {
  if (typeof window === "undefined") return "";
  const keys = [
    "intent-ui-user-domain-v1",
    "intent-ui-domain-v1",
    "intent-ui-domain-reservation-v1",
    "edgerun_user_domain"
  ];
  for (const key of keys) {
    const raw = localStorage.getItem(key);
    if (!raw) continue;
    const trimmed = String(raw).trim();
    if (!trimmed) continue;
    if (trimmed.startsWith("{")) {
      try {
        const parsed = JSON.parse(trimmed);
        const value = String(parsed?.domain || parsed?.assignedDomain || parsed?.fqdn || "").trim();
        if (value) return value;
      } catch {
        // ignore parse failures
      }
    } else {
      return trimmed;
    }
  }
  return "";
}

function publishRuntimeStarted() {
  publishEvent("browser.runtime.started", { app: "intent-ui" }, { source: "browser" });
}

function setupIntentDebugApi() {
  if (typeof window === "undefined") return;
  window.__intentDebug = window.__intentDebug || {};
  window.__intentDebug.openWindow = (id) => openWindow(id);
  window.__intentDebug.askAssistant = (prompt, options = {}) => openAssistantResponse(prompt, options);
  window.__intentDebug.switchSession = (selector) => switchWorkflowSession(selector);
  window.__intentDebug.getWorkflowUi = () => workflowUi();
  window.__intentDebug.getEventBusRuntime = () => eventBusRuntime();
  window.__intentDebug.getEventBusTimeline = () => eventTimeline();
  window.__intentDebug.publishEvent = (topic, payload = {}) => publishEvent(topic, payload, { source: "intent-debug" });
}

function cleanupIntentDebugApi() {
  if (typeof window === "undefined" || !window.__intentDebug?.openWindow) return;
  delete window.__intentDebug.openWindow;
  delete window.__intentDebug.askAssistant;
  delete window.__intentDebug.switchSession;
  delete window.__intentDebug.getWorkflowUi;
  delete window.__intentDebug.getEventBusRuntime;
  delete window.__intentDebug.getEventBusTimeline;
  delete window.__intentDebug.publishEvent;
}

export function useIntentUiLifecycle(params) {
  let handleGlobalHotkeys;
  let handleGlobalKeyUp;
  let handleWindowBlur;
  let handleGlobalContextMenu;
  let handleGlobalClick;
  let handleContextEscape;
  let handleStorageUpdate;
  let layerIndicatorTimeout;
  let unsubscribeBootstrapOpen;

  const updateLayerFromEvent = (event) => {
    if (layerIndicatorTimeout) {
      clearTimeout(layerIndicatorTimeout);
      layerIndicatorTimeout = null;
    }
    if (event.altKey) {
      params.setInputLayer(2);
      params.setShowLayerIndicator(true);
      return;
    }
    if (event.ctrlKey) {
      params.setInputLayer(0);
      params.setShowLayerIndicator(true);
      return;
    }
    params.setInputLayer(1);
    params.setShowLayerIndicator(true);
    layerIndicatorTimeout = window.setTimeout(() => {
      params.setShowLayerIndicator(false);
      layerIndicatorTimeout = null;
    }, 1000);
  };

  onMount(() => {
    hydrateProfileRuntime();
    hydrateWorkflowUiFromStorage();
    void initializeBrowserEventBus();
    initializeUiActionBridge();
    publishRuntimeStarted();
    params.setIsClient(true);

    handleGlobalHotkeys = (event) => {
      updateLayerFromEvent(event);
      const target = event.target;
      if (
        target instanceof HTMLElement &&
        (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable) &&
        event.key !== "Meta"
      ) {
        return;
      }

      const isSuperKey =
        (event.key === "Meta" || event.code === "MetaLeft" || event.code === "MetaRight") &&
        !event.repeat &&
        !event.shiftKey &&
        !event.ctrlKey &&
        !event.altKey;

      if (isSuperKey) {
        event.preventDefault();
        toggleIntentBar();
        return;
      }

      if (event.key === "Escape") {
        if (workflowUi().isOpen) {
          closeWorkflowDemo();
          return;
        }
        closeTopWindow();
      }
    };

    handleGlobalKeyUp = (event) => {
      updateLayerFromEvent(event);
    };
    handleWindowBlur = () => {
      params.setInputLayer(1);
      params.setShowLayerIndicator(false);
      if (layerIndicatorTimeout) {
        clearTimeout(layerIndicatorTimeout);
        layerIndicatorTimeout = null;
      }
    };
    handleGlobalContextMenu = (event) => {
      event.preventDefault();
    };
    handleGlobalClick = () => {
      params.setMenuOpen(false);
      params.setAccountMenuOpen(false);
    };
    handleContextEscape = (event) => {
      if (event.key === "Escape") {
        params.setMenuOpen(false);
      }
    };

    window.addEventListener("keydown", handleGlobalHotkeys);
    window.addEventListener("keyup", handleGlobalKeyUp);
    window.addEventListener("blur", handleWindowBlur);
    window.addEventListener("contextmenu", handleGlobalContextMenu, { capture: true });
    window.addEventListener("pointerdown", handleGlobalClick);
    window.addEventListener("keydown", handleContextEscape);
    unsubscribeBootstrapOpen = subscribeEvent(UI_EVENT_TOPICS.action.profileBootstrapOpened, (event) => {
      if (!event?.payload?.manual) return;
      if (typeof params.setShowBootstrapGate === "function") {
        params.setShowBootstrapGate(true);
      }
    });
    handleStorageUpdate = () => params.setRegisteredDomain(readRegisteredDomain());
    window.addEventListener("storage", handleStorageUpdate);
    params.setRegisteredDomain(readRegisteredDomain());
    setupIntentDebugApi();
  });

  onCleanup(() => {
    if (handleGlobalHotkeys) window.removeEventListener("keydown", handleGlobalHotkeys);
    if (handleGlobalKeyUp) window.removeEventListener("keyup", handleGlobalKeyUp);
    if (handleWindowBlur) window.removeEventListener("blur", handleWindowBlur);
    if (handleGlobalContextMenu) window.removeEventListener("contextmenu", handleGlobalContextMenu, { capture: true });
    if (handleGlobalClick) window.removeEventListener("pointerdown", handleGlobalClick);
    if (handleContextEscape) window.removeEventListener("keydown", handleContextEscape);
    if (unsubscribeBootstrapOpen) unsubscribeBootstrapOpen();
    if (handleStorageUpdate) window.removeEventListener("storage", handleStorageUpdate);
    if (layerIndicatorTimeout) {
      clearTimeout(layerIndicatorTimeout);
      layerIndicatorTimeout = null;
    }
    cleanupIntentDebugApi();
  });
}
