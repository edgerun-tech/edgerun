export function bytesToHex(bytes: Uint8Array | number[] | null | undefined): string {
  if (!bytes) return ""
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("")
}
