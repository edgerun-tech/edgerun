import { cx, uiTheme } from '../../lib/ui-theme'

export function Checkbox(props: any) {
  return (
    <input
      type="checkbox"
      {...props}
      class={cx(
        'h-4 w-4 rounded border-input bg-background text-primary',
        uiTheme.focusRing,
        props.class
      )}
    />
  )
}
