import { UI_EVENT_TOPICS, UI_INTENT_TOPICS, uiIntentMeta } from "../lib/ui-intents";
import { publishEvent, subscribeEvent } from "./eventbus";

let uiActionBridgeInitialized = false;

function toggleIntentBar() {
  publishEvent(UI_INTENT_TOPICS.action.intentBarToggle, {}, uiIntentMeta("ui-actions"));
}

function openProfileBootstrap() {
  publishEvent(UI_INTENT_TOPICS.action.profileBootstrapOpen, {}, uiIntentMeta("ui-actions"));
}

function navigateBrowser(url) {
  publishEvent(UI_INTENT_TOPICS.action.browserNavigate, { url: String(url || "") }, uiIntentMeta("ui-actions"));
}

function ringCall(contact, mode = "call") {
  publishEvent(
    UI_INTENT_TOPICS.action.callRing,
    { contact: String(contact || ""), mode: mode === "video" ? "video" : "call" },
    uiIntentMeta("ui-actions")
  );
}

function sendTerminalInput(text, execute = true, final = true) {
  publishEvent(
    UI_INTENT_TOPICS.action.terminalInput,
    { text: String(text || ""), execute: Boolean(execute), final: Boolean(final) },
    uiIntentMeta("ui-actions")
  );
}

function resetWidgetPositions() {
  publishEvent(UI_INTENT_TOPICS.action.widgetsResetPositions, {}, uiIntentMeta("ui-actions"));
}

function initializeUiActionBridge() {
  if (uiActionBridgeInitialized) return;
  uiActionBridgeInitialized = true;

  subscribeEvent(UI_INTENT_TOPICS.action.intentBarToggle, (event) => {
    publishEvent(UI_EVENT_TOPICS.action.intentBarToggled, event?.payload || {}, uiIntentMeta("ui-actions.bridge"));
  });
  subscribeEvent(UI_INTENT_TOPICS.action.profileBootstrapOpen, (event) => {
    publishEvent(UI_EVENT_TOPICS.action.profileBootstrapOpened, event?.payload || {}, uiIntentMeta("ui-actions.bridge"));
  });
  subscribeEvent(UI_INTENT_TOPICS.action.browserNavigate, (event) => {
    publishEvent(UI_EVENT_TOPICS.action.browserNavigated, event?.payload || {}, uiIntentMeta("ui-actions.bridge"));
  });
  subscribeEvent(UI_INTENT_TOPICS.action.callRing, (event) => {
    publishEvent(UI_EVENT_TOPICS.action.callRinging, event?.payload || {}, uiIntentMeta("ui-actions.bridge"));
  });
  subscribeEvent(UI_INTENT_TOPICS.action.terminalInput, (event) => {
    publishEvent(UI_EVENT_TOPICS.action.terminalInputSent, event?.payload || {}, uiIntentMeta("ui-actions.bridge"));
  });
  subscribeEvent(UI_INTENT_TOPICS.action.widgetsResetPositions, (event) => {
    publishEvent(UI_EVENT_TOPICS.action.widgetsPositionsReset, event?.payload || {}, uiIntentMeta("ui-actions.bridge"));
  });
}

export {
  initializeUiActionBridge,
  navigateBrowser,
  openProfileBootstrap,
  resetWidgetPositions,
  ringCall,
  sendTerminalInput,
  toggleIntentBar
};
