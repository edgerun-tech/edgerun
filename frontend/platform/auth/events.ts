import { bytesToBase64, sha256Hex } from "@/platform/utils/bytes"
import { canonicalJson } from "./crypto"
import type { P256KeyMaterial, ProfileEvent, ProfilePreferences, UnlockedProfileContainer, WebAuthnBinding } from "@/stores/auth-types"
import { ZERO_HASH } from "@/stores/auth-types"

const textEncoder = new TextEncoder()

async function signAndHash(unsigned: Record<string, unknown>, nodePrivateKey: CryptoKey): Promise<{ signatureBase64: string; eventHash: string }> {
  const signature = new Uint8Array(await crypto.subtle.sign(
    { name: "ECDSA", hash: "SHA-256" },
    nodePrivateKey,
    textEncoder.encode(canonicalJson(unsigned)),
  ))
  const signatureBase64 = bytesToBase64(signature)
  const eventHash = await sha256Hex(canonicalJson({ ...unsigned, signatureBase64 }))
  return { signatureBase64, eventHash }
}

export async function createGenesisEvent(node: P256KeyMaterial, nodePrivateKey: CryptoKey, ownerId: string, label?: string): Promise<ProfileEvent> {
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
  const { signatureBase64, eventHash } = await signAndHash(unsigned, nodePrivateKey)
  return { ...unsigned, signatureBase64, eventHash }
}

export async function createProfileSettingsEvent(profile: UnlockedProfileContainer, nodePrivateKey: CryptoKey, preferences: ProfilePreferences): Promise<ProfileEvent> {
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
  const { signatureBase64, eventHash } = await signAndHash(unsigned, nodePrivateKey)
  return { ...unsigned, signatureBase64, eventHash }
}

export async function createWebAuthnBoundEvent(profile: UnlockedProfileContainer, nodePrivateKey: CryptoKey, binding: WebAuthnBinding): Promise<ProfileEvent> {
  const previous = profile.eventLog[profile.eventLog.length - 1]
  const payloadSha256 = await sha256Hex(canonicalJson(binding))
  const unsigned = {
    seq: profile.eventLog.length,
    kind: "PROFILE_WEBAUTHN_BOUND" as const,
    previousEventHash: previous?.eventHash ?? ZERO_HASH,
    payloadSha256,
    payload: binding,
  }
  const { signatureBase64, eventHash } = await signAndHash(unsigned, nodePrivateKey)
  return { ...unsigned, signatureBase64, eventHash }
}
