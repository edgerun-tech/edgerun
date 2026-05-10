import { atom, computed } from "nanostores"
import { bytesToHex, bytesToBase64, base64ToBytes, sha256Hex } from "@/platform/utils/bytes"
import { patchStore } from "@/platform/utils/store"
import { canonicalJson, generateP256Identity, generateP256EncryptionIdentity, importP256PrivateKey, importP256EcdhPublicKey, sealToRecipient, openSealedNestedContainer, sealProfile, openProfile } from "@/platform/auth/crypto"
import { createGenesisEvent, createProfileSettingsEvent, createWebAuthnBoundEvent } from "@/platform/auth/events"
import { profileIdFor, readProfileIndex, readProfileById, readSealedProfile, persistSealedProfile, activeProfileId, readLocalMessageQueue, writeLocalMessageQueue, localMessagesFor } from "@/platform/auth/persistence"
import { clearSessionResumeTicket, startSessionResumeHeartbeat, consumeSessionResumeTicket } from "@/platform/auth/session"
import { webAuthnAvailable, webAuthnVaultExists, hasUsableWebAuthnBinding, credentialCreationOptions, webAuthnPrfResult, requestWebAuthnPrf, writeWebAuthnVault, readWebAuthnVaultPassword } from "@/platform/auth/web-authn"
import { sanitizeProfilePreferences, isIsoDate, isHex, normalizeUnlockedProfile, deriveProfilePreferences, deriveWebAuthnBinding, selfContact, normalizeOAuthSecret, validateOAuthSecretInput, registrationFor, sanitizeKnownNodeIds } from "@/platform/auth/helpers"
import type {
  AuthState,
  NodeProvisionInput,
  StoredNodeRegistration,
  AuthStore,
  ContactRecord,
  ProfilePreferences,
  WebAuthnBinding,
  OAuthAppId,
  OAuthProfileSecret,
  OAuthProfileSecretInput,
  GmailProfileSecret,
  ProfileEvent,
  SealedNestedContainer,
  RoutedSealedEnvelope,
  UnlockedProfileContainer,
  SealedProfileContainer,
  ProfileSummary,
  LocalQueuedMessage,
  NodeRelayPublishResult,
} from "./auth-types"


export type {
  AuthState,
  AuthStore,
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

async function persistUnlockedProfile(profile: UnlockedProfileContainer, password: string): Promise<SealedProfileContainer> {
  const current = readSealedProfile()
  if (!current) throw new Error("No sealed profile container found.")
  await openProfile(current, password)
  const normalizedProfile = normalizeUnlockedProfile(profile)
  const sealed = await sealProfile(normalizedProfile, password)
  persistSealedProfile(sealed)
  startSessionResumeHeartbeat(
    normalizedProfile.ownerEncryption.identityIdHex,
    () => canonicalJson(normalizedProfile),
    normalizedProfile.handle,
    () => authStore.get().authState === "authenticated",
  )
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
    localMessages: localMessagesFor(normalizedProfile.ownerEncryption.identityIdHex),
  })
  return sealed
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
    startSessionResumeHeartbeat(
      profile.ownerEncryption.identityIdHex,
      () => canonicalJson(profile),
      profile.handle,
      () => authStore.get().authState === "authenticated",
    )
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
      localMessages: localMessagesFor(profile.ownerEncryption.identityIdHex),
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
    startSessionResumeHeartbeat(
      profile.ownerEncryption.identityIdHex,
      () => canonicalJson(profile),
      profile.handle,
      () => authStore.get().authState === "authenticated",
    )
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
      localMessages: localMessagesFor(profile.ownerEncryption.identityIdHex),
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
  if (!sealed || !credentialIdBase64) {
    authStore.set({ ...state, error: "No usable passkey unlock vault is bound to this profile." })
    return false
  }
  const sealedProfileId = profileIdFor(sealed)
  if (!hasUsableWebAuthnBinding(sealed, sealedProfileId)) {
    authStore.set({ ...state, error: "No usable passkey unlock vault is bound to this profile." })
    return false
  }
  authStore.set({ ...state, authState: "authenticating", isLoading: true, error: null, sealedProfile: sealed })
  try {
    const profileId = sealedProfileId
    const prfSecret = await requestWebAuthnPrf(credentialIdBase64, profileId)
    const password = await readWebAuthnVaultPassword(profileId, credentialIdBase64, prfSecret)
    const profile = await openProfile(sealed, password)
    await importP256PrivateKey(profile.owner)
    await importP256PrivateKey(profile.browserNode)
    startSessionResumeHeartbeat(
      profile.ownerEncryption.identityIdHex,
      () => canonicalJson(profile),
      profile.handle,
      () => authStore.get().authState === "authenticated",
    )
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
      localMessages: localMessagesFor(profile.ownerEncryption.identityIdHex),
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
    const rawProfile = await consumeSessionResumeTicket(
      sealed.ownerIdHint,
      () => sealed ? profileIdFor(sealed) : null,
      async () => {},
      (raw) => normalizeUnlockedProfile(raw as UnlockedProfileContainer),
    )
    if (!rawProfile) {
      authStore.set({ ...authStore.get(), authState: "locked", isLoading: false, error: null, sealedProfile: sealed })
      return false
    }
    const profile = rawProfile as UnlockedProfileContainer
    await importP256PrivateKey(profile.owner)
    await importP256PrivateKey(profile.browserNode)
    startSessionResumeHeartbeat(
      profile.ownerEncryption.identityIdHex,
      () => canonicalJson(profile),
      profile.handle,
      () => authStore.get().authState === "authenticated",
    )
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
      localMessages: localMessagesFor(profile.ownerEncryption.identityIdHex),
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
  patchStore(authStore, { error: null })
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
  localStorage.setItem("edgerun:active_profile_v1", profileId)
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
    patchStore(authStore, { isLoading: false })
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
    patchStore(authStore, { isLoading: false })
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
    patchStore(authStore, { isLoading: false })
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
    patchStore(authStore, { isLoading: false })
    return true
  } catch (err) {
    authStore.set({ ...authStore.get(), isLoading: false, error: err instanceof Error ? err.message : "Contact import failed." })
    return false
  }
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
    patchStore(authStore, { isLoading: false })
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
    patchStore(authStore, { isLoading: false })
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
    patchStore(authStore, { isLoading: false })
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
    patchStore(authStore, { isLoading: false })
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
    patchStore(authStore, { isLoading: false })
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
