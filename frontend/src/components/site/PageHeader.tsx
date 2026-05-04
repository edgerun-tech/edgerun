import type { ComponentProps } from "solid-js"
import { splitProps } from "solid-js"
import { cn } from "@/lib/cx"

export const PageHeader = (props: ComponentProps<"section">) => {
  const [, rest] = splitProps(props, ["class", "children"])
  return (
    <section class={cn("border-b border-white/10", props.class)} {...rest}>
      <div class="mx-auto flex max-w-6xl flex-col items-center gap-3 px-6 py-14 text-center sm:py-16 lg:py-20">
        {props.children}
      </div>
    </section>
  )
}

export const PageHeaderHeading = (props: ComponentProps<"h1">) => {
  const [, rest] = splitProps(props, ["class"])
  return <h1 class={cn("text-balance text-3xl font-bold tracking-tight text-white sm:text-5xl", props.class)} {...rest} />
}

export const PageHeaderDescription = (props: ComponentProps<"p">) => {
  const [, rest] = splitProps(props, ["class"])
  return <p class={cn("max-w-3xl text-base text-slate-300 sm:text-lg", props.class)} {...rest} />
}
