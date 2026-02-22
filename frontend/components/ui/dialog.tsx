// SPDX-License-Identifier: Apache-2.0
import { createContext, Show, splitProps, useContext } from 'solid-js'
import { Portal } from 'solid-js/web'
import { cx, uiTheme } from '../../lib/ui-theme'

type DialogContextValue = {
  open: () => boolean
  setOpen: (next: boolean) => void
}

const DialogContext = createContext<DialogContextValue>()

export function Dialog(props: { open: boolean; onOpenChange: (next: boolean) => void; children?: any }) {
  const setOpen = (next: boolean) => props.onOpenChange(next)
  return (
    <DialogContext.Provider value={{ open: () => props.open, setOpen }}>
      {props.children}
    </DialogContext.Provider>
  )
}

export function DialogTrigger(props: any) {
  const [local, rest] = splitProps(props, ['class', 'children'])
  const ctx = useContext(DialogContext) || { open: () => false, setOpen: () => undefined }
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

export function DialogContent(props: any) {
  const [local, rest] = splitProps(props, ['class', 'children'])
  const ctx = useContext(DialogContext) || { open: () => false, setOpen: () => undefined }
  return (
    <Portal>
      <Show when={ctx.open()}>
        <div class="fixed inset-0 z-[120]">
          <button
            type="button"
            aria-label="Close dialog"
            class="absolute inset-0 bg-black/60"
            onClick={() => ctx.setOpen(false)}
          />
          <div class="absolute inset-0 flex items-start justify-center overflow-y-auto p-4">
            <section
              role="dialog"
              aria-modal="true"
              {...rest}
              class={cx('my-4 w-full max-w-xl rounded-lg border border-border bg-card p-5 shadow-xl max-h-[calc(100dvh-2rem)] overflow-y-auto', local.class)}
            >
              {local.children}
            </section>
          </div>
        </div>
      </Show>
    </Portal>
  )
}

export function DialogHeader(props: any) {
  return <header {...props} class={cx('mb-3 space-y-1', props.class)}>{props.children}</header>
}

export function DialogTitle(props: any) {
  return <h2 {...props} class={cx('text-lg font-semibold', props.class)}>{props.children}</h2>
}

export function DialogDescription(props: any) {
  return <p {...props} class={cx('text-sm text-muted-foreground', props.class)}>{props.children}</p>
}

export function DialogFooter(props: any) {
  return <footer {...props} class={cx('mt-4 flex justify-end gap-2', props.class)}>{props.children}</footer>
}

export function DialogClose(props: any) {
  const [local, rest] = splitProps(props, ['class', 'children'])
  const ctx = useContext(DialogContext) || { open: () => false, setOpen: () => undefined }
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
