import { cn } from "@/lib/utils"

type StatusLevel = "success" | "warning" | "error" | "info" | "neutral"

function statusClassForLevel(level: StatusLevel): string {
  switch (level) {
    case "success":
      return "border-[var(--status-online)]/30 bg-[var(--status-online)]/10 text-[var(--status-online)]"
    case "warning":
      return "border-[var(--status-warning)]/30 bg-[var(--status-warning)]/10 text-[var(--status-warning)]"
    case "error":
      return "border-[var(--status-error)]/30 bg-[var(--status-error)]/10 text-[var(--status-error)]"
    case "info":
      return "border-[var(--status-info)]/30 bg-[var(--status-info)]/10 text-[var(--status-info)]"
    case "neutral":
      return "border-border/50 bg-muted/50 text-muted-foreground"
  }
}

const STATUS_LEVELS: Record<string, StatusLevel> = {
  online: "success",
  connected: "success",
  ready: "success",
  running: "success",
  strong: "success",
  verified: "success",
  active: "success",
  success: "success",
  "read-write": "success",
  read: "success",
  low: "success",
  resolved: "success",
  checking: "warning",
  limited: "warning",
  review: "warning",
  draft: "warning",
  "ask-every-time": "warning",
  medium: "warning",
  warning: "warning",
  info: "info",
  offline: "error",
  blocked: "error",
  failed: "error",
  revoked: "error",
  danger: "error",
  error: "error",
  critical: "error",
  none: "error",
  high: "error",
  cancelled: "neutral",
  pending: "neutral",
  idle: "neutral",
}

export function statusClass(value: string): string {
  return statusClassForLevel(STATUS_LEVELS[value] ?? "neutral")
}

export function cnStatus(additionalClasses: string, value: string): string {
  return cn(statusClass(value), additionalClasses)
}
