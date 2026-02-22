// SPDX-License-Identifier: Apache-2.0
import { cx, uiTheme } from '../../lib/ui-theme'

export function Textarea(props: any) {
  return (
    <textarea
      {...props}
      class={cx(uiTheme.textarea.base, uiTheme.textarea.active, props.class)}
    />
  )
}
