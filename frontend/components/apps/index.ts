/**
 * Canonical app surface exports.
 *
 * This folder represents full app-level UI. During migration, these exports may
 * point at mature implementations that still physically live under
 * `components/os`. New app surfaces should land here directly.
 */

export { SettingsApp } from "@/components/os/settings-app"
export { IdentityApp } from "@/components/os/identity-app"
export { FileManager } from "@/components/os/file-manager"
export { GmailApp } from "@/components/os/gmail-app"
export { GoogleDriveApp } from "@/components/os/google-drive-app"
export { GooglePhotosApp } from "@/components/os/google-photos-app"
export { GoogleContactsApp } from "@/components/os/google-contacts-app"
export { GitHubApp } from "@/components/os/github-app"
export { CloudflareApp } from "@/components/os/cloudflare-app"
export { AppStore } from "@/components/os/app-store"
