import { createContext, createMemo, createSignal, splitProps, useContext } from 'solid-js'
import { cx, uiTheme } from '../../lib/ui-theme'

type TabsContextValue = {
  value: () => string
  setValue: (next: string) => void
}

const TabsContext = createContext<TabsContextValue>()

export function Tabs(props: { defaultValue?: string; value?: string; onValueChange?: (next: string) => void; class?: string; children?: any } & Record<string, any>) {
  const [local, rest] = splitProps(props, ['defaultValue', 'value', 'onValueChange', 'class', 'children'])
  const [internalValue, setInternalValue] = createSignal(local.defaultValue ?? '')
  const value = createMemo(() => local.value ?? internalValue())
  const controlled = createMemo(() => local.value !== undefined)
  const setValue = (next: string) => {
    if (!controlled()) setInternalValue(next)
    local.onValueChange?.(next)
  }
  return (
    <TabsContext.Provider value={{ value, setValue }}>
      <div {...rest} class={cx('space-y-4', local.class)}>{local.children}</div>
    </TabsContext.Provider>
  )
}

export function TabsList(props: any) {
  return <div {...props} role="tablist" class={cx('inline-flex h-10 items-center rounded-md border border-border bg-card p-1', props.class)}>{props.children}</div>
}

export function TabsTrigger(props: { value: string; class?: string; children?: any } & Record<string, any>) {
  const [local, rest] = splitProps(props, ['value', 'class', 'children'])
  const ctx = useContext(TabsContext) || { value: () => '', setValue: () => undefined }
  const active = () => ctx.value() === local.value
  return (
    <button
      {...rest}
      type="button"
      role="tab"
      aria-selected={active()}
      onClick={() => ctx.setValue(local.value)}
      class={cx(
        uiTheme.focusRing,
        'inline-flex items-center justify-center rounded-sm px-3 py-1.5 text-sm transition-colors',
        active() ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground',
        local.class
      )}
    >
      {local.children}
    </button>
  )
}

export function TabsContent(props: { value: string; class?: string; children?: any } & Record<string, any>) {
  const [local, rest] = splitProps(props, ['value', 'class', 'children'])
  const ctx = useContext(TabsContext) || { value: () => '', setValue: () => undefined }
  return (
    <div
      {...rest}
      role="tabpanel"
      hidden={ctx.value() !== local.value}
      class={cx('rounded-lg border border-border bg-card p-4 md:p-5', local.class)}
    >
      {local.children}
    </div>
  )
}
