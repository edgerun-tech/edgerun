import { base64ToBytes, bytesToBase64 } from "@/platform/utils/bytes"
import { importAesGcmKey } from "./crypto"

const WEBAUTHN_VAULT_PREFIX = "edgerun:webauthn_vault_v1"

export function webAuthnAvailable(): boolean {
  return typeof window !== "undefined" && "PublicKeyCredential" in window && Boolean(navigator.credentials)
}

export function webAuthnVaultExists(profileId: string, credentialIdBase64?: string): boolean {
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

export function hasUsableWebAuthnBinding(sealed: { webAuthnCredentialIdHint?: string } | null, profileId: string): boolean {
  if (!sealed || !sealed.webAuthnCredentialIdHint) return false
  return webAuthnVaultExists(profileId, sealed.webAuthnCredentialIdHint)
}

async function webAuthnPrfSalt(profileId: string): Promise<Uint8Array> {
  return new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(`edgerun:profile-unlock:${profileId}`)))
}

export async function credentialCreationOptions(profile: { ownerEncryption: { identityIdHex: string; publicKeyRawBase64: string }; handle: string }): Promise<PublicKeyCredentialCreationOptions> {
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

export function webAuthnPrfResult(credential: PublicKeyCredential | null): Uint8Array | null {
  const results = credential?.getClientExtensionResults() as { prf?: { enabled?: boolean; results?: { first?: ArrayBuffer } } }
  const first = results.prf?.results?.first
  return first ? new Uint8Array(first) : null
}

export async function requestWebAuthnPrf(credentialIdBase64: string, profileId: string): Promise<Uint8Array> {
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

export async function writeWebAuthnVault(profileId: string, credentialIdBase64: string, profilePassword: string, prfSecret: Uint8Array) {
  const key = await importAesGcmKey(prfSecret)
  const iv = crypto.getRandomValues(new Uint8Array(12))
  const ciphertext = new Uint8Array(await crypto.subtle.encrypt({ name: "AES-GCM", iv }, key, new TextEncoder().encode(profilePassword)))
  localStorage.setItem(`${WEBAUTHN_VAULT_PREFIX}${profileId}`, JSON.stringify({
    version: 1,
    credentialIdBase64,
    ivBase64: bytesToBase64(iv),
    ciphertextBase64: bytesToBase64(ciphertext),
  }))
}

export async function readWebAuthnVaultPassword(profileId: string, credentialIdBase64: string, prfSecret: Uint8Array): Promise<string> {
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
  return new TextDecoder().decode(plaintext)
}
