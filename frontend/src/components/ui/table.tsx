import type { ComponentProps } from "solid-js"
import { splitProps } from "solid-js"
import { cn } from "@/lib/cx"

export const Table = (props: ComponentProps<"table">) => {
  const [local, rest] = splitProps(props, ["class"])
  return (
    <div class="overflow-x-auto">
      <table class={cn("w-full border-collapse text-sm", local.class)} {...rest} />
    </div>
  )
}

export const TableHead = (props: ComponentProps<"thead">) => {
  const [local, rest] = splitProps(props, ["class"])
  return <thead class={cn("text-left text-slate-200/80", local.class)} {...rest} />
}

export const TableBody = (props: ComponentProps<"tbody">) => {
  const [local, rest] = splitProps(props, ["class"])
  return <tbody class={cn("text-slate-100", local.class)} {...rest} />
}

export const TableRow = (props: ComponentProps<"tr">) => {
  const [local, rest] = splitProps(props, ["class"])
  return (
    <tr
      class={cn("border-b border-white/10 transition-colors hover:bg-white/5", local.class)}
      {...rest}
    />
  )
}

export const TableHeadCell = (props: ComponentProps<"th">) => {
  const [local, rest] = splitProps(props, ["class"])
  return (
    <th
      class={cn("whitespace-nowrap px-3 py-2 text-left text-xs font-semibold uppercase tracking-wide", local.class)}
      {...rest}
    />
  )
}

export const TableCell = (props: ComponentProps<"td">) => {
  const [local, rest] = splitProps(props, ["class"])
  return (
    <td
      class={cn("px-3 py-2 align-middle", local.class)}
      {...rest}
    />
  )
}
