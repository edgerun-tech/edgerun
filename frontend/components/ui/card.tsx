// SPDX-License-Identifier: Apache-2.0
import { cx, uiTheme } from '../../lib/ui-theme'

export function Card(props: any) {
  return <div {...props} class={cx(uiTheme.surfaces.card, props.class)}>{props.children}</div>
}

export function CardHeader(props: any) {
  return <div {...props} class={cx('p-4 md:p-5', props.class)}>{props.children}</div>
}

export function CardContent(props: any) {
  return <div {...props} class={cx('px-4 pb-4 md:px-5 md:pb-5', props.class)}>{props.children}</div>
}

export function CardTitle(props: any) {
  return <h3 {...props} class={cx('text-lg font-semibold', props.class)}>{props.children}</h3>
}

export function CardDescription(props: any) {
  return <p {...props} class={cx('text-sm text-muted-foreground', props.class)}>{props.children}</p>
}
