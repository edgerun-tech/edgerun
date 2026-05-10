import { base64ToBytes } from "@/platform/utils/bytes"
import type {
  ContactRecord,
  OAuthAppId,
  OAuthProfileSecret,
  OAuthProfileSecretInput,
  ProfileEvent,
  ProfilePreferences,
  StoredNodeRegistration,
  UnlockedProfileContainer,
  WebAuthnBinding,
} from "@/stores/auth-types"
import { DEFAULT_PROFILE_PREFERENCES } from "@/stores/auth-types"

export function profileInitials(handle: string): string {
  const initials = handle.trim().split(/\s+/).map((part) => part[0]).join("").slice(0, 2).toUpperCase()
  return initials || "ID"
}

export function profileColor(seed: string): string {
  const hue = seed.split("").reduce((acc, char) => acc + char.charCodeAt(0), 0) % 360
  return `oklch(0.3 0.1 ${hue})`
}

export function sanitizeProfilePreferences(value: unknown, handle = ""): ProfilePreferences {
  const input = value && typeof value === "object" ? value as Partial<ProfilePreferences> : {}
  const initials = typeof input.avatarInitials === "string" && input.avatarInitials.trim()
    ? input.avatarInitials.trim().slice(0, 2).toUpperCase()
    : profileInitials(handle)
  return {
    avatarInitials: initials,
    avatarColor: typeof input.avatarColor === "string" && input.avatarColor.trim() ? input.avatarColor.trim() : profileColor(handle),
    connectToNetwork: typeof input.connectToNetwork === "boolean" ? input.connectToNetwork : DEFAULT_PROFILE_PREFERENCES.connectToNetwork,
    shareResources: typeof input.shareResources === "boolean" ? input.shareResources : DEFAULT_PROFILE_PREFERENCES.shareResources,
  }
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === "object" && !Array.isArray(value))
}

export function isIsoDate(value: unknown): value is string {
  return typeof value === "string" && value.length > 0 && Number.isFinite(Date.parse(value))
}

export function isHex(value: unknown, length?: number): value is string {
  return typeof value === "string" && /^[a-f0-9]+$/i.test(value) && (!length || value.length === length)
}

export function validBase64(value: unknown): value is string {
  if (typeof value !== "string" || !value) return false
  try {
    base64ToBytes(value)
    return true
  } catch {
    return false
  }
}

export function normalizeOAuthSecret(secret: unknown): OAuthProfileSecret | null {
  if (!isRecord(secret)) return null
  const appId = secret.appId
  if (appId !== "gmail" && appId !== "google-drive" && appId !== "github" && appId !== "cloudflare") return null
  if (secret.kind !== "oauth2") return null
  if (typeof secret.email !== "string" || typeof secret.accessToken !== "string" || !secret.accessToken.trim()) return null
  if (!isIsoDate(secret.expiresAtIso) || !isIsoDate(secret.updatedAtIso)) return null
  if (!Array.isArray(secret.scopes) || !secret.scopes.every((scope) => typeof scope === "string")) return null

  const normalized: OAuthProfileSecretInput & { appId: OAuthAppId; kind: "oauth2"; updatedAtIso: string } = {
    appId,
    kind: "oauth2",
    email: secret.email,
    accessToken: secret.accessToken,
    refreshToken: typeof secret.refreshToken === "string" ? secret.refreshToken : undefined,
    expiresAtIso: secret.expiresAtIso,
    scopes: secret.scopes,
    updatedAtIso: secret.updatedAtIso,
  }
  if (appId === "cloudflare") {
    if (isHex(secret.accountId, 32)) normalized.accountId = secret.accountId
    if (isHex(secret.tokenId, 32)) normalized.tokenId = secret.tokenId
    if (isHex(secret.zoneId, 32)) normalized.zoneId = secret.zoneId
    if (normalized.zoneId && typeof secret.zoneName === "string" && secret.zoneName.trim()) normalized.zoneName = secret.zoneName.trim()
  }
  return normalized
}

export function validateOAuthSecretInput(appId: OAuthAppId, secret: OAuthProfileSecretInput) {
  if (typeof secret.email !== "string") throw new Error("Profile secret is missing a connector label.")
  if (typeof secret.accessToken !== "string" || !secret.accessToken.trim()) throw new Error("Profile secret is missing an access token.")
  if (!isIsoDate(secret.expiresAtIso)) throw new Error("Profile secret has an invalid expiration timestamp.")
  if (!Array.isArray(secret.scopes) || !secret.scopes.every((scope) => typeof scope === "string")) throw new Error("Profile secret scopes are invalid.")
  if (appId === "cloudflare") {
    if (secret.accountId && !isHex(secret.accountId, 32)) throw new Error("Cloudflare account ID must be a 32-character hex value.")
    if (secret.tokenId && !isHex(secret.tokenId, 32)) throw new Error("Cloudflare token ID must be a 32-character hex value.")
    if (secret.zoneId && !isHex(secret.zoneId, 32)) throw new Error("Cloudflare zone ID must be a 32-character hex value.")
    if (secret.zoneName && !secret.zoneId) throw new Error("Cloudflare zone name cannot be saved without a zone ID.")
  }
}

export function selfContact(profile: Pick<UnlockedProfileContainer, "handle" | "ownerEncryption">): ContactRecord {
  return {
    id: profile.ownerEncryption.identityIdHex,
    label: `${profile.handle} self`,
    publicKeyRawBase64: profile.ownerEncryption.publicKeyRawBase64,
    identityIdHex: profile.ownerEncryption.identityIdHex,
    routeHint: "local-profile",
    addedAtIso: new Date().toISOString(),
  }
}

export function normalizeUnlockedProfile(profile: UnlockedProfileContainer): UnlockedProfileContainer {
  if (profile.version !== 1) throw new Error("Profile validation failed: unsupported profile version.")
  if (typeof profile.handle !== "string" || !profile.handle.trim()) throw new Error("Profile validation failed: missing handle.")
  if (!isIsoDate(profile.createdAtIso)) throw new Error("Profile validation failed: invalid creation timestamp.")
  for (const [label, key] of [["owner", profile.owner], ["owner encryption", profile.ownerEncryption], ["browser node", profile.browserNode]] as const) {
    if (!isHex(key.identityIdHex)) throw new Error(`Profile validation failed: invalid ${label} identity.`)
    if (!validBase64(key.publicKeyRawBase64) || !validBase64(key.privateKeyPkcs8Base64)) throw new Error(`Profile validation failed: invalid ${label} key material.`)
  }
  return {
    ...profile,
    handle: profile.handle.trim(),
    nodes: Array.isArray(profile.nodes) && profile.nodes.length ? profile.nodes : [profile.browserNode],
    contacts: Array.isArray(profile.contacts) ? profile.contacts : [selfContact(profile)],
    eventLog: Array.isArray(profile.eventLog) ? profile.eventLog : [],
    profilePreferences: sanitizeProfilePreferences(profile.profilePreferences, profile.handle),
    webAuthnBinding: deriveWebAuthnBinding(Array.isArray(profile.eventLog) ? profile.eventLog : [], profile.webAuthnBinding),
    appSecrets: Array.isArray(profile.appSecrets) ? profile.appSecrets.map(normalizeOAuthSecret).filter((secret): secret is OAuthProfileSecret => Boolean(secret)) : [],
    sealedContainers: Array.isArray(profile.sealedContainers) ? profile.sealedContainers : [],
    outbox: Array.isArray(profile.outbox) ? profile.outbox : [],
  }
}

export function deriveProfilePreferences(events: ProfileEvent[], fallback: unknown, handle: string): ProfilePreferences {
  let preferences = sanitizeProfilePreferences(fallback, handle)
  for (const event of events) {
    if (event.kind === "PROFILE_SETTINGS_UPDATED") {
      preferences = sanitizeProfilePreferences(event.payload.preferences, handle)
    }
  }
  return preferences
}

export function deriveWebAuthnBinding(events: ProfileEvent[], fallback?: WebAuthnBinding): WebAuthnBinding | undefined {
  let binding = fallback
  for (const event of events) {
    if (event.kind === "PROFILE_WEBAUTHN_BOUND") binding = event.payload
  }
  return binding
}

export function registrationFor(profile: UnlockedProfileContainer): StoredNodeRegistration {
  const genesis = profile.eventLog[0]
  return {
    nodeId: profile.browserNode.identityIdHex,
    nodeTarget: "browser-contained",
    username: profile.handle,
    ownerId: profile.owner.identityIdHex,
    genesisEventHash: genesis?.eventHash ?? "",
    registeredAtIso: profile.createdAtIso,
  }
}

export function sanitizeKnownNodeIds(value: unknown): string[] {
  if (!Array.isArray(value)) return []
  return Array.from(new Set(value
    .map((item) => typeof item === "string" ? item.trim() : "")
    .filter(Boolean)))
}
