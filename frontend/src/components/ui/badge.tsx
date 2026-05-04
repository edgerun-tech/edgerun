import type { ComponentProps } from "solid-js"
import { splitProps } from "solid-js"
import { cn } from "@/lib/cx"

const variants = {
  default: "bg-indigo-500/90 text-white",
  secondary: "bg-slate-700 text-slate-100",
  outline: "bg-transparent border border-cyan-200/30 text-cyan-100",
}

export type BadgeProps = Omit<ComponentProps<"span">, "children"> & {
  tone?: keyof typeof variants
}

export const Badge = (props: BadgeProps) => {
  const [local, rest] = splitProps(props, ["class", "tone"])
  return (
    <span
      class={cn(
        "inline-flex items-center rounded-full px-2.5 py-1 text-xs font-semibold",
        variants[local.tone ?? "default"],
        local.class,
      )}
      {...rest}
    />
  )
}
