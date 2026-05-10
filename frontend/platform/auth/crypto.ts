import { bytesToHex, bytesToBase64, base64ToBytes } from "@/platform/utils/bytes"
import type { P256KeyMaterial, P256EncryptionKeyMaterial } from "@/stores/auth-types"

const textEncoder = new TextEncoder()
const textDecoder = new TextDecoder()

export function canonicalJson(value: unknown): string {
  if (value === null || typeof value !== "object") return JSON.stringify(value)
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`
  const record = value as Record<string, unknown>
  return `{${Object.keys(record).sort().map((key) => `${JSON.stringify(key)}:${canonicalJson(record[key])}`).join(",")}}`
}

export async function sha256Hex(bytes: Uint8Array | string): Promise<string> {
  const data = typeof bytes === "string" ? textEncoder.encode(bytes) : bytes
  return bytesToHex(new Uint8Array(await crypto.subtle.digest("SHA-256", data)))
}

export async function deriveAesKey(password: string, salt: Uint8Array, rounds: number): Promise<CryptoKey> {
  const baseKey = await crypto.subtle.importKey("raw", textEncoder.encode(password), "PBKDF2", false, ["deriveKey"])
  return crypto.subtle.deriveKey(
    { name: "PBKDF2", salt, iterations: rounds, hash: "SHA-256" },
    baseKey,
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt", "decrypt"],
  )
}

export async function importAesGcmKey(raw: BufferSource): Promise<CryptoKey> {
  return crypto.subtle.importKey("raw", raw, { name: "AES-GCM", length: 256 }, false, ["encrypt", "decrypt"])
}

export async function generateP256Identity(): Promise<{ keyPair: CryptoKeyPair; material: P256KeyMaterial }> {
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

export async function generateP256EncryptionIdentity(): Promise<{ keyPair: CryptoKeyPair; material: P256EncryptionKeyMaterial }> {
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

export async function importP256PrivateKey(material: P256KeyMaterial): Promise<CryptoKey> {
  return crypto.subtle.importKey(
    "pkcs8",
    base64ToBytes(material.privateKeyPkcs8Base64),
    { name: "ECDSA", namedCurve: "P-256" },
    false,
    ["sign"],
  )
}

export async function importP256EcdhPublicKey(publicKeyRawBase64: string): Promise<CryptoKey> {
  return crypto.subtle.importKey(
    "raw",
    base64ToBytes(publicKeyRawBase64),
    { name: "ECDH", namedCurve: "P-256" },
    false,
    [],
  )
}

export async function importP256EcdhPrivateKey(material: P256EncryptionKeyMaterial): Promise<CryptoKey> {
  return crypto.subtle.importKey(
    "pkcs8",
    base64ToBytes(material.privateKeyPkcs8Base64),
    { name: "ECDH", namedCurve: "P-256" },
    false,
    ["deriveKey"],
  )
}

export async function sealToRecipient(
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

export async function openSealedNestedContainer(
  container: { seal: { ephemeralPublicKeyRawBase64: string }; ivBase64: string; ciphertextBase64: string; plaintextSha256: string },
  recipient: P256EncryptionKeyMaterial,
): Promise<string> {
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
