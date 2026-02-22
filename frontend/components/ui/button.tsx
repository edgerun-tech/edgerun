// SPDX-License-Identifier: Apache-2.0
import { splitProps } from 'solid-js'
import { cx, uiTheme } from '../../lib/ui-theme'

type Variant = 'default' | 'outline' | 'ghost'
type Size = 'sm' | 'md' | 'lg'
type ExtendedVariant = Variant | 'secondary' | 'destructive'

type ButtonProps = {
  variant?: ExtendedVariant
  size?: Size
  type?: 'button' | 'submit' | 'reset'
  class?: string
  children?: any
} & Record<string, any>

export function Button(props: ButtonProps) {
  const [local, rest] = splitProps(props, ['variant', 'size', 'type', 'class', 'children'])
  const variant = () => local.variant || 'default'
  const size = () => local.size || 'md'
  const type = () => local.type || 'button'

  return (
    <button
      {...rest}
      type={type()}
      class={cx(
        uiTheme.button.base,
        uiTheme.focusRing,
        uiTheme.button.variant[variant()],
        uiTheme.button.size[size()],
        local.class
      )}
    >
      {local.children}
    </button>
  )
}
