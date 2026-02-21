import { createContext, Show, splitProps, useContext } from 'solid-js'
import { cx, uiTheme } from '../../lib/ui-theme'

type SheetContextValue = {
  open: () => boolean
  setOpen: (next: boolean) => void
}

const SheetContext = createContext<SheetContextValue>()

export function Sheet(props: { open: boolean; onOpenChange: (next: boolean) => void; children?: any }) {
  const setOpen = (next: boolean) => props.onOpenChange(next)
  return (
    <SheetContext.Provider value={{ open: () => props.open, setOpen }}>
      {props.children}
    </SheetContext.Provider>
  )
}

export function SheetTrigger(props: any) {
  const [local, rest] = splitProps(props, ['class', 'children'])
  const ctx = useContext(SheetContext) || { open: () => false, setOpen: () => undefined }
  return (
    <button
      {...rest}
      type="button"
      onClick={() => ctx.setOpen(true)}
      class={cx(uiTheme.focusRing, local.class)}
    >
      {local.children}
    </button>
  )
}

export function SheetContent(props: { side?: 'left' | 'right'; class?: string; children?: any }) {
  const [local] = splitProps(props, ['side', 'class', 'children'])
  const ctx = useContext(SheetContext) || { open: () => false, setOpen: () => undefined }
  const side = () => local.side || 'right'
  return (
    <Show when={ctx.open()}>
      <div class="fixed inset-0 z-[60]">
        <button
          type="button"
          aria-label="Close panel"
          class="absolute inset-0 bg-black/50"
          onClick={() => ctx.setOpen(false)}
        />
        <aside
          role="dialog"
          aria-modal="true"
          class={cx(
            'absolute top-0 h-full w-[86vw] max-w-sm border-border bg-card p-4 shadow-xl',
            side() === 'left' ? 'left-0 border-r' : 'right-0 border-l',
            local.class
          )}
        >
          {local.children}
        </aside>
      </div>
    </Show>
  )
}

export function SheetHeader(props: any) {
  return <div {...props} class={cx('mb-4 flex items-center justify-between gap-3', props.class)}>{props.children}</div>
}

export function SheetTitle(props: any) {
  return <h2 {...props} class={cx('text-lg font-semibold', props.class)}>{props.children}</h2>
}

export function SheetClose(props: any) {
  const [local, rest] = splitProps(props, ['class', 'children'])
  const ctx = useContext(SheetContext) || { open: () => false, setOpen: () => undefined }
  return (
    <button
      {...rest}
      type="button"
      onClick={() => ctx.setOpen(false)}
      class={cx(uiTheme.focusRing, local.class)}
    >
      {local.children}
    </button>
  )
}
