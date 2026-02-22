// SPDX-License-Identifier: Apache-2.0
import { For } from 'solid-js'

const spacingScale = [
  { size: '0', rem: '0', px: '0px' },
  { size: '1', rem: '0.25rem', px: '4px' },
  { size: '2', rem: '0.5rem', px: '8px' },
  { size: '3', rem: '0.75rem', px: '12px' },
  { size: '4', rem: '1rem', px: '16px' },
  { size: '5', rem: '1.25rem', px: '20px' },
  { size: '6', rem: '1.5rem', px: '24px' },
  { size: '8', rem: '2rem', px: '32px' },
  { size: '10', rem: '2.5rem', px: '40px' },
  { size: '12', rem: '3rem', px: '48px' },
  { size: '16', rem: '4rem', px: '64px' }
]

export function SpacingDemo() {
  return (
    <div class="space-y-6">
      <div class="space-y-2">
        <h4 class="text-lg font-semibold">Spacing Scale</h4>
        <p class="text-sm text-muted-foreground">Use Tailwind spacing utilities (`p-*`, `m-*`, `gap-*`) for consistent spacing.</p>
      </div>
      <div class="space-y-4">
        <For each={spacingScale}>{(item: (typeof spacingScale)[number]) => (
          <div class="flex items-center gap-4">
            <div class="w-16 text-sm font-mono text-muted-foreground">{item.size}</div>
            <div class="flex-1"><div class="h-8 rounded bg-primary" style={{ width: item.rem }} /></div>
            <div class="w-24 text-right text-sm text-muted-foreground">{item.rem}</div>
            <div class="w-16 text-right text-xs font-mono text-muted-foreground">{item.px}</div>
          </div>
        )}</For>
      </div>
      <div class="space-y-2 rounded-lg border border-border bg-card p-4">
        <p class="text-sm font-semibold">Common Patterns:</p>
        <ul class="space-y-1 text-sm text-muted-foreground">
          <li>
            <code class="rounded bg-muted px-1 py-0.5 text-xs">gap-4</code> - Default gap between flex/grid items
          </li>
          <li>
            <code class="rounded bg-muted px-1 py-0.5 text-xs">p-6</code> - Card padding
          </li>
          <li>
            <code class="rounded bg-muted px-1 py-0.5 text-xs">space-y-4</code> - Vertical spacing between elements
          </li>
          <li>
            <code class="rounded bg-muted px-1 py-0.5 text-xs">mb-8</code> - Section bottom margin
          </li>
        </ul>
      </div>
    </div>
  )
}
