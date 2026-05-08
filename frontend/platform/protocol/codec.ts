/**
 * Protocol codec - encode/decode generated protobuf messages.
 * Handles binary serialization, hashing, and transport encoding.
 */

export function encodeBase64(bytes: Uint8Array): string {
  return btoa(String.fromCharCode(...bytes))
}

export function decodeBase64(base64: string): Uint8Array {
  const binary = atob(base64)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i)
  }
  return bytes
}

export function encodeHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("")
}

export function decodeHex(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2)
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16)
  }
  return bytes
}

export async function sha256(bytes: Uint8Array): Promise<Uint8Array> {
  const hashBuffer = await crypto.subtle.digest("SHA-256", bytes)
  return new Uint8Array(hashBuffer)
}

export function concatenateBytes(...arrays: Uint8Array[]): Uint8Array {
  const totalLength = arrays.reduce((sum, arr) => sum + arr.length, 0)
  const result = new Uint8Array(totalLength)
  let offset = 0
  for (const arr of arrays) {
    result.set(arr, offset)
    offset += arr.length
  }
  return result
}

export function utf8Encode(str: string): Uint8Array {
  return new TextEncoder().encode(str)
}

export function utf8Decode(bytes: Uint8Array): string {
  return new TextDecoder().decode(bytes)
}

export interface CanonicalBytes {
  bytes: Uint8Array
  hash: Uint8Array
  hashHex: string
}

export async function computeCanonical(bytes: Uint8Array): Promise<CanonicalBytes> {
  const hash = await sha256(bytes)
  return {
    bytes,
    hash,
    hashHex: encodeHex(hash),
  }
}
