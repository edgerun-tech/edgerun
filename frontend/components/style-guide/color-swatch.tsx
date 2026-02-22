// SPDX-License-Identifier: Apache-2.0
export function ColorSwatch(props: { name: string; variable: string; value: string; textColor?: string }) {
  return (
    <div class="space-y-2">
      <div
        class={`h-24 rounded-lg border border-border flex items-center justify-center ${props.textColor ?? 'text-foreground'}`}
        style={{ 'background-color': `var(${props.variable})` }}
      >
        <span class="text-sm font-mono">{props.variable}</span>
      </div>
      <div>
        <p class="text-sm font-medium">{props.name}</p>
        <p class="text-xs font-mono text-muted-foreground">{props.value}</p>
      </div>
    </div>
  )
}
