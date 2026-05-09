/**
 * Canonical OS surface exports.
 *
 * New production imports should come from this barrel instead of deep-linking
 * into random OS files. Legacy draggable-window files are intentionally not
 * exported here.
 */

export { Desktop } from "./desktop"
export { XrayDesktopSurface } from "./xray-desktop-surface"
export { AppOverlayHost } from "./app-overlay-host"
export { TopBar } from "./top-bar"
export { AuthOverlay } from "./auth-overlay"
export { EdgerunLogo } from "./edgerun-logo"

export { SettingsApp } from "./settings-app"
export { TrustManagerSurface } from "./trust-manager-surface"
export { TrustContainerApp } from "./trust-container-app"
export { IdentityApp } from "./identity-app"
export { ProfileMenu } from "./profile-menu"
export { FileManager } from "./file-manager"
export { GmailApp } from "./gmail-app"
export { AppStore } from "./app-store"
