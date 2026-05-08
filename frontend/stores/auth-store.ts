import { atom, computed } from "nanostores"

export type AuthState = "idle" | "registering" | "authenticating" | "authenticated" | "locked" | "error" | "unauthenticated"

export interface NodeProvisionInput {
  passphrase?: string
}

export interface StoredNodeRegistration {
  nodeId: string
  nodeTarget: string
  username: string
  ownerId: string
  genesisEventHash: string
  registeredAtIso: string
}

type P256KeyMaterial = {
  algorithm: "ECDSA_P256_SHA256"
  identityIdHex: string
  publicKeyRawBase64: string
  privateKeyPkcs8Base64: string
}

type P256EncryptionKeyMaterial = {
  algorithm: "ECDH_P256_HKDF_SHA256"
  identityIdHex: string
  publicKeyRawBase64: string
  privateKeyPkcs8Base64: string
}

export type ContactRecord = {
  id: string
  label: string
  publicKeyRawBase64: string
  identityIdHex: string
  routeHint: string
  addedAtIso: string
}

export type ProfilePreferences = {
  avatarInitials: string
  avatarColor: string
  connectToNetwork: boolean
  shareResources: boolean
}

export type WebAuthnBinding = {
  credentialIdBase64: string
  boundAtIso: string
  unlockMethod: "prf-local-vault"
}

export type GmailProfileSecret = {
  appId: "gmail"
  kind: "oauth2"
  email: string
  accessToken: string
  refreshToken?: string
  expiresAtIso: string
  scopes: string[]
  updatedAtIso: string
}

type NodeGenesisEvent = {
  seq: number
  kind: "NODE_GENESIS"
  previousEventHash: string
  payloadSha256: string
  payload: {
    nodeId: string
    initialControllers: string[]
    createdAtIso: string
    label?: string
  }
  signatureBase64: string
  eventHash: string
}

type ProfileSettingsEvent = {
  seq: number
  kind: "PROFILE_SETTINGS_UPDATED"
  previousEventHash: string
  payloadSha256: string
  payload: {
    preferences: ProfilePreferences
    updatedAtIso: string
  }
  signatureBase64: string
  eventHash: string
}

type ProfileWebAuthnBoundEvent = {
  seq: number
  kind: "PROFILE_WEBAUTHN_BOUND"
  previousEventHash: string
  payloadSha256: string
  payload: WebAuthnBinding
  signatureBase64: string
  eventHash: string
}

export type ProfileEvent = NodeGenesisEvent | ProfileSettingsEvent | ProfileWebAuthnBoundEvent

export type SealedNestedContainer = {
  version: 1
  id: string
  kind: "message" | "profile" | "data"
  label: string
  recipientId: string
  createdAtIso: string
  cipher: "AES-GCM-256"
  seal: {
    scheme: "P256-ECDH-AES-GCM-SHA256"
    ephemeralPublicKeyRawBase64: string
    senderEncryptionId: string
  }
  ivBase64: string
  ciphertextBase64: string
  plaintextSha256: string
}

export type RoutedSealedEnvelope = {
  version: 1
  id: string
  fromId: string
  toId: string
  routeHint: string
  sealedContainerId: string
  createdAtIso: string
  status: "queued"
}

export type UnlockedProfileContainer = {
  version: 1
  handle: string
  createdAtIso: string
  owner: P256KeyMaterial
  ownerEncryption: P256EncryptionKeyMaterial
  browserNode: P256KeyMaterial
  nodes: P256KeyMaterial[]
  contacts: ContactRecord[]
  eventLog: ProfileEvent[]
  profilePreferences: ProfilePreferences
  webAuthnBinding?: WebAuthnBinding
  appSecrets: GmailProfileSecret[]
  sealedContainers: SealedNestedContainer[]
  outbox: RoutedSealedEnvelope[]
}

export type SealedProfileContainer = {
  version: 1
  kind: "edgerun.browser-profile.sealed"
  cipher: "AES-GCM-256"
  kdf: {
    name: "PBKDF2-HMAC-SHA256"
    rounds: number
    saltBase64: string
  }
  ivBase64: string
  ciphertextBase64: string
  profileId: string
  handleHint: string
  ownerIdHint: string
  encryptionIdHint: string
  encryptionPublicKeyHint: string
  webAuthnCredentialIdHint?: string
  nodeIdHint: string
  createdAtIso: string
}

export type ProfileSummary = {
  profileId: string
  handle: string
  ownerIdHint: string
  encryptionIdHint: string
  encryptionPublicKeyHint: string
  webAuthnCredentialIdHint?: string
  webAuthnUnlockAvailable: boolean
  nodeIdHint: string
  createdAtIso: string
}

export type LocalQueuedMessage = {
  version: 1
  id: string
  fromId: string
  toId: string
  routeHint: string
  sealedContainer: SealedNestedContainer
  createdAtIso: string
  status: "queued"
}

export interface AuthStore {
  authState: AuthState
  username: string
  isLoading: boolean
  error: string | null
  webAuthnAvailable: boolean
  nodeRegistration: StoredNodeRegistration | null
  unlockedProfile: UnlockedProfileContainer | null
  sealedProfile: SealedProfileContainer | null
  profileSummaries: ProfileSummary[]
  activeProfileId: string | null
  localMessages: LocalQueuedMessage[]
}

const LEGACY_PROFILE_STORAGE_KEY = "edgerun:sealed-profile-container:v1"
const PROFILE_INDEX_KEY = "edgerun:sealed-profile-index:v1"
const ACTIVE_PROFILE_KEY = "edgerun:active-profile-id:v1"
const PROFILE_RECORD_PREFIX = "edgerun:sealed-profile:"
const LOCAL_MESSAGE_QUEUE_KEY = "edgerun:local-message-queue:v1"
const WEBAUTHN_VAULT_PREFIX = "edgerun:webauthn-profile-vault:"
const LEGACY_KEYS = ["edgerun_credential_id", "edgerun_username", "edgerun_node_registration_v1"]
const PBKDF2_ROUNDS = 210_000
const ZERO_HASH = "00".repeat(32)
const DEFAULT_PROFILE_PREFERENCES: ProfilePreferences = {
  avatarInitials: "ID",
  avatarColor: "oklch(0.3 0.1 145)",
  connectToNetwork: true,
  shareResources: true,
}

const textEncoder = new TextEncoder()
const textDecoder = new TextDecoder()

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
  return {
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
  }
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

export function continueAsGuest() {
  authStore.set({ ...authStore.get(), authState: "unauthenticated", username: "", error: null })
}

export function lockAuth() {
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
  const sealed = await sealProfile(profile, password)
  persistSealedProfile(sealed)
  authStore.set({
    ...authStore.get(),
    authState: "authenticated",
    username: profile.handle,
    error: null,
    nodeRegistration: registrationFor(profile),
    unlockedProfile: profile,
    sealedProfile: sealed,
    profileSummaries: readProfileIndex(),
    activeProfileId: profileIdFor(sealed),
    localMessages: localMessagesFor(profile),
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

export async function saveGmailProfileSecret(input: { password: string; secret: Omit<GmailProfileSecret, "appId" | "kind" | "updatedAtIso"> }): Promise<boolean> {
  const state = authStore.get()
  const profile = state.unlockedProfile
  if (!profile) return false
  authStore.set({ ...state, isLoading: true, error: null })
  try {
    await persistUnlockedProfile(profile, input.password)
    const nextSecret: GmailProfileSecret = {
      appId: "gmail",
      kind: "oauth2",
      ...input.secret,
      updatedAtIso: new Date().toISOString(),
    }
    await persistUnlockedProfile({
      ...profile,
      appSecrets: [
        ...profile.appSecrets.filter((secret) => secret.appId !== "gmail"),
        nextSecret,
      ],
    }, input.password)
    authStore.set({ ...authStore.get(), isLoading: false })
    return true
  } catch (err) {
    authStore.set({ ...authStore.get(), isLoading: false, error: err instanceof Error ? err.message : "Gmail profile secret save failed." })
    return false
  }
}

export async function removeGmailProfileSecret(password: string): Promise<boolean> {
  const state = authStore.get()
  const profile = state.unlockedProfile
  if (!profile) return false
  authStore.set({ ...state, isLoading: true, error: null })
  try {
    await persistUnlockedProfile(profile, password)
    await persistUnlockedProfile({
      ...profile,
      appSecrets: profile.appSecrets.filter((secret) => secret.appId !== "gmail"),
    }, password)
    authStore.set({ ...authStore.get(), isLoading: false })
    return true
  } catch (err) {
    authStore.set({ ...authStore.get(), isLoading: false, error: err instanceof Error ? err.message : "Gmail profile secret removal failed." })
    return false
  }
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
