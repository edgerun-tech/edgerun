import type { ComponentProps } from "solid-js"
import { splitProps } from "solid-js"
import { cn } from "@/lib/cx"

const base =
  "inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-300 disabled:pointer-events-none disabled:opacity-50"

const variants = {
  default:
    "bg-indigo-500 text-white hover:bg-indigo-400 shadow-md",
  ghost: "text-slate-100 hover:bg-white/10 border border-white/10",
  secondary:
    "bg-slate-700 text-slate-50 hover:bg-slate-600",
  outline:
    "border border-slate-400/50 text-slate-100 hover:bg-slate-400/10",
  link: "text-cyan-200 underline-offset-4 hover:underline",
}

const sizes = {
  default: "h-10 px-4 py-2",
  sm: "h-8 px-3 text-xs",
  lg: "h-11 px-6 text-sm",
  icon: "h-9 w-9",
}

export type ButtonVariant = keyof typeof variants
export type ButtonSize = keyof typeof sizes
export type ButtonProps = Omit<ComponentProps<"button">, "size"> & {
  variant?: ButtonVariant
  size?: ButtonSize
}

export const Button = (props: ButtonProps) => {
  const [local, rest] = splitProps(props, ["class", "variant", "size"])
  const variant = local.variant ?? "default"
  const size = local.size ?? "default"

  return (
    <button
      class={cn(base, variants[variant], sizes[size], local.class)}
      {...rest}
    />
  )
}
