import { cx, uiTheme } from '../../lib/ui-theme'

export function Select(props: any) {
  return (
    <select
      {...props}
      class={cx(uiTheme.input.base, uiTheme.input.active, 'pr-8', props.class)}
    >
      {props.children}
    </select>
  )
}
