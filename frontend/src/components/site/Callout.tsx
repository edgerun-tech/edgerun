import type { ComponentProps } from "solid-js"
import { splitProps } from "solid-js"
import { cn } from "@/lib/cx"

export const Callout = (props: ComponentProps<"section">) => {
  const [local, rest] = splitProps(props, ["class"])
  return (
    <section class={cn("rounded-2xl glass-panel border-l-4 border-cyan-300/80 p-4", local.class)} {...rest}>
      <div class="space-y-2 text-sm text-slate-100">{props.children}</div>
    </section>
  )
}
