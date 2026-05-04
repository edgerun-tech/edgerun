"use client"

import type { ReactNode } from "react"
import { GlowingEffect } from "@/components/ui/glowing-effect"
import { cn } from "@/lib/utils"

type GlowingContainerProps = {
  children: ReactNode
  className?: string
  contentClassName?: string
  glow?: boolean
  proximity?: number
  spread?: number
  borderWidth?: number
  disabled?: boolean
}

export function GlowingContainer({
  children,
  className,
  contentClassName,
  glow = true,
  proximity = 64,
  spread = 80,
  borderWidth = 3,
  disabled = false,
}: GlowingContainerProps) {
  return (
    <div className={cn("relative rounded-2xl border p-2 md:rounded-3xl md:p-3", className)}>
      <GlowingEffect
        blur={0}
        borderWidth={borderWidth}
        spread={spread}
        glow={glow}
        disabled={disabled}
        proximity={proximity}
        inactiveZone={0.01}
      />
      <div
        className={cn(
          "relative h-full overflow-hidden rounded-xl border-0 bg-background/70 backdrop-blur-md dark:shadow-[0px_0px_27px_0px_#2D2D2D]",
          contentClassName,
        )}
      >
        {children}
      </div>
    </div>
  )
}

type GlowingGridItemProps = GlowingContainerProps & {
  area?: string
}

export function GlowingGridItem({ area, className, ...props }: GlowingGridItemProps) {
  return (
    <li className={cn("list-none", area)}>
      <GlowingContainer className={className} {...props} />
    </li>
  )
}
