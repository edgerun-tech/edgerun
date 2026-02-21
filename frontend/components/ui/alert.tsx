import { splitProps } from 'solid-js'
import { cx, uiTheme } from '../../lib/ui-theme'

type AlertVariant = 'default' | 'destructive'

export function Alert(props: { variant?: AlertVariant; class?: string; children?: any } & Record<string, any>) {
  const [local, rest] = splitProps(props, ['variant', 'class', 'children'])
  const variant = () => local.variant || 'default'
  return (
    <div
      role="alert"
      {...rest}
      class={cx(uiTheme.alert.base, uiTheme.alert.variant[variant()], local.class)}
    >
      {local.children}
    </div>
  )
}

export function AlertTitle(props: any) {
  return <h5 {...props} class={cx('mb-1 font-semibold', props.class)}>{props.children}</h5>
}

export function AlertDescription(props: any) {
  return <div {...props} class={cx('text-sm text-muted-foreground', props.class)}>{props.children}</div>
}
