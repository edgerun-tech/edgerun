import { splitProps } from 'solid-js'
import { cx, uiTheme } from '../../lib/ui-theme'

type BadgeVariant = 'default' | 'secondary' | 'outline' | 'destructive'

export function Badge(props: { variant?: BadgeVariant; class?: string; children?: any } & Record<string, any>) {
  const [local, rest] = splitProps(props, ['variant', 'class', 'children'])
  const variant = () => local.variant || 'default'

  return (
    <span
      {...rest}
      class={cx(
        uiTheme.badge.base,
        uiTheme.badge.variant[variant()],
        local.class
      )}
    >
      {local.children}
    </span>
  )
}
