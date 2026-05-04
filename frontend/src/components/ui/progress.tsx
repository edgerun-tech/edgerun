import type { ComponentProps } from "solid-js"
import { splitProps } from "solid-js"
import { cn } from "@/lib/cx"

export const Progress = (props: { value: number; class?: string }) => {
  const percent = Math.max(0, Math.min(100, Math.round(props.value)))

  return (
    <div class={cn("h-2 w-full rounded-full bg-slate-700/70", props.class)}>
      <div
        class="h-full rounded-full bg-gradient-to-r from-indigo-500 via-cyan-300 to-emerald-300 transition-all"
        style={{ width: `${percent}%` }}
      />
    </div>
  )
}

export const ProgressBlock = (props: ComponentProps<"div">) => {
  const [local, rest] = splitProps(props, ["children", "class"])
  return <div class={cn("space-y-1", local.class)} {...rest} />
}
