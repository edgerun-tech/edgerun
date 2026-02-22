// SPDX-License-Identifier: Apache-2.0
import { createContext, createSignal, splitProps, useContext } from 'solid-js'
import { cx, uiTheme } from '../../lib/ui-theme'

type AccordionContextValue = {
  isOpen: (item: string) => boolean
  toggle: (item: string) => void
}

const AccordionContext = createContext<AccordionContextValue>()

export function Accordion(props: { class?: string; children?: any } & Record<string, any>) {
  const [local, rest] = splitProps(props, ['class', 'children'])
  const [openItem, setOpenItem] = createSignal<string | null>(null)
  return (
    <AccordionContext.Provider
      value={{
        isOpen: (item) => openItem() === item,
        toggle: (item) => setOpenItem((prev) => (prev === item ? null : item))
      }}
    >
      <div {...rest} class={cx('space-y-2', local.class)}>{local.children}</div>
    </AccordionContext.Provider>
  )
}

export function AccordionItem(props: { value: string; class?: string; children?: any } & Record<string, any>) {
  const [local, rest] = splitProps(props, ['class', 'children'])
  return <section {...rest} class={cx('overflow-hidden rounded-lg border border-border bg-card', local.class)}>{local.children}</section>
}

export function AccordionTrigger(props: { value: string; class?: string; children?: any } & Record<string, any>) {
  const [local, rest] = splitProps(props, ['value', 'class', 'children'])
  const ctx = useContext(AccordionContext) || { isOpen: () => false, toggle: () => undefined }
  const open = () => ctx.isOpen(local.value)
  return (
    <button
      {...rest}
      type="button"
      aria-expanded={open()}
      onClick={() => ctx.toggle(local.value)}
      class={cx(
        uiTheme.focusRing,
        'flex w-full items-center justify-between px-4 py-3 text-left text-sm font-medium',
        local.class
      )}
    >
      <span>{local.children}</span>
      <span aria-hidden="true" class={cx('transition-transform', open() ? 'rotate-45' : '')}>+</span>
    </button>
  )
}

export function AccordionContent(props: { value: string; class?: string; children?: any } & Record<string, any>) {
  const [local, rest] = splitProps(props, ['value', 'class', 'children'])
  const ctx = useContext(AccordionContext) || { isOpen: () => false, toggle: () => undefined }
  return (
    <div
      {...rest}
      hidden={!ctx.isOpen(local.value)}
      class={cx('border-t border-border px-4 py-3 text-sm text-muted-foreground', local.class)}
    >
      {local.children}
    </div>
  )
}
