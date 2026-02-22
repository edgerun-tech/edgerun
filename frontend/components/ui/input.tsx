// SPDX-License-Identifier: Apache-2.0
import { cx, uiTheme } from '../../lib/ui-theme'

export function Input(props: any) {
  return (
    <input
      {...props}
      class={cx(uiTheme.input.base, uiTheme.input.active, props.class)}
    />
  )
}
