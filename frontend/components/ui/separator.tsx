// SPDX-License-Identifier: Apache-2.0
import { splitProps } from 'solid-js'
import { cx, uiTheme } from '../../lib/ui-theme'

type SeparatorProps = {
  orientation?: 'horizontal' | 'vertical'
  class?: string
} & Record<string, any>

export function Separator(props: SeparatorProps) {
  const [local, rest] = splitProps(props, ['orientation', 'class'])
  const orientation = () => local.orientation || 'horizontal'
  return (
    <div
      role="separator"
      aria-orientation={orientation()}
      {...rest}
      class={cx(orientation() === 'vertical' ? uiTheme.separator.vertical : uiTheme.separator.horizontal, local.class)}
    />
  )
}
