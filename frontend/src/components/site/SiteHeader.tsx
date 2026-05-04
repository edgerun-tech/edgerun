import type { ComponentProps } from "solid-js"
import { For, splitProps } from "solid-js"
import type { Locale } from "@/data/i18n"

type NavProps = {
  locale: Locale
  homeLabel: string
  methodologyLabel: string
  benchmarksLabel: string
}

export const SiteHeader = (props: NavProps) => {
  const links = [
    { href: `/${props.locale}`, label: props.homeLabel },
    {
      href: `/${props.locale}/methodology`,
      label: props.methodologyLabel,
    },
    { href: `/${props.locale}/benchmarks`, label: props.benchmarksLabel },
  ]

  return (
    <header class="sticky top-0 z-40 border-b border-white/10 bg-slate-950/70 backdrop-blur">
      <nav class="mx-auto flex max-w-6xl items-center justify-between gap-3 px-4 py-4 sm:px-6">
        <a href="/" class="inline-flex items-center gap-2 font-medium tracking-wide text-white">
          <span class="size-2 rounded-full bg-cyan-300" />
          EdgeRun Benchmarks
        </a>
        <div class="flex flex-wrap items-center gap-1 sm:gap-2">
          <For each={links}>
            {(item) => <a class="rounded-md px-3 py-2 text-sm text-slate-200 hover:text-white" href={item.href}>{item.label}</a>}
          </For>
        </div>
      </nav>
    </header>
  )
}

type LocaleSwitcherProps = {
  current: Locale
}
export const LocaleSwitcher = (props: LocaleSwitcherProps & ComponentProps<"div">) => {
  const [local, rest] = splitProps(props, ["current", "class"])
  return (
    <div class={local.class} {...rest}>
      <a
        href="/en"
        class={local.current === "en" ? "text-white font-semibold" : "text-slate-300"}
      >
        English
      </a>
      <span class="mx-2 text-slate-500">•</span>
      <a
        href="/et"
        class={local.current === "et" ? "text-white font-semibold" : "text-slate-300"}
      >
        Eesti
      </a>
      <span class="mx-2 text-slate-500">•</span>
      <a
        href="/th"
        class={local.current === "th" ? "text-white font-semibold" : "text-slate-300"}
      >
        ไทย
      </a>
    </div>
  )
}
