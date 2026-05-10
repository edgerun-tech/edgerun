import type { SealedProfileContainer, ProfileSummary, LocalQueuedMessage } from "@/stores/auth-types"

const PROFILE_INDEX_KEY = "edgerun:profile:index_v2"
const ACTIVE_PROFILE_KEY = "edgerun:active_profile_v1"
const PROFILE_RECORD_PREFIX = "edgerun:profile:"
const LOCAL_MESSAGE_QUEUE_KEY = "edgerun:local_message_queue_v1"
const LEGACY_PROFILE_STORAGE_KEY = "edgerun:sealed-profile"
const LEGACY_KEYS = [
  "edgerun:sealed-profile",
  "edgerun:profile:locked",
  "edgerun:profile:unlocked",
  "edgerun:profile:container",
  "edgerun:active-profile",
]

export function profileIdFor(sealed: SealedProfileContainer): string {
  return sealed.profileId ?? sealed.encryptionIdHint ?? sealed.ownerIdHint
}

export function shortProfileId(value: string): string {
  return value ? `profile-${value.slice(0, 8)}` : "profile"
}

export function summaryFor(sealed: SealedProfileContainer): ProfileSummary {
  const profileId = profileIdFor(sealed)
  return {
    profileId,
    handle: sealed.handleHint ?? shortProfileId(sealed.ownerIdHint),
    ownerIdHint: sealed.ownerIdHint,
    encryptionIdHint: sealed.encryptionIdHint ?? profileId,
    encryptionPublicKeyHint: sealed.encryptionPublicKeyHint ?? "",
    webAuthnCredentialIdHint: sealed.webAuthnCredentialIdHint,
    webAuthnUnlockAvailable: false,
    nodeIdHint: sealed.nodeIdHint,
    createdAtIso: sealed.createdAtIso,
  }
}

export function readProfileIndex(): ProfileSummary[] {
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

export function writeProfileIndex(summaries: ProfileSummary[]) {
  localStorage.setItem(PROFILE_INDEX_KEY, JSON.stringify(summaries))
}

export function readProfileById(profileId: string): SealedProfileContainer | null {
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

export function migrateLegacyProfile(): SealedProfileContainer | null {
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

export function readSealedProfile(): SealedProfileContainer | null {
  if (typeof window === "undefined") return null
  const index = readProfileIndex()
  const activeProfileId = localStorage.getItem(ACTIVE_PROFILE_KEY) ?? index[0]?.profileId ?? null
  if (activeProfileId) {
    const active = readProfileById(activeProfileId)
    if (active) return active
  }
  return migrateLegacyProfile()
}

export function persistSealedProfile(sealed: SealedProfileContainer) {
  const pid = profileIdFor(sealed)
  const normalized: SealedProfileContainer = {
    ...sealed,
    profileId: pid,
    handleHint: sealed.handleHint ?? shortProfileId(sealed.ownerIdHint),
    encryptionIdHint: sealed.encryptionIdHint ?? pid,
    encryptionPublicKeyHint: sealed.encryptionPublicKeyHint ?? "",
    webAuthnCredentialIdHint: sealed.webAuthnCredentialIdHint,
  }
  localStorage.setItem(`${PROFILE_RECORD_PREFIX}${pid}`, JSON.stringify(normalized))
  const nextSummary = summaryFor(normalized)
  const existing = readProfileIndex().filter((item) => item.profileId !== pid)
  writeProfileIndex([...existing, nextSummary].sort((left, right) => left.createdAtIso.localeCompare(right.createdAtIso)))
  localStorage.setItem(ACTIVE_PROFILE_KEY, pid)
  for (const key of LEGACY_KEYS) localStorage.removeItem(key)
}

export function activeProfileId(): string | null {
  if (typeof window === "undefined") return null
  return localStorage.getItem(ACTIVE_PROFILE_KEY) ?? readProfileIndex()[0]?.profileId ?? null
}

export function readLocalMessageQueue(): LocalQueuedMessage[] {
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

export function writeLocalMessageQueue(messages: LocalQueuedMessage[]) {
  localStorage.setItem(LOCAL_MESSAGE_QUEUE_KEY, JSON.stringify(messages))
}

export function localMessagesFor(profileIdHex: string): LocalQueuedMessage[] {
  return readLocalMessageQueue().filter((message) => message.toId === profileIdHex)
}
