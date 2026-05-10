import { atom, computed } from "nanostores"
import type {
  AuthState,
  NodeProvisionInput,
  StoredNodeRegistration,
  AuthStore,
  P256KeyMaterial,
  P256EncryptionKeyMaterial,
  ContactRecord,
  ProfilePreferences,
  WebAuthnBinding,
  OAuthAppId,
  OAuthProfileSecret,
  OAuthProfileSecretInput,
  ProfileEvent,
  SealedNestedContainer,
  RoutedSealedEnvelope,
  UnlockedProfileContainer,
  SealedProfileContainer,
  ProfileSummary,
  LocalQueuedMessage,
  NodeRelayPublishResult,
} from "./auth-types"
import {
  LEGACY_PROFILE_STORAGE_KEY,
  PROFILE_INDEX_KEY,
  ACTIVE_PROFILE_KEY,
  PROFILE_RECORD_PREFIX,
  LOCAL_MESSAGE_QUEUE_KEY,
  WEBAUTHN_VAULT_PREFIX,
  SESSION_RESUME_KEY,
  LEGACY_KEYS,
  PBKDF2_ROUNDS,
  SESSION_RESUME_TTL_MS,
  SESSION_RESUME_REFRESH_MS,
  ZERO_HASH,
  DEFAULT_PROFILE_PREFERENCES,
} from "./auth-types"

export type {
  AuthState,
  NodeProvisionInput,
  StoredNodeRegistration,
  ContactRecord,
  ProfilePreferences,
  WebAuthnBinding,
  OAuthAppId,
  OAuthProfileSecret,
  OAuthProfileSecretInput,
  GmailProfileSecret,
  GoogleDriveProfileSecret,
  GitHubProfileSecret,
  CloudflareProfileSecret,
  ProfileEvent,
  SealedNestedContainer,
  RoutedSealedEnvelope,
  UnlockedProfileContainer,
  SealedProfileContainer,
  ProfileSummary,
  LocalQueuedMessage,
  NodeRelayPublishResult,
} from "./auth-types"

const NODE_RELAY_DOMAIN = "nodes.edgerun.tech"
const textEncoder = new TextEncoder()
const textDecoder = new TextDecoder()
let sessionResumeTimer: ReturnType<typeof setInterval> | null = null

const APP_SESSION_COOKIE_NAMES = [
  "gmail_access_token",
  "gmail_refresh_token",
  "gmail_email",
  "gmail_profile_pending",
  "gmail_oauth_state",
  "google_drive_access_token",
  "google_drive_refresh_token",
  "google_drive_email",
  "google_drive_profile_pending",
  "google_drive_oauth_state",
  "github_access_token",
  "github_login",
  "github_profile_pending",
  "github_oauth_state",
  "cloudflare_api_token",
  "cloudflare_label",
  "cloudflare_account_id",
  "cloudflare_token_id",
  "cloudflare_zone_id",
  "cloudflare_zone_name",
] as const

const APP_SESSION_STORAGE_KEYS = [
  "edgerun:oauth-pending:gmail",
  "edgerun:oauth-pending:google-drive",
  "edgerun:oauth-pending:github",
] as const

function clearTransientAppSessions() {
  if (typeof document !== "undefined") {
    for (const name of APP_SESSION_COOKIE_NAMES) {
      document.cookie = `${name}=; Max-Age=0; path=/`
    }
  }
  if (typeof sessionStorage !== "undefined") {
    for (const key of APP_SESSION_STORAGE_KEYS) sessionStorage.removeItem(key)
  }
  if (typeof navigator !== "undefined" && "serviceWorker" in navigator) {
    navigator.serviceWorker.controller?.postMessage({ type: "CLEAR_APP_SESSIONS" })
  }
  if (typeof fetch !== "undefined") {
    void fetch("/api/session/clear", { method: "POST", keepalive: true }).catch(() => undefined)
  }
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = ""
  for (let offset = 0; offset < bytes.length; offset += 0x8000) {
    binary += String.fromCharCode(...bytes.slice(offset, offset + 0x8000))
  }
  return btoa(binary)
}

function base64ToBytes(base64: string): Uint8Array {
  const binary = atob(base64)
  const bytes = new Uint8Array(binary.length)
  for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index)
  return bytes
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("")
}

function profileInitials(handle: string): string {
  const initials = handle.trim().split(/\s+/).map((part) => part[0]).join("").slice(0, 2).toUpperCase()
  return initials || "ID"
}

function profileColor(seed: string): string {
  const hue = seed.split("").reduce((acc, char) => acc + char.charCodeAt(0), 0) % 360
  return `oklch(0.3 0.1 ${hue})`
}

function sanitizeProfilePreferences(value: unknown, handle = ""): ProfilePreferences {
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === "object" && !Array.isArray(value))
}

function isIsoDate(value: unknown): value is string {
  return typeof value === "string" && value.length > 0 && Number.isFinite(Date.parse(value))
}

function isHex(value: unknown, length?: number): value is string {
  return typeof value === "string" && /^[a-f0-9]+$/i.test(value) && (!length || value.length === length)
}

function validBase64(value: unknown): value is string {
  if (typeof value !== "string" || !value) return false
  try {
    base64ToBytes(value)
    return true
  } catch {
    return false
  }
}

function normalizeOAuthSecret(secret: unknown): OAuthProfileSecret | null {
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

function validateOAuthSecretInput(appId: OAuthAppId, secret: OAuthProfileSecretInput) {
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

function normalizeUnlockedProfile(profile: UnlockedProfileContainer): UnlockedProfileContainer {
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

function deriveProfilePreferences(events: ProfileEvent[], fallback: unknown, handle: string): ProfilePreferences {
  let preferences = sanitizeProfilePreferences(fallback, handle)
  for (const event of events) {
    if (event.kind === "PROFILE_SETTINGS_UPDATED") {
      preferences = sanitizeProfilePreferences(event.payload.preferences, handle)
    }
  }
  return preferences
}

function deriveWebAuthnBinding(events: ProfileEvent[], fallback?: WebAuthnBinding): WebAuthnBinding | undefined {
  let binding = fallback
  for (const event of events) {
    if (event.kind === "PROFILE_WEBAUTHN_BOUND") binding = event.payload
  }
  return binding
}

function canonicalJson(value: unknown): string {
  if (value === null || typeof value !== "object") return JSON.stringify(value)
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`
  const record = value as Record<string, unknown>
  return `{${Object.keys(record).sort().map((key) => `${JSON.stringify(key)}:${canonicalJson(record[key])}`).join(",")}}`
}

async function sha256Hex(bytes: Uint8Array | string): Promise<string> {
  const data = typeof bytes === "string" ? textEncoder.encode(bytes) : bytes
  return bytesToHex(new Uint8Array(await crypto.subtle.digest("SHA-256", data)))
}

async function deriveAesKey(password: string, salt: Uint8Array, rounds: number): Promise<CryptoKey> {
  const baseKey = await crypto.subtle.importKey("raw", textEncoder.encode(password), "PBKDF2", false, ["deriveKey"])
  return crypto.subtle.deriveKey(
    { name: "PBKDF2", salt, iterations: rounds, hash: "SHA-256" },
    baseKey,
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt", "decrypt"],
  )
}

async function importAesGcmKey(raw: BufferSource): Promise<CryptoKey> {
  return crypto.subtle.importKey("raw", raw, { name: "AES-GCM", length: 256 }, false, ["encrypt", "decrypt"])
}

type SessionResumeTicket = {
  version: 1
  profileId: string
  expiresAtIso: string
  handle: string
}

type SessionResumeWorkerResponse = {
  ok: boolean
  handle?: string
  profileId?: string
  expiresAtIso?: string
  profileJson?: string
  error?: string
}

async function sessionResumeWorker(): Promise<ServiceWorker | null> {
  if (typeof window === "undefined" || !("serviceWorker" in navigator)) return null
  try {
    const registration = await navigator.serviceWorker.register("/session-resume-sw.js", { scope: "/" })
    const ready = await navigator.serviceWorker.ready
    return ready.active ?? registration.active ?? null
  } catch {
    return null
  }
}

async function postSessionResumeWorker(message: Record<string, unknown>): Promise<SessionResumeWorkerResponse | null> {
  const worker = await sessionResumeWorker()
  if (!worker) return null
  return new Promise((resolve) => {
    const channel = new MessageChannel()
    const timeout = window.setTimeout(() => {
      channel.port1.close()
      resolve(null)
    }, 1500)
    channel.port1.onmessage = (event: MessageEvent<SessionResumeWorkerResponse>) => {
      window.clearTimeout(timeout)
      channel.port1.close()
      resolve(event.data)
    }
    worker.postMessage(message, [channel.port2])
  })
}

function clearSessionResumeTicket() {
  if (typeof window === "undefined") return
  if (sessionResumeTimer) {
    clearInterval(sessionResumeTimer)
    sessionResumeTimer = null
  }
  const raw = sessionStorage.getItem(SESSION_RESUME_KEY)
  if (raw) {
    try {
      const ticket = JSON.parse(raw) as SessionResumeTicket
      if (ticket.handle) void postSessionResumeWorker({ type: "CLEAR_SESSION_RESUME", handle: ticket.handle })
    } catch {
      // Ignore malformed local handle records.
    }
  }
  sessionStorage.removeItem(SESSION_RESUME_KEY)
}

async function writeSessionResumeTicket(profile: UnlockedProfileContainer) {
  if (typeof window === "undefined") return
  let previousHandle: string | undefined
  const existing = sessionStorage.getItem(SESSION_RESUME_KEY)
  if (existing) {
    try {
      previousHandle = (JSON.parse(existing) as SessionResumeTicket).handle
    } catch {
      previousHandle = undefined
    }
  }
  const response = await postSessionResumeWorker({
    type: "STORE_SESSION_RESUME",
    profileId: profile.ownerEncryption.identityIdHex,
    profileJson: canonicalJson(profile),
    ttlMs: SESSION_RESUME_TTL_MS,
    previousHandle,
  })
  if (!response?.ok || !response.handle || !response.expiresAtIso) {
    sessionStorage.removeItem(SESSION_RESUME_KEY)
    return
  }
  const ticket: SessionResumeTicket = {
    version: 1,
    profileId: profile.ownerEncryption.identityIdHex,
    expiresAtIso: response.expiresAtIso,
    handle: response.handle,
  }
  sessionStorage.setItem(SESSION_RESUME_KEY, JSON.stringify(ticket))
}

function startSessionResumeHeartbeat(profile: UnlockedProfileContainer) {
  if (typeof window === "undefined") return
  if (sessionResumeTimer) clearInterval(sessionResumeTimer)
  void writeSessionResumeTicket(profile)
  sessionResumeTimer = setInterval(() => {
    const current = authStore.get()
    if (current.authState !== "authenticated" || !current.unlockedProfile) {
      clearSessionResumeTicket()
      return
    }
    void writeSessionResumeTicket(current.unlockedProfile)
  }, SESSION_RESUME_REFRESH_MS)
}

async function consumeSessionResumeTicket(): Promise<UnlockedProfileContainer | null> {
  if (typeof window === "undefined") return null
  const raw = sessionStorage.getItem(SESSION_RESUME_KEY)
  if (!raw) return null
  sessionStorage.removeItem(SESSION_RESUME_KEY)
  try {
    const ticket = JSON.parse(raw) as SessionResumeTicket
    if (ticket.version !== 1 || !ticket.profileId || !ticket.handle || Date.parse(ticket.expiresAtIso) <= Date.now()) return null
    const response = await postSessionResumeWorker({ type: "CONSUME_SESSION_RESUME", handle: ticket.handle })
    if (!response?.ok || response.profileId !== ticket.profileId || !response.profileJson) return null
    const profile = JSON.parse(response.profileJson) as UnlockedProfileContainer
    if (profile.ownerEncryption.identityIdHex !== ticket.profileId) return null
    const sealed = readSealedProfile()
    if (!sealed || profileIdFor(sealed) !== ticket.profileId) return null
    await importP256PrivateKey(profile.owner)
    await importP256PrivateKey(profile.browserNode)
    return normalizeUnlockedProfile({
      ...profile,
      nodes: profile.nodes ?? [profile.browserNode],
      contacts: profile.contacts ?? [selfContact(profile)],
      eventLog: profile.eventLog ?? [],
      profilePreferences: deriveProfilePreferences(profile.eventLog ?? [], profile.profilePreferences, profile.handle),
      webAuthnBinding: deriveWebAuthnBinding(profile.eventLog ?? [], profile.webAuthnBinding),
      appSecrets: profile.appSecrets ?? [],
      sealedContainers: profile.sealedContainers ?? [],
      outbox: profile.outbox ?? [],
    })
  } catch {
    return null
  }
}

async function generateP256Identity(): Promise<{ keyPair: CryptoKeyPair; material: P256KeyMaterial }> {
  const keyPair = await crypto.subtle.generateKey(
    { name: "ECDSA", namedCurve: "P-256" },
    true,
    ["sign", "verify"],
  )
  const publicRaw = new Uint8Array(await crypto.subtle.exportKey("raw", keyPair.publicKey))
  const privatePkcs8 = new Uint8Array(await crypto.subtle.exportKey("pkcs8", keyPair.privateKey))
  const identityBytes = publicRaw[0] === 0x04 && publicRaw.length === 65 ? publicRaw.slice(1) : publicRaw
  return {
    keyPair,
    material: {
      algorithm: "ECDSA_P256_SHA256",
      identityIdHex: bytesToHex(identityBytes),
      publicKeyRawBase64: bytesToBase64(publicRaw),
      privateKeyPkcs8Base64: bytesToBase64(privatePkcs8),
    },
  }
}

async function generateP256EncryptionIdentity(): Promise<{ keyPair: CryptoKeyPair; material: P256EncryptionKeyMaterial }> {
  const keyPair = await crypto.subtle.generateKey(
    { name: "ECDH", namedCurve: "P-256" },
    true,
    ["deriveKey"],
  )
  const publicRaw = new Uint8Array(await crypto.subtle.exportKey("raw", keyPair.publicKey))
  const privatePkcs8 = new Uint8Array(await crypto.subtle.exportKey("pkcs8", keyPair.privateKey))
  const identityBytes = publicRaw[0] === 0x04 && publicRaw.length === 65 ? publicRaw.slice(1) : publicRaw
  return {
    keyPair,
    material: {
      algorithm: "ECDH_P256_HKDF_SHA256",
      identityIdHex: bytesToHex(identityBytes),
      publicKeyRawBase64: bytesToBase64(publicRaw),
      privateKeyPkcs8Base64: bytesToBase64(privatePkcs8),
    },
  }
}

async function importP256PrivateKey(material: P256KeyMaterial): Promise<CryptoKey> {
  return crypto.subtle.importKey(
    "pkcs8",
    base64ToBytes(material.privateKeyPkcs8Base64),
    { name: "ECDSA", namedCurve: "P-256" },
    false,
    ["sign"],
  )
}

async function importP256EcdhPublicKey(publicKeyRawBase64: string): Promise<CryptoKey> {
  return crypto.subtle.importKey(
    "raw",
    base64ToBytes(publicKeyRawBase64),
    { name: "ECDH", namedCurve: "P-256" },
    false,
    [],
  )
}

async function importP256EcdhPrivateKey(material: P256EncryptionKeyMaterial): Promise<CryptoKey> {
  return crypto.subtle.importKey(
    "pkcs8",
    base64ToBytes(material.privateKeyPkcs8Base64),
    { name: "ECDH", namedCurve: "P-256" },
    false,
    ["deriveKey"],
  )
}

async function sealToRecipient(
  plaintext: Uint8Array,
  recipientPublicKeyRawBase64: string,
): Promise<{ ciphertext: Uint8Array; iv: Uint8Array; ephemeralPublicKeyRawBase64: string }> {
  const recipientPublicKey = await importP256EcdhPublicKey(recipientPublicKeyRawBase64)
  const ephemeral = await crypto.subtle.generateKey(
    { name: "ECDH", namedCurve: "P-256" },
    true,
    ["deriveKey"],
  )
  const key = await crypto.subtle.deriveKey(
    { name: "ECDH", public: recipientPublicKey },
    ephemeral.privateKey,
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt"],
  )
  const iv = crypto.getRandomValues(new Uint8Array(12))
  const ciphertext = new Uint8Array(await crypto.subtle.encrypt({ name: "AES-GCM", iv }, key, plaintext))
  const ephemeralPublicKeyRawBase64 = bytesToBase64(new Uint8Array(await crypto.subtle.exportKey("raw", ephemeral.publicKey)))
  return { ciphertext, iv, ephemeralPublicKeyRawBase64 }
}

async function openSealedNestedContainer(container: SealedNestedContainer, recipient: P256EncryptionKeyMaterial): Promise<string> {
  const privateKey = await importP256EcdhPrivateKey(recipient)
  const ephemeralPublicKey = await importP256EcdhPublicKey(container.seal.ephemeralPublicKeyRawBase64)
  const key = await crypto.subtle.deriveKey(
    { name: "ECDH", public: ephemeralPublicKey },
    privateKey,
    { name: "AES-GCM", length: 256 },
    false,
    ["decrypt"],
  )
  const plaintext = await crypto.subtle.decrypt(
    { name: "AES-GCM", iv: base64ToBytes(container.ivBase64) },
    key,
    base64ToBytes(container.ciphertextBase64),
  )
  const text = textDecoder.decode(plaintext)
  const digest = await sha256Hex(new Uint8Array(plaintext))
  if (digest !== container.plaintextSha256) throw new Error("Sealed message digest check failed.")
  return text
}

function selfContact(profile: Pick<UnlockedProfileContainer, "handle" | "ownerEncryption">): ContactRecord {
  return {
    id: profile.ownerEncryption.identityIdHex,
    label: `${profile.handle} self`,
    publicKeyRawBase64: profile.ownerEncryption.publicKeyRawBase64,
    identityIdHex: profile.ownerEncryption.identityIdHex,
    routeHint: "local-profile",
    addedAtIso: new Date().toISOString(),
  }
}

async function createGenesisEvent(node: P256KeyMaterial, nodePrivateKey: CryptoKey, ownerId: string, label?: string): Promise<ProfileEvent> {
  const payload = {
    nodeId: node.identityIdHex,
    initialControllers: [ownerId],
    createdAtIso: new Date().toISOString(),
    ...(label ? { label } : {}),
  }
  const payloadSha256 = await sha256Hex(canonicalJson(payload))
  const unsigned = {
    seq: 0,
    kind: "NODE_GENESIS" as const,
    previousEventHash: ZERO_HASH,
    payloadSha256,
    payload,
  }
  const signature = new Uint8Array(await crypto.subtle.sign(
    { name: "ECDSA", hash: "SHA-256" },
    nodePrivateKey,
    textEncoder.encode(canonicalJson(unsigned)),
  ))
  const eventHash = await sha256Hex(canonicalJson({ ...unsigned, signatureBase64: bytesToBase64(signature) }))
  return { ...unsigned, signatureBase64: bytesToBase64(signature), eventHash }
}

async function createProfileSettingsEvent(profile: UnlockedProfileContainer, nodePrivateKey: CryptoKey, preferences: ProfilePreferences): Promise<ProfileEvent> {
  const previous = profile.eventLog[profile.eventLog.length - 1]
  const payload = {
    preferences,
    updatedAtIso: new Date().toISOString(),
  }
  const payloadSha256 = await sha256Hex(canonicalJson(payload))
  const unsigned = {
    seq: profile.eventLog.length,
    kind: "PROFILE_SETTINGS_UPDATED" as const,
    previousEventHash: previous?.eventHash ?? ZERO_HASH,
    payloadSha256,
    payload,
  }
  const signature = new Uint8Array(await crypto.subtle.sign(
    { name: "ECDSA", hash: "SHA-256" },
    nodePrivateKey,
    textEncoder.encode(canonicalJson(unsigned)),
  ))
  const eventHash = await sha256Hex(canonicalJson({ ...unsigned, signatureBase64: bytesToBase64(signature) }))
  return { ...unsigned, signatureBase64: bytesToBase64(signature), eventHash }
}

async function createWebAuthnBoundEvent(profile: UnlockedProfileContainer, nodePrivateKey: CryptoKey, binding: WebAuthnBinding): Promise<ProfileEvent> {
  const previous = profile.eventLog[profile.eventLog.length - 1]
  const payloadSha256 = await sha256Hex(canonicalJson(binding))
  const unsigned = {
    seq: profile.eventLog.length,
    kind: "PROFILE_WEBAUTHN_BOUND" as const,
    previousEventHash: previous?.eventHash ?? ZERO_HASH,
    payloadSha256,
    payload: binding,
  }
  const signature = new Uint8Array(await crypto.subtle.sign(
    { name: "ECDSA", hash: "SHA-256" },
    nodePrivateKey,
    textEncoder.encode(canonicalJson(unsigned)),
  ))
  const eventHash = await sha256Hex(canonicalJson({ ...unsigned, signatureBase64: bytesToBase64(signature) }))
  return { ...unsigned, signatureBase64: bytesToBase64(signature), eventHash }
}

async function sealProfile(profile: UnlockedProfileContainer, password: string): Promise<SealedProfileContainer> {
  if (password.length < 8) throw new Error("Password must be at least 8 characters.")
  const salt = crypto.getRandomValues(new Uint8Array(16))
  const iv = crypto.getRandomValues(new Uint8Array(12))
  const key = await deriveAesKey(password, salt, PBKDF2_ROUNDS)
  const ciphertext = new Uint8Array(await crypto.subtle.encrypt(
    { name: "AES-GCM", iv },
    key,
    textEncoder.encode(canonicalJson(profile)),
  ))
  return {
    version: 1,
    kind: "edgerun.browser-profile.sealed",
    cipher: "AES-GCM-256",
    kdf: { name: "PBKDF2-HMAC-SHA256", rounds: PBKDF2_ROUNDS, saltBase64: bytesToBase64(salt) },
    ivBase64: bytesToBase64(iv),
    ciphertextBase64: bytesToBase64(ciphertext),
    profileId: profile.ownerEncryption.identityIdHex,
    handleHint: profile.handle,
    ownerIdHint: profile.owner.identityIdHex,
    encryptionIdHint: profile.ownerEncryption.identityIdHex,
    encryptionPublicKeyHint: profile.ownerEncryption.publicKeyRawBase64,
    webAuthnCredentialIdHint: profile.webAuthnBinding?.credentialIdBase64,
    nodeIdHint: profile.browserNode.identityIdHex,
    createdAtIso: profile.createdAtIso,
  }
}

async function openProfile(sealed: SealedProfileContainer, password: string): Promise<UnlockedProfileContainer> {
  const key = await deriveAesKey(password, base64ToBytes(sealed.kdf.saltBase64), sealed.kdf.rounds)
  const plaintext = await crypto.subtle.decrypt(
    { name: "AES-GCM", iv: base64ToBytes(sealed.ivBase64) },
    key,
    base64ToBytes(sealed.ciphertextBase64),
  )
  const parsed = JSON.parse(textDecoder.decode(plaintext)) as UnlockedProfileContainer
  if (parsed.version !== 1 || parsed.owner.identityIdHex !== sealed.ownerIdHint || parsed.browserNode.identityIdHex !== sealed.nodeIdHint) {
    throw new Error("Profile container integrity check failed.")
  }
  const ownerEncryption = parsed.ownerEncryption ?? (await generateP256EncryptionIdentity()).material
  const eventLog = parsed.eventLog ?? []
  const profilePreferences = deriveProfilePreferences(eventLog, parsed.profilePreferences, parsed.handle)
  const webAuthnBinding = deriveWebAuthnBinding(eventLog, parsed.webAuthnBinding)
  return normalizeUnlockedProfile({
    ...parsed,
    ownerEncryption,
    nodes: parsed.nodes ?? [parsed.browserNode],
    contacts: parsed.contacts ?? [selfContact({ handle: parsed.handle, ownerEncryption })],
    eventLog,
    profilePreferences,
    webAuthnBinding,
    appSecrets: parsed.appSecrets ?? [],
    sealedContainers: parsed.sealedContainers ?? [],
    outbox: parsed.outbox ?? [],
  })
}

function profileIdFor(sealed: SealedProfileContainer): string {
  return sealed.profileId ?? sealed.encryptionIdHint ?? sealed.ownerIdHint
}

function summaryFor(sealed: SealedProfileContainer): ProfileSummary {
  const profileId = profileIdFor(sealed)
  return {
    profileId,
    handle: sealed.handleHint ?? shortProfileId(sealed.ownerIdHint),
    ownerIdHint: sealed.ownerIdHint,
    encryptionIdHint: sealed.encryptionIdHint ?? profileId,
    encryptionPublicKeyHint: sealed.encryptionPublicKeyHint ?? "",
    webAuthnCredentialIdHint: sealed.webAuthnCredentialIdHint,
    webAuthnUnlockAvailable: webAuthnVaultExists(profileId, sealed.webAuthnCredentialIdHint),
    nodeIdHint: sealed.nodeIdHint,
    createdAtIso: sealed.createdAtIso,
  }
}

function shortProfileId(value: string): string {
  return value ? `profile-${value.slice(0, 8)}` : "profile"
}

function readProfileIndex(): ProfileSummary[] {
  if (typeof window === "undefined") return []
  const raw = localStorage.getItem(PROFILE_INDEX_KEY)
  if (!raw) return []
  try {
    const parsed = JSON.parse(raw) as ProfileSummary[]
    return Array.isArray(parsed) ? parsed.filter((item) => item.profileId) : []
  } catch {
    localStorage.removeItem(PROFILE_INDEX_KEY)
    return []
  }
}

function writeProfileIndex(summaries: ProfileSummary[]) {
  localStorage.setItem(PROFILE_INDEX_KEY, JSON.stringify(summaries))
}

function readProfileById(profileId: string): SealedProfileContainer | null {
  if (typeof window === "undefined") return null
  const raw = localStorage.getItem(`${PROFILE_RECORD_PREFIX}${profileId}`)
  if (!raw) return null
  try {
    const parsed = JSON.parse(raw) as SealedProfileContainer
    if (parsed.kind === "edgerun.browser-profile.sealed" && parsed.version === 1) return parsed
  } catch {
    localStorage.removeItem(`${PROFILE_RECORD_PREFIX}${profileId}`)
  }
  return null
}

function migrateLegacyProfile(): SealedProfileContainer | null {
  if (typeof window === "undefined") return null
  const raw = localStorage.getItem(LEGACY_PROFILE_STORAGE_KEY)
  if (!raw) return null
  try {
    const parsed = JSON.parse(raw) as SealedProfileContainer
    if (parsed.kind === "edgerun.browser-profile.sealed" && parsed.version === 1) {
      persistSealedProfile(parsed)
      localStorage.removeItem(LEGACY_PROFILE_STORAGE_KEY)
      return readSealedProfile()
    }
  } catch {
    localStorage.removeItem(LEGACY_PROFILE_STORAGE_KEY)
  }
  return null
}

function readSealedProfile(): SealedProfileContainer | null {
  if (typeof window === "undefined") return null
  const index = readProfileIndex()
  const activeProfileId = localStorage.getItem(ACTIVE_PROFILE_KEY) ?? index[0]?.profileId ?? null
  if (activeProfileId) {
    const active = readProfileById(activeProfileId)
    if (active) return active
  }
  return migrateLegacyProfile()
}

function persistSealedProfile(sealed: SealedProfileContainer) {
  const profileId = profileIdFor(sealed)
  const normalized: SealedProfileContainer = {
    ...sealed,
    profileId,
    handleHint: sealed.handleHint ?? shortProfileId(sealed.ownerIdHint),
    encryptionIdHint: sealed.encryptionIdHint ?? profileId,
    encryptionPublicKeyHint: sealed.encryptionPublicKeyHint ?? "",
    webAuthnCredentialIdHint: sealed.webAuthnCredentialIdHint,
  }
  localStorage.setItem(`${PROFILE_RECORD_PREFIX}${profileId}`, JSON.stringify(normalized))
  const nextSummary = summaryFor(normalized)
  const existing = readProfileIndex().filter((item) => item.profileId !== profileId)
  writeProfileIndex([...existing, nextSummary].sort((left, right) => left.createdAtIso.localeCompare(right.createdAtIso)))
  localStorage.setItem(ACTIVE_PROFILE_KEY, profileId)
  for (const key of LEGACY_KEYS) localStorage.removeItem(key)
}

function activeProfileId(): string | null {
  if (typeof window === "undefined") return null
  return localStorage.getItem(ACTIVE_PROFILE_KEY) ?? readProfileIndex()[0]?.profileId ?? null
}

function readLocalMessageQueue(): LocalQueuedMessage[] {
  if (typeof window === "undefined") return []
  const raw = localStorage.getItem(LOCAL_MESSAGE_QUEUE_KEY)
  if (!raw) return []
  try {
    const parsed = JSON.parse(raw) as LocalQueuedMessage[]
    return Array.isArray(parsed) ? parsed.filter((item) => item.version === 1 && item.sealedContainer) : []
  } catch {
    localStorage.removeItem(LOCAL_MESSAGE_QUEUE_KEY)
    return []
  }
}

function writeLocalMessageQueue(messages: LocalQueuedMessage[]) {
  localStorage.setItem(LOCAL_MESSAGE_QUEUE_KEY, JSON.stringify(messages))
}

function localMessagesFor(profile: UnlockedProfileContainer | null): LocalQueuedMessage[] {
  if (!profile) return []
  return readLocalMessageQueue().filter((message) => message.toId === profile.ownerEncryption.identityIdHex)
}

function webAuthnAvailable(): boolean {
  return typeof window !== "undefined" && "PublicKeyCredential" in window && Boolean(navigator.credentials)
}

function webAuthnVaultExists(profileId: string, credentialIdBase64?: string): boolean {
  if (typeof window === "undefined" || !credentialIdBase64) return false
  try {
    const raw = localStorage.getItem(`${WEBAUTHN_VAULT_PREFIX}${profileId}`)
    if (!raw) return false
    const parsed = JSON.parse(raw) as { version?: number; credentialIdBase64?: string }
    return parsed.version === 1 && parsed.credentialIdBase64 === credentialIdBase64
  } catch {
    localStorage.removeItem(`${WEBAUTHN_VAULT_PREFIX}${profileId}`)
    return false
  }
}

function hasUsableWebAuthnBinding(sealed: SealedProfileContainer | null): boolean {
  if (!sealed || !sealed.webAuthnCredentialIdHint) return false
  return webAuthnVaultExists(profileIdFor(sealed), sealed.webAuthnCredentialIdHint)
}

async function webAuthnPrfSalt(profileId: string): Promise<Uint8Array> {
  return new Uint8Array(await crypto.subtle.digest("SHA-256", textEncoder.encode(`edgerun:profile-unlock:${profileId}`)))
}

async function credentialCreationOptions(profile: UnlockedProfileContainer): Promise<PublicKeyCredentialCreationOptions> {
  const profileId = profile.ownerEncryption.identityIdHex
  return {
    challenge: crypto.getRandomValues(new Uint8Array(32)),
    rp: { name: "Edgerun" },
    user: {
      id: base64ToBytes(profile.ownerEncryption.publicKeyRawBase64).slice(0, 32),
      name: profile.handle,
      displayName: profile.handle,
    },
    pubKeyCredParams: [
      { type: "public-key", alg: -7 },
      { type: "public-key", alg: -257 },
    ],
    authenticatorSelection: {
      residentKey: "preferred",
      userVerification: "required",
    },
    timeout: 60_000,
    extensions: {
      prf: { eval: { first: await webAuthnPrfSalt(profileId) } },
    } as AuthenticationExtensionsClientInputs,
  }
}

function webAuthnPrfResult(credential: PublicKeyCredential | null): Uint8Array | null {
  const results = credential?.getClientExtensionResults() as { prf?: { enabled?: boolean; results?: { first?: ArrayBuffer } } }
  const first = results.prf?.results?.first
  return first ? new Uint8Array(first) : null
}

async function requestWebAuthnPrf(credentialIdBase64: string, profileId: string): Promise<Uint8Array> {
  const assertion = await navigator.credentials.get({
    publicKey: {
      challenge: crypto.getRandomValues(new Uint8Array(32)),
      allowCredentials: [{ type: "public-key", id: base64ToBytes(credentialIdBase64) }],
      userVerification: "required",
      timeout: 60_000,
      extensions: {
        prf: { eval: { first: await webAuthnPrfSalt(profileId) } },
      } as AuthenticationExtensionsClientInputs,
    },
  }) as PublicKeyCredential | null
  const first = webAuthnPrfResult(assertion)
  if (!first) throw new Error("This passkey cannot unlock the profile because WebAuthn PRF/hmac-secret was not returned. Try a YubiKey with FIDO2 hmac-secret enabled, or use the profile password.")
  return first
}

async function writeWebAuthnVault(profileId: string, credentialIdBase64: string, profilePassword: string, prfSecret: Uint8Array) {
  const key = await importAesGcmKey(prfSecret)
  const iv = crypto.getRandomValues(new Uint8Array(12))
  const ciphertext = new Uint8Array(await crypto.subtle.encrypt({ name: "AES-GCM", iv }, key, textEncoder.encode(profilePassword)))
  localStorage.setItem(`${WEBAUTHN_VAULT_PREFIX}${profileId}`, JSON.stringify({
    version: 1,
    credentialIdBase64,
    ivBase64: bytesToBase64(iv),
    ciphertextBase64: bytesToBase64(ciphertext),
  }))
}

async function readWebAuthnVaultPassword(profileId: string, credentialIdBase64: string, prfSecret: Uint8Array): Promise<string> {
  const raw = localStorage.getItem(`${WEBAUTHN_VAULT_PREFIX}${profileId}`)
  if (!raw) throw new Error("No local passkey unlock vault is bound for this profile.")
  const parsed = JSON.parse(raw) as { version: 1; credentialIdBase64: string; ivBase64: string; ciphertextBase64: string }
  if (parsed.version !== 1 || parsed.credentialIdBase64 !== credentialIdBase64) throw new Error("Passkey vault does not match this profile.")
  const key = await importAesGcmKey(prfSecret)
  const plaintext = await crypto.subtle.decrypt(
    { name: "AES-GCM", iv: base64ToBytes(parsed.ivBase64) },
    key,
    base64ToBytes(parsed.ciphertextBase64),
  )
  return textDecoder.decode(plaintext)
}

function registrationFor(profile: UnlockedProfileContainer): StoredNodeRegistration {
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

const initialSealedProfile = typeof window !== "undefined" ? readSealedProfile() : null
const initialProfileSummaries = typeof window !== "undefined" ? readProfileIndex() : []
const initialActiveProfileId = typeof window !== "undefined" ? activeProfileId() : null

export const authStore = atom<AuthStore>({
  authState: initialSealedProfile ? "locked" : "unauthenticated",
  username: "",
  isLoading: false,
  error: null,
  webAuthnAvailable: webAuthnAvailable(),
  nodeRegistration: null,
  unlockedProfile: null,
  sealedProfile: initialSealedProfile,
  profileSummaries: initialProfileSummaries,
  activeProfileId: initialActiveProfileId,
  localMessages: [],
})

if (typeof window !== "undefined" && initialSealedProfile) {
  queueMicrotask(() => {
    if (authStore.get().authState !== "authenticated") clearTransientAppSessions()
  })
}

export const isAuthenticatedStore = computed(authStore, (s) => s.authState === "authenticated")
export const isGuestStore = computed(authStore, () => false)

export function hasRegistered(): boolean {
  return readProfileIndex().length > 0 || readSealedProfile() !== null
}

export async function registerAuth(name: string, nodeProvision?: NodeProvisionInput): Promise<boolean> {
  const state = authStore.get()
  if (state.isLoading) return false
  authStore.set({ ...state, authState: "registering", isLoading: true, error: null })

  try {
    const password = nodeProvision?.passphrase ?? ""
    const owner = await generateP256Identity()
    const ownerEncryption = await generateP256EncryptionIdentity()
    const browserNode = await generateP256Identity()
    const genesis = await createGenesisEvent(browserNode.material, browserNode.keyPair.privateKey, owner.material.identityIdHex)
    const profile: UnlockedProfileContainer = {
      version: 1,
      handle: name.trim(),
      createdAtIso: new Date().toISOString(),
      owner: owner.material,
      ownerEncryption: ownerEncryption.material,
      browserNode: browserNode.material,
      nodes: [browserNode.material],
      contacts: [],
      eventLog: [genesis],
      profilePreferences: sanitizeProfilePreferences(undefined, name.trim()),
      appSecrets: [],
      sealedContainers: [],
      outbox: [],
    }
    profile.contacts = [selfContact(profile)]
    const sealed = await sealProfile(profile, password)
    persistSealedProfile(sealed)
    startSessionResumeHeartbeat(profile)
    authStore.set({
      authState: "authenticated",
      username: profile.handle,
      isLoading: false,
      error: null,
      webAuthnAvailable: webAuthnAvailable(),
      nodeRegistration: registrationFor(profile),
      unlockedProfile: profile,
      sealedProfile: sealed,
      profileSummaries: readProfileIndex(),
      activeProfileId: profileIdFor(sealed),
      localMessages: localMessagesFor(profile),
    })
    return true
  } catch (err) {
    authStore.set({ ...authStore.get(), authState: "unauthenticated", isLoading: false, error: err instanceof Error ? err.message : "Profile creation failed" })
    return false
  }
}

export async function authenticateAuth(password?: string): Promise<boolean> {
  const state = authStore.get()
  if (state.isLoading) return false
  const sealed = readSealedProfile()
  if (!sealed) {
    authStore.set({ ...state, authState: "unauthenticated", isLoading: false, error: "No sealed profile container found." })
    return false
  }
  authStore.set({ ...state, authState: "authenticating", isLoading: true, error: null, sealedProfile: sealed })

  try {
    const profile = await openProfile(sealed, password ?? "")
    await importP256PrivateKey(profile.owner)
    await importP256PrivateKey(profile.browserNode)
    startSessionResumeHeartbeat(profile)
    authStore.set({
      authState: "authenticated",
      username: profile.handle,
      isLoading: false,
      error: null,
      webAuthnAvailable: webAuthnAvailable(),
      nodeRegistration: registrationFor(profile),
      unlockedProfile: profile,
      sealedProfile: sealed,
      profileSummaries: readProfileIndex(),
      activeProfileId: profileIdFor(sealed),
      localMessages: localMessagesFor(profile),
    })
    return true
  } catch {
    authStore.set({ ...authStore.get(), authState: "locked", isLoading: false, error: "Profile unlock failed." })
    return false
  }
}

export async function authenticateWithWebAuthn(): Promise<boolean> {
  const state = authStore.get()
  if (state.isLoading) return false
  const sealed = readSealedProfile()
  const credentialIdBase64 = sealed?.webAuthnCredentialIdHint
  if (!sealed || !credentialIdBase64 || !hasUsableWebAuthnBinding(sealed)) {
    authStore.set({ ...state, error: "No usable passkey unlock vault is bound to this profile." })
    return false
  }
  authStore.set({ ...state, authState: "authenticating", isLoading: true, error: null, sealedProfile: sealed })
  try {
    const profileId = profileIdFor(sealed)
    const prfSecret = await requestWebAuthnPrf(credentialIdBase64, profileId)
    const password = await readWebAuthnVaultPassword(profileId, credentialIdBase64, prfSecret)
    const profile = await openProfile(sealed, password)
    await importP256PrivateKey(profile.owner)
    await importP256PrivateKey(profile.browserNode)
    startSessionResumeHeartbeat(profile)
    authStore.set({
      authState: "authenticated",
      username: profile.handle,
      isLoading: false,
      error: null,
      webAuthnAvailable: webAuthnAvailable(),
      nodeRegistration: registrationFor(profile),
      unlockedProfile: profile,
      sealedProfile: sealed,
      profileSummaries: readProfileIndex(),
      activeProfileId: profileIdFor(sealed),
      localMessages: localMessagesFor(profile),
    })
    return true
  } catch (err) {
    authStore.set({ ...authStore.get(), authState: "locked", isLoading: false, error: err instanceof Error ? err.message : "Passkey unlock failed." })
    return false
  }
}

export async function resumeSessionAuth(): Promise<boolean> {
  const state = authStore.get()
  if (state.isLoading || state.authState === "authenticated") return false
  const sealed = readSealedProfile()
  if (!sealed) return false
  authStore.set({ ...state, authState: "authenticating", isLoading: true, error: null, sealedProfile: sealed })
  try {
    const profile = await consumeSessionResumeTicket()
    if (!profile) {
      authStore.set({ ...authStore.get(), authState: "locked", isLoading: false, error: null, sealedProfile: sealed })
      return false
    }
    startSessionResumeHeartbeat(profile)
    authStore.set({
      authState: "authenticated",
      username: profile.handle,
      isLoading: false,
      error: null,
      webAuthnAvailable: webAuthnAvailable(),
      nodeRegistration: registrationFor(profile),
      unlockedProfile: profile,
      sealedProfile: sealed,
      profileSummaries: readProfileIndex(),
      activeProfileId: profileIdFor(sealed),
      localMessages: localMessagesFor(profile),
    })
    return true
  } catch {
    authStore.set({ ...authStore.get(), authState: "locked", isLoading: false, error: null, sealedProfile: sealed })
    return false
  }
}

export function continueAsGuest() {
  authStore.set({ ...authStore.get(), authState: "unauthenticated", username: "", error: null })
}

export function lockAuth() {
  clearSessionResumeTicket()
  clearTransientAppSessions()
  const current = authStore.get()
  authStore.set({
    ...current,
    authState: current.sealedProfile ? "locked" : "unauthenticated",
    username: "",
    nodeRegistration: null,
    unlockedProfile: null,
    localMessages: [],
    isLoading: false,
  })
}

export function signOutAuth() {
  lockAuth()
}

export function clearAuthError() {
  authStore.set({ ...authStore.get(), error: null })
}

export function exportProfileContainer(): string | null {
  const sealed = readSealedProfile()
  return sealed ? JSON.stringify(sealed, null, 2) : null
}

export async function importProfileContainer(serialized: string): Promise<boolean> {
  try {
    const parsed = JSON.parse(serialized) as SealedProfileContainer
    if (parsed.kind !== "edgerun.browser-profile.sealed" || parsed.version !== 1) {
      throw new Error("Not an Edgerun sealed profile container.")
    }
    persistSealedProfile(parsed)
    clearSessionResumeTicket()
    clearTransientAppSessions()
    const sealed = readSealedProfile() ?? parsed
    authStore.set({
      ...authStore.get(),
      authState: "locked",
      username: "",
      isLoading: false,
      error: null,
      nodeRegistration: null,
      unlockedProfile: null,
      sealedProfile: sealed,
      profileSummaries: readProfileIndex(),
      activeProfileId: profileIdFor(sealed),
      localMessages: [],
    })
    return true
  } catch (err) {
    authStore.set({ ...authStore.get(), error: err instanceof Error ? err.message : "Profile import failed." })
    return false
  }
}

export function switchProfile(profileId: string) {
  const sealed = readProfileById(profileId)
  if (!sealed) {
    authStore.set({ ...authStore.get(), error: "Profile container was not found." })
    return
  }
  clearSessionResumeTicket()
  clearTransientAppSessions()
  localStorage.setItem(ACTIVE_PROFILE_KEY, profileId)
  authStore.set({
    ...authStore.get(),
    authState: "locked",
    username: "",
    error: null,
    nodeRegistration: null,
    unlockedProfile: null,
    sealedProfile: sealed,
    profileSummaries: readProfileIndex(),
    activeProfileId: profileId,
    localMessages: [],
  })
}

async function persistUnlockedProfile(profile: UnlockedProfileContainer, password: string): Promise<SealedProfileContainer> {
  const current = readSealedProfile()
  if (!current) throw new Error("No sealed profile container found.")
  await openProfile(current, password)
  const normalizedProfile = normalizeUnlockedProfile(profile)
  const sealed = await sealProfile(normalizedProfile, password)
  persistSealedProfile(sealed)
  startSessionResumeHeartbeat(normalizedProfile)
  authStore.set({
    ...authStore.get(),
    authState: "authenticated",
    username: normalizedProfile.handle,
    error: null,
    nodeRegistration: registrationFor(normalizedProfile),
    unlockedProfile: normalizedProfile,
    sealedProfile: sealed,
    profileSummaries: readProfileIndex(),
    activeProfileId: profileIdFor(sealed),
    localMessages: localMessagesFor(normalizedProfile),
  })
  return sealed
}

export async function createProfileNode(label: string, password: string): Promise<boolean> {
  const state = authStore.get()
  const profile = state.unlockedProfile
  if (!profile) return false
  authStore.set({ ...state, isLoading: true, error: null })
  try {
    const generated = await generateP256Identity()
    const event = await createGenesisEvent(
      generated.material,
      generated.keyPair.privateKey,
      profile.owner.identityIdHex,
      label.trim() || `node-${profile.nodes.length + 1}`,
    )
    const nextProfile: UnlockedProfileContainer = {
      ...profile,
      nodes: [...profile.nodes, generated.material],
      eventLog: [...profile.eventLog, event],
    }
    await persistUnlockedProfile(nextProfile, password)
    authStore.set({ ...authStore.get(), isLoading: false })
    return true
  } catch (err) {
    authStore.set({ ...authStore.get(), isLoading: false, error: err instanceof Error ? err.message : "Node creation failed." })
    return false
  }
}

export async function publishBrowserNodeRelayRoute(input: { reachableTarget: string; expiresInSeconds?: number }): Promise<NodeRelayPublishResult | null> {
  const state = authStore.get()
  const profile = state.unlockedProfile
  if (!profile) return null
  const reachableTarget = input.reachableTarget.trim()
  if (!reachableTarget) {
    authStore.set({ ...state, error: "Reachable target is required." })
    return null
  }
  authStore.set({ ...state, isLoading: true, error: null })
  try {
    const updatedAtIso = new Date().toISOString()
    const expiresAtIso = new Date(Date.now() + (input.expiresInSeconds ?? 3600) * 1000).toISOString()
    const signedPayload = JSON.stringify({
      version: 1,
      nodeId: profile.browserNode.identityIdHex,
      publicKeyRawBase64: profile.browserNode.publicKeyRawBase64,
      reachableTarget,
      updatedAtIso,
      expiresAtIso,
      protocols: ["edgerun-msg-v1"],
      nonce: crypto.randomUUID(),
    })
    const privateKey = await importP256PrivateKey(profile.browserNode)
    const signature = new Uint8Array(await crypto.subtle.sign(
      { name: "ECDSA", hash: "SHA-256" },
      privateKey,
      textEncoder.encode(signedPayload),
    ))
    const response = await fetch(`https://${NODE_RELAY_DOMAIN}/nodes/update`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ signedPayload, signatureBase64: bytesToBase64(signature) }),
    })
    const data = await response.json().catch(() => null) as NodeRelayPublishResult | { error?: string } | null
    if (!response.ok || !data || !("ok" in data)) {
      throw new Error(data && "error" in data ? data.error : "Relay publish failed.")
    }
    authStore.set({ ...authStore.get(), isLoading: false })
    return data
  } catch (err) {
    authStore.set({ ...authStore.get(), isLoading: false, error: err instanceof Error ? err.message : "Relay publish failed." })
    return null
  }
}

export async function createNestedSealedContainer(input: {
  kind: SealedNestedContainer["kind"]
  label: string
  recipientId: string
  plaintext: string
  password: string
}): Promise<boolean> {
  const state = authStore.get()
  const profile = state.unlockedProfile
  if (!profile) return false
  authStore.set({ ...state, isLoading: true, error: null })
  try {
    await persistUnlockedProfile(profile, input.password)
    const contact = profile.contacts.find((item) => item.identityIdHex === input.recipientId)
    if (!contact) throw new Error("Recipient public key is not in contacts.")
    const payload = textEncoder.encode(input.plaintext)
    const sealed = await sealToRecipient(payload, contact.publicKeyRawBase64)
    const nested: SealedNestedContainer = {
      version: 1,
      id: crypto.randomUUID(),
      kind: input.kind,
      label: input.label.trim() || input.kind,
      recipientId: input.recipientId,
      createdAtIso: new Date().toISOString(),
      cipher: "AES-GCM-256",
      seal: {
        scheme: "P256-ECDH-AES-GCM-SHA256",
        ephemeralPublicKeyRawBase64: sealed.ephemeralPublicKeyRawBase64,
        senderEncryptionId: profile.ownerEncryption.identityIdHex,
      },
      ivBase64: bytesToBase64(sealed.iv),
      ciphertextBase64: bytesToBase64(sealed.ciphertext),
      plaintextSha256: await sha256Hex(payload),
    }
    const outbox: RoutedSealedEnvelope[] = contact.routeHint === "local-profile" ? profile.outbox : [
      ...profile.outbox,
      {
        version: 1,
        id: crypto.randomUUID(),
        fromId: profile.ownerEncryption.identityIdHex,
        toId: contact.identityIdHex,
        routeHint: contact.routeHint,
        sealedContainerId: nested.id,
        createdAtIso: nested.createdAtIso,
        status: "queued",
      },
    ]
    if (input.kind === "message") {
      const queued: LocalQueuedMessage = {
        version: 1,
        id: crypto.randomUUID(),
        fromId: profile.ownerEncryption.identityIdHex,
        toId: contact.identityIdHex,
        routeHint: contact.routeHint,
        sealedContainer: nested,
        createdAtIso: nested.createdAtIso,
        status: "queued",
      }
      const existingQueue = readLocalMessageQueue()
      writeLocalMessageQueue([...existingQueue, queued])
    }
    await persistUnlockedProfile({ ...profile, sealedContainers: [...profile.sealedContainers, nested], outbox }, input.password)
    authStore.set({ ...authStore.get(), isLoading: false })
    return true
  } catch (err) {
    authStore.set({ ...authStore.get(), isLoading: false, error: err instanceof Error ? err.message : "Sealed container creation failed." })
    return false
  }
}

export async function addContactToProfile(input: {
  label: string
  publicKeyRawBase64: string
  routeHint: string
  password: string
}): Promise<boolean> {
  const state = authStore.get()
  const profile = state.unlockedProfile
  if (!profile) return false
  authStore.set({ ...state, isLoading: true, error: null })
  try {
    await persistUnlockedProfile(profile, input.password)
    const raw = base64ToBytes(input.publicKeyRawBase64.trim())
    const identityBytes = raw[0] === 0x04 && raw.length === 65 ? raw.slice(1) : raw
    const contact: ContactRecord = {
      id: bytesToHex(identityBytes),
      label: input.label.trim() || "External identity",
      publicKeyRawBase64: input.publicKeyRawBase64.trim(),
      identityIdHex: bytesToHex(identityBytes),
      routeHint: input.routeHint.trim() || "mesh",
      addedAtIso: new Date().toISOString(),
    }
    await importP256EcdhPublicKey(contact.publicKeyRawBase64)
    const contacts = profile.contacts.some((item) => item.identityIdHex === contact.identityIdHex)
      ? profile.contacts.map((item) => item.identityIdHex === contact.identityIdHex ? contact : item)
      : [...profile.contacts, contact]
    await persistUnlockedProfile({ ...profile, contacts }, input.password)
    authStore.set({ ...authStore.get(), isLoading: false })
    return true
  } catch (err) {
    authStore.set({ ...authStore.get(), isLoading: false, error: err instanceof Error ? err.message : "Contact import failed." })
    return false
  }
}

function sanitizeKnownNodeIds(value: unknown): string[] {
  if (!Array.isArray(value)) return []
  return Array.from(new Set(value
    .map((item) => typeof item === "string" ? item.trim() : "")
    .filter(Boolean)))
}

export async function saveContactToProfile(input: {
  id?: string
  label: string
  password: string
  publicKeyRawBase64?: string
  identityIdHex?: string
  routeHint?: string
  email?: string
  phone?: string
  photoUrl?: string
  source?: ContactRecord["source"]
  sourceId?: string
  knownNodeIds?: string[]
  notes?: string
}): Promise<boolean> {
  const state = authStore.get()
  const profile = state.unlockedProfile
  if (!profile) return false
  authStore.set({ ...state, isLoading: true, error: null })
  try {
    await persistUnlockedProfile(profile, input.password)
    const now = new Date().toISOString()
    const existing = profile.contacts.find((item) =>
      (input.id && item.id === input.id)
      || (input.identityIdHex && item.identityIdHex === input.identityIdHex)
      || (input.source && input.sourceId && item.source === input.source && item.sourceId === input.sourceId)
    )
    const publicKeyRawBase64 = input.publicKeyRawBase64?.trim() ?? existing?.publicKeyRawBase64 ?? ""
    let identityIdHex = input.identityIdHex?.trim() || existing?.identityIdHex || ""

    if (publicKeyRawBase64) {
      const raw = base64ToBytes(publicKeyRawBase64)
      const identityBytes = raw[0] === 0x04 && raw.length === 65 ? raw.slice(1) : raw
      identityIdHex = bytesToHex(identityBytes)
      await importP256EcdhPublicKey(publicKeyRawBase64)
    }

    if (!identityIdHex) {
      identityIdHex = await sha256Hex(`${input.source ?? "manual"}:${input.sourceId ?? input.email ?? input.phone ?? input.label}`)
    }

    const contact: ContactRecord = {
      id: existing?.id ?? input.id ?? identityIdHex,
      label: input.label.trim() || existing?.label || "Contact",
      publicKeyRawBase64,
      identityIdHex,
      routeHint: input.routeHint?.trim() || existing?.routeHint || (publicKeyRawBase64 ? "mesh" : "address-book"),
      addedAtIso: existing?.addedAtIso ?? now,
      email: input.email !== undefined ? input.email.trim() || undefined : existing?.email,
      phone: input.phone !== undefined ? input.phone.trim() || undefined : existing?.phone,
      photoUrl: input.photoUrl !== undefined ? input.photoUrl.trim() || undefined : existing?.photoUrl,
      source: input.source ?? existing?.source ?? (publicKeyRawBase64 ? "edgerun" : "manual"),
      sourceId: input.sourceId?.trim() || existing?.sourceId,
      knownNodeIds: sanitizeKnownNodeIds(input.knownNodeIds ?? existing?.knownNodeIds),
      notes: input.notes !== undefined ? input.notes.trim() || undefined : existing?.notes,
      updatedAtIso: now,
    }

    const contacts = profile.contacts.some((item) => item.id === contact.id || item.identityIdHex === contact.identityIdHex)
      ? profile.contacts.map((item) => item.id === contact.id || item.identityIdHex === contact.identityIdHex ? contact : item)
      : [...profile.contacts, contact]
    await persistUnlockedProfile({ ...profile, contacts }, input.password)
    authStore.set({ ...authStore.get(), isLoading: false })
    return true
  } catch (err) {
    authStore.set({ ...authStore.get(), isLoading: false, error: err instanceof Error ? err.message : "Contact save failed." })
    return false
  }
}

export async function updateProfilePreferences(input: {
  password: string
  patch: Partial<ProfilePreferences>
}): Promise<boolean> {
  const state = authStore.get()
  const profile = state.unlockedProfile
  if (!profile) return false
  authStore.set({ ...state, isLoading: true, error: null })
  try {
    await persistUnlockedProfile(profile, input.password)
    const preferences = sanitizeProfilePreferences({ ...profile.profilePreferences, ...input.patch }, profile.handle)
    const nodePrivateKey = await importP256PrivateKey(profile.browserNode)
    const event = await createProfileSettingsEvent(profile, nodePrivateKey, preferences)
    await persistUnlockedProfile({
      ...profile,
      profilePreferences: preferences,
      eventLog: [...profile.eventLog, event],
    }, input.password)
    authStore.set({ ...authStore.get(), isLoading: false })
    return true
  } catch (err) {
    authStore.set({ ...authStore.get(), isLoading: false, error: err instanceof Error ? err.message : "Profile settings update failed." })
    return false
  }
}

export async function bindWebAuthnToProfile(password: string): Promise<boolean> {
  const state = authStore.get()
  const profile = state.unlockedProfile
  if (!profile) return false
  if (!webAuthnAvailable()) {
    authStore.set({ ...state, error: "WebAuthn is not available in this browser." })
    return false
  }
  authStore.set({ ...state, isLoading: true, error: null })
  try {
    await persistUnlockedProfile(profile, password)
    const credential = await navigator.credentials.create({
      publicKey: await credentialCreationOptions(profile),
    }) as PublicKeyCredential | null
    if (!credential) throw new Error("Passkey creation was cancelled.")
    const credentialIdBase64 = bytesToBase64(new Uint8Array(credential.rawId))
    const profileId = profile.ownerEncryption.identityIdHex
    const createPrfSecret = webAuthnPrfResult(credential)
    const prfSecret = createPrfSecret ?? await requestWebAuthnPrf(credentialIdBase64, profileId)
    await writeWebAuthnVault(profileId, credentialIdBase64, password, prfSecret)
    const binding: WebAuthnBinding = {
      credentialIdBase64,
      boundAtIso: new Date().toISOString(),
      unlockMethod: "prf-local-vault",
    }
    const nodePrivateKey = await importP256PrivateKey(profile.browserNode)
    const event = await createWebAuthnBoundEvent(profile, nodePrivateKey, binding)
    await persistUnlockedProfile({
      ...profile,
      webAuthnBinding: binding,
      eventLog: [...profile.eventLog, event],
    }, password)
    authStore.set({ ...authStore.get(), isLoading: false })
    return true
  } catch (err) {
    authStore.set({ ...authStore.get(), isLoading: false, error: err instanceof Error ? err.message : "Passkey binding failed." })
    return false
  }
}

export async function saveOAuthProfileSecret(input: { appId: OAuthAppId; password: string; secret: OAuthProfileSecretInput }): Promise<boolean> {
  const state = authStore.get()
  const profile = state.unlockedProfile
  if (!profile) return false
  authStore.set({ ...state, isLoading: true, error: null })
  try {
    validateOAuthSecretInput(input.appId, input.secret)
    const nextSecret: OAuthProfileSecret = {
      appId: input.appId,
      kind: "oauth2",
      ...input.secret,
      updatedAtIso: new Date().toISOString(),
    }
    await persistUnlockedProfile({
      ...profile,
      appSecrets: [
        ...profile.appSecrets.filter((secret) => secret.appId !== input.appId),
        nextSecret,
      ],
    }, input.password)
    authStore.set({ ...authStore.get(), isLoading: false })
    return true
  } catch (err) {
    authStore.set({ ...authStore.get(), isLoading: false, error: err instanceof Error ? err.message : "OAuth profile secret save failed." })
    return false
  }
}

export async function removeOAuthProfileSecret(appId: OAuthAppId, password: string): Promise<boolean> {
  const state = authStore.get()
  const profile = state.unlockedProfile
  if (!profile) return false
  authStore.set({ ...state, isLoading: true, error: null })
  try {
    await persistUnlockedProfile(profile, password)
    await persistUnlockedProfile({
      ...profile,
      appSecrets: profile.appSecrets.filter((secret) => secret.appId !== appId),
    }, password)
    authStore.set({ ...authStore.get(), isLoading: false })
    return true
  } catch (err) {
    authStore.set({ ...authStore.get(), isLoading: false, error: err instanceof Error ? err.message : "OAuth profile secret removal failed." })
    return false
  }
}

export async function saveGmailProfileSecret(input: { password: string; secret: Omit<GmailProfileSecret, "appId" | "kind" | "updatedAtIso"> }): Promise<boolean> {
  return saveOAuthProfileSecret({ appId: "gmail", ...input })
}

export async function removeGmailProfileSecret(password: string): Promise<boolean> {
  return removeOAuthProfileSecret("gmail", password)
}

export async function openLocalQueuedMessage(messageId: string): Promise<string | null> {
  const state = authStore.get()
  const profile = state.unlockedProfile
  if (!profile) return null
  const message = readLocalMessageQueue().find((item) => (
    item.id === messageId
    && item.toId === profile.ownerEncryption.identityIdHex
  ))
  if (!message) {
    authStore.set({ ...state, error: "Queued message was not found for this profile." })
    return null
  }
  try {
    return await openSealedNestedContainer(message.sealedContainer, profile.ownerEncryption)
  } catch (err) {
    authStore.set({ ...authStore.get(), error: err instanceof Error ? err.message : "Queued message open failed." })
    return null
  }
}
