export function formatDate(d: Date): string {
  const diff = Date.now() - d.getTime()
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`
  if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`
  return d.toLocaleDateString("en-US", { month: "short", day: "numeric" })
}

export function formatDateFromIso(dateStr: string): string {
  const now = new Date()
  const date = new Date(dateStr)
  const diff = now.getTime() - date.getTime()
  if (diff < 86400000 && now.getDate() === date.getDate()) return `Today: ${date.toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit" })}`
  return date.toLocaleDateString("en-US", { month: "short", day: "numeric" })
}

export function formatTime(date: Date): string {
  return date.toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit", hour12: false })
}

export function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60)
  const s = Math.floor(seconds % 60)
  return `${m}:${s.toString().padStart(2, "0")}`
}

export function formatBytes(bytes: number | string | undefined | null, fallback = "0 B"): string {
  const n = typeof bytes === "string" ? Number(bytes) : (bytes ?? 0)
  if (!Number.isFinite(n) || n <= 0) return fallback
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`
  if (n < 1024 * 1024 * 1024 * 1024) return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`
  return `${(n / (1024 * 1024 * 1024 * 1024)).toFixed(2)} TB`
}

export function shortHex(value: string, prefixLen = 10, suffixLen = 6): string {
  if (value.length <= prefixLen + suffixLen + 1) return value
  return `${value.slice(0, prefixLen)}…${value.slice(-suffixLen)}`
}
