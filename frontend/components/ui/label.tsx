import { cx, uiTheme } from '../../lib/ui-theme'

export function Label(props: any) {
  return (
    <label
      {...props}
      class={cx(uiTheme.label.base, props.class)}
    >
      {props.children}
    </label>
  )
}
