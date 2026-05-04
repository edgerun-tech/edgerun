import type { ComponentProps } from "solid-js"
import { splitProps } from "solid-js"
import { cn } from "@/lib/cx"

export const Card = (props: ComponentProps<"article">) => {
  const [local, rest] = splitProps(props, ["class"])
  return <article class={cn("rounded-2xl border border-white/12 bg-slate-900/80 p-5 shadow-lg shadow-black/30", local.class)} {...rest} />
}

export const CardHeader = (props: ComponentProps<"div">) => {
  const [local, rest] = splitProps(props, ["class"])
  return <div class={cn("mb-4 space-y-2", local.class)} {...rest} />
}

export const CardTitle = (props: ComponentProps<"h3">) => {
  const [local, rest] = splitProps(props, ["class"])
  return <h3 class={cn("text-lg font-semibold text-white", local.class)} {...rest} />
}

export const CardDescription = (props: ComponentProps<"p">) => {
  const [local, rest] = splitProps(props, ["class"])
  return <p class={cn("text-sm text-slate-300 leading-relaxed", local.class)} {...rest} />
}

export const CardContent = (props: ComponentProps<"div">) => {
  const [local, rest] = splitProps(props, ["class"])
  return <div class={cn("space-y-3", local.class)} {...rest} />
}
