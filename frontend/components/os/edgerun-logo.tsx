import { cn } from "@/lib/utils"

interface EdgerunLogoProps {
  className?: string
  /** "mark" shows just the icon, "full" shows icon + wordmark */
  variant?: "mark" | "full"
  size?: "sm" | "md" | "lg"
}

export function EdgerunLogo({ className, variant = "full", size = "md" }: EdgerunLogoProps) {
  const sizes = {
    sm: { mark: 20, text: "text-sm", gap: "gap-1.5" },
    md: { mark: 26, text: "text-base", gap: "gap-2" },
    lg: { mark: 40, text: "text-2xl", gap: "gap-3" },
  }

  const s = sizes[size]

  return (
    <div className={cn("flex items-center", s.gap, className)}>
      {/* Mark: 4-node distributed network that reads as a stylized E */}
      <svg
        width={s.mark}
        height={s.mark}
        viewBox="0 0 32 32"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        aria-hidden="true"
      >
        {/* Outer glow ring — subtle */}
        <circle cx="16" cy="16" r="15" stroke="currentColor" strokeOpacity="0.08" strokeWidth="1" />

        {/* Connection lines — form the skeleton of an E rotated */}
        {/* Top horizontal: node A(4,6) → node B(26,6) */}
        <line x1="6" y1="7" x2="24" y2="7" stroke="currentColor" strokeOpacity="0.35" strokeWidth="1" strokeLinecap="round" />
        {/* Middle horizontal: node A(4,16) → node C(20,16) */}
        <line x1="6" y1="16" x2="22" y2="16" stroke="currentColor" strokeOpacity="0.35" strokeWidth="1" strokeLinecap="round" />
        {/* Bottom horizontal: node A(4,25) → node D(26,25) */}
        <line x1="6" y1="25" x2="24" y2="25" stroke="currentColor" strokeOpacity="0.35" strokeWidth="1" strokeLinecap="round" />
        {/* Vertical spine: A-top(4,6) → A-mid(4,16) → A-bot(4,25) */}
        <line x1="6" y1="7" x2="6" y2="25" stroke="currentColor" strokeOpacity="0.35" strokeWidth="1" strokeLinecap="round" />

        {/* Diagonal accent lines — speed/edge feel */}
        <line x1="22" y1="16" x2="24" y2="7" stroke="currentColor" strokeOpacity="0.2" strokeWidth="0.8" strokeLinecap="round" strokeDasharray="1.5 2" />
        <line x1="22" y1="16" x2="24" y2="25" stroke="currentColor" strokeOpacity="0.2" strokeWidth="0.8" strokeLinecap="round" strokeDasharray="1.5 2" />

        {/* Node dots */}
        {/* Spine node — top */}
        <circle cx="6" cy="7" r="2.5" fill="currentColor" fillOpacity="0.5" />
        {/* Top-right node */}
        <circle cx="24" cy="7" r="2" fill="currentColor" fillOpacity="0.6" />
        {/* Mid-right node — primary, brighter */}
        <circle cx="22" cy="16" r="3" fill="currentColor" />
        {/* Spine node — mid */}
        <circle cx="6" cy="16" r="2.5" fill="currentColor" fillOpacity="0.5" />
        {/* Bottom-right node */}
        <circle cx="24" cy="25" r="2" fill="currentColor" fillOpacity="0.6" />
        {/* Spine node — bottom */}
        <circle cx="6" cy="25" r="2.5" fill="currentColor" fillOpacity="0.5" />

        {/* Pulse ring on the primary node */}
        <circle cx="22" cy="16" r="5.5" stroke="currentColor" strokeOpacity="0.25" strokeWidth="1" />
      </svg>

      {variant === "full" && (
        <div className="flex flex-col leading-none">
          <span
            className={cn(
              "font-mono font-bold tracking-widest text-foreground uppercase",
              s.text
            )}
          >
            Edgerun
          </span>
          {size !== "sm" && (
            <span className="font-mono text-[9px] font-medium tracking-[0.25em] text-primary uppercase opacity-80">
              Distributed Runtime
            </span>
          )}
        </div>
      )}
    </div>
  )
}
