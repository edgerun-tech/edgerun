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
  grantLocalCapabilities,
  revokeLocalCapabilities,
  revokeAllLocalCapabilityGrants,
  hasLocalCapabilityGrant,
  getMissingCapabilities,
} from "./local-capability-grants-store"
