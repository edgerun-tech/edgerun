export function bytesToHex(bytes: Uint8Array | number[] | null | undefined): string {
  if (!bytes) return ""
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("")
}

export function bytesToBase64(bytes: Uint8Array): string {
  let binary = ""
  for (let offset = 0; offset < bytes.length; offset += 0x8000) {
    binary += String.fromCharCode(...bytes.slice(offset, offset + 0x8000))
  }
  return btoa(binary)
}

export function base64ToBytes(base64: string): Uint8Array {
  const binary = atob(base64)
  const bytes = new Uint8Array(binary.length)
  for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index)
  return bytes
}

export async function sha256Hex(data: Uint8Array | string): Promise<string> {
  const bytes = typeof data === "string" ? new TextEncoder().encode(data) : data
  return bytesToHex(new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)))
}
