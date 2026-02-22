// SPDX-License-Identifier: Apache-2.0
export function GeneratingIndicator(props: any) {
  return <span {...props} class={`status-generating ${props.class ?? ''}`} data-generating-label>Generating</span>
}
