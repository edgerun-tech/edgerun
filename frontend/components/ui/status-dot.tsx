import { cn } from "@/lib/utils"

export interface StatusDotProps {
  className?: string
  onClick?: () => void
  size?: "sm" | "md"
}

export function StatusDot({ className, onClick, size = "sm" }: StatusDotProps) {
  const dot = (
    <span
      className={cn(
        "rounded-full",
        size === "md" ? "h-2.5 w-2.5" : "h-2 w-2",
        className,
      )}
      aria-hidden={!onClick}
    />
  )

  if (onClick) {
    return (
      <button
        type="button"
        onClick={onClick}
        className="flex h-7 w-7 items-center justify-center rounded-full text-muted-foreground transition-colors hover:text-foreground"
        aria-label={`Status: ${className ?? "unknown"}`}
      >
        {dot}
      </button>
    )
  }

  return dot
}
