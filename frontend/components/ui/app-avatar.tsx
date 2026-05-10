import { cn } from "@/lib/utils"

type AppAvatarSize = "xs" | "sm" | "md" | "lg"

interface AppAvatarProps {
  name: string
  size?: AppAvatarSize
  pulse?: boolean
}

function getInitials(name: string): string {
  if (name === "You") return "ME"
  return name.split(" ").map((n) => n[0]).join("").slice(0, 2).toUpperCase() || "ID"
}

function getHue(name: string): number {
  if (name === "You") return 145
  return name.split("").reduce((a, c) => a + c.charCodeAt(0), 0) % 360
}

const sizeClasses: Record<AppAvatarSize, string> = {
  xs: "h-6 w-6 text-[9px]",
  sm: "h-8 w-8 text-xs",
  md: "h-10 w-10 text-xs",
  lg: "h-20 w-20 text-2xl",
}

function AppAvatar({ name, size = "sm", pulse }: AppAvatarProps) {
  const initials = getInitials(name)
  const hue = getHue(name)

  const avatar = (
    <div
      className={cn(
        "flex flex-shrink-0 items-center justify-center rounded-full font-mono font-bold",
        sizeClasses[size],
      )}
      style={{ background: `oklch(0.25 0.1 ${hue})`, color: `oklch(0.85 0.1 ${hue})` }}
    >
      {initials}
    </div>
  )

  if (pulse) {
    return (
      <div className="relative flex items-center justify-center">
        <div
          className="absolute h-28 w-28 animate-ping rounded-full opacity-20"
          style={{ background: `oklch(0.65 0.2 145)` }}
        />
        <div
          className="absolute h-24 w-24 animate-ping rounded-full opacity-10"
          style={{ animationDelay: "0.3s", background: `oklch(0.65 0.2 145)` }}
        />
        <div className="relative">{avatar}</div>
      </div>
    )
  }

  return avatar
}

export { AppAvatar, type AppAvatarSize }
