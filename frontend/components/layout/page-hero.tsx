// SPDX-License-Identifier: Apache-2.0
import { Show, type JSX } from 'solid-js'
import { Badge } from '../ui/badge'

type PageHeroProps = {
  title: string
  description: string
  badge?: string
  badgeVariant?: 'default' | 'secondary' | 'outline' | 'destructive'
  maxWidthClass?: string
  actions?: JSX.Element
}

export function PageHero(props: PageHeroProps) {
  const maxWidthClass = () => props.maxWidthClass || 'max-w-7xl'

  return (
    <section class="border-b border-border bg-card">
      <div class={`mx-auto px-4 py-10 sm:px-6 lg:px-8 sm:py-12 ${maxWidthClass()}`}>
        <div class="flex flex-wrap items-center gap-2">
          <h1 class="text-3xl font-bold sm:text-4xl">{props.title}</h1>
          <Show when={props.badge}><Badge variant={props.badgeVariant}>{props.badge}</Badge></Show>
        </div>
        <p class="mt-3 max-w-3xl text-lg text-muted-foreground">{props.description}</p>
        <Show when={props.actions}><div class="mt-5 flex flex-wrap items-center gap-3">{props.actions}</div></Show>
      </div>
    </section>
  )
}
