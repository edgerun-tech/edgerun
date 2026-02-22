// SPDX-License-Identifier: Apache-2.0
import { cx } from '../../lib/ui-theme'

export function Skeleton(props: any) {
  return (
    <div
      aria-hidden="true"
      {...props}
      class={cx('animate-pulse rounded-md bg-muted/60', props.class)}
    />
  )
}
