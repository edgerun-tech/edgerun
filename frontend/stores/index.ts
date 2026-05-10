/**
 * Canonical frontend store exports.
 *
 * Prefer importing from this barrel for app/desktop state. It makes the active
 * state model visible and keeps legacy aliases contained in their owning files.
 */

export {
  appSurfacesStore,
  appSurfaceOrderStore,
  focusedAppSurfaceStore,
  openAppSurface,
  closeAppSurface,
  focusAppSurface,
  systemStatsStore,
  pendingGateStore,
  widgetVisibleStore,
  terminalLogsStore,
  runningAppsStore,
  addLog,
  type AppSurfaceDef,
  type AppSurfaceKind,
  type AppSurfaceSlot,
  type LogEntry,
  type PendingGate,
} from "./desktop-store"

export {
  launchApp,
  launchAppById,
  handleCloseAppSurface,
  getAppIcon,
} from "./app-launcher"

export {
  executeUiAction,
  executeUiActions,
  executeUiCommand,
  focusAssistantInput,
  focusDockInput,
  getUiCommandHelp,
  registerUiCommandHandler,
  type UiAction,
  type UiCommandSource,
} from "./ui-command-center"

export {
  assistantElapsedMsStore,
  assistantAbortControllerStore,
  assistantLastDurationStore,
  assistantLoadingStore,
  assistantMessagesStore,
  assistantStartedAtStore,
  assistantStatusStore,
  abortAssistantRequest,
  appendAssistantMessage,
  finishAssistantRequest,
  getAssistantUserMessageContents,
  isAssistantAbortError,
  setAssistantStatus,
  setAssistantAbortController,
  startAssistantRequest,
  updateAssistantMessages,
  type AssistantMessage,
  type AssistantStatus,
} from "./assistant-store"

export {
  installedAppIdsStore,
  installApp,
  uninstallApp,
  isAppInstalled,
  isCoreApp,
  normalizeAppId,
  CORE_APP_IDS,
  DEFAULT_INSTALLED_APP_IDS,
} from "./installed-apps-store"

export {
  localCapabilityGrantsStore,
  listLocalGrantsForApp,
  grantLocalCapabilities,
  revokeLocalCapabilityGrant,
  revokeAllLocalCapabilityGrants,
  hasLocalCapabilityGrant,
  getMissingCapabilities,
  type LocalCapabilityGrant,
} from "./local-capability-grants-store"

export {
  authStore,
  isAuthenticatedStore,
  type AuthStore,
} from "./auth-store"

export {
  codebaseStore,
} from "./codebase-context"

export {
  wasmCacheStore,
  installWasm,
  getWasmFromCache,
  removeWasm,
  fetchWasmPackage,
} from "./wasm-store"

export {
  fileSystemStore,
} from "./file-system-store"

export {
  toastsStore,
  addToast,
  dismissToast,
} from "./toast-store"

export {
  trustManagerStore,
} from "./trust-manager-store"

export {
  workflowStore,
} from "./workflow-store"

export {
  uiSettingsStore,
} from "./ui-settings-store"

export {
  clientMountedStore,
} from "./ui-runtime-store"

export {
  projectChecklistStore,
} from "./project-checklist-store"

export {
  relayStateStore,
  devHealthStore,
  cdpToolsStore,
} from "./dev-tools-store"
