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

export type P256KeyMaterial = {
  algorithm: "ECDSA_P256_SHA256"
  identityIdHex: string
  publicKeyRawBase64: string
  privateKeyPkcs8Base64: string
}

export type P256EncryptionKeyMaterial = {
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
  email?: string
  phone?: string
  photoUrl?: string
  source?: "edgerun" | "google" | "manual"
  sourceId?: string
  knownNodeIds?: string[]
  notes?: string
  updatedAtIso?: string
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

export type OAuthAppId = "gmail" | "google-drive" | "github" | "cloudflare"

export type OAuthProfileSecret = {
  appId: OAuthAppId
  kind: "oauth2"
  email: string
  accessToken: string
  refreshToken?: string
  expiresAtIso: string
  scopes: string[]
  updatedAtIso: string
}

export type GmailProfileSecret = OAuthProfileSecret & { appId: "gmail" }
export type GoogleDriveProfileSecret = OAuthProfileSecret & { appId: "google-drive" }
export type GitHubProfileSecret = OAuthProfileSecret & { appId: "github" }
export type CloudflareProfileSecret = OAuthProfileSecret & {
  appId: "cloudflare"
  accountId?: string
  tokenId?: string
  zoneId?: string
  zoneName?: string
}

export type OAuthProfileSecretInput = Omit<OAuthProfileSecret, "appId" | "kind" | "updatedAtIso"> & {
  accountId?: string
  tokenId?: string
  zoneId?: string
  zoneName?: string
}

export type NodeGenesisEvent = {
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

export type ProfileSettingsEvent = {
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

export type ProfileWebAuthnBoundEvent = {
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
  appSecrets: OAuthProfileSecret[]
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

export type NodeRelayPublishResult = {
  ok: boolean
  nodeId: string
  relayHost: string
  updatedAtIso: string
  expiresAtIso: string
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

export const LEGACY_PROFILE_STORAGE_KEY = "edgerun:sealed-profile-container:v1"
export const PROFILE_INDEX_KEY = "edgerun:sealed-profile-index:v1"
export const ACTIVE_PROFILE_KEY = "edgerun:active-profile-id:v1"
export const PROFILE_RECORD_PREFIX = "edgerun:sealed-profile:"
export const LOCAL_MESSAGE_QUEUE_KEY = "edgerun:local-message-queue:v1"
export const WEBAUTHN_VAULT_PREFIX = "edgerun:webauthn-profile-vault:"
export const SESSION_RESUME_KEY = "edgerun:session-resume-ticket:v1"
export const LEGACY_KEYS = ["edgerun_credential_id", "edgerun_username", "edgerun_node_registration_v1"]
export const PBKDF2_ROUNDS = 210_000
export const SESSION_RESUME_TTL_MS = 30_000
export const SESSION_RESUME_REFRESH_MS = 15_000
export const ZERO_HASH = "00".repeat(32)
export const DEFAULT_PROFILE_PREFERENCES: ProfilePreferences = {
  avatarInitials: "ID",
  avatarColor: "oklch(0.3 0.1 145)",
  connectToNetwork: true,
  shareResources: true,
}
