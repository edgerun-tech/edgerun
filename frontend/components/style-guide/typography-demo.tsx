// SPDX-License-Identifier: Apache-2.0
export function TypographyDemo() {
  return (
    <div class="space-y-8">
      <div class="space-y-4">
        <h1 class="text-4xl font-bold text-balance">Heading 1 - Bold Display</h1>
        <h2 class="text-3xl font-bold text-balance">Heading 2 - Section Title</h2>
        <h3 class="text-2xl font-semibold text-balance">Heading 3 - Subsection</h3>
        <h4 class="text-xl font-semibold">Heading 4 - Card Title</h4>
        <h5 class="text-lg font-medium">Heading 5 - Small Heading</h5>
      </div>
      <div class="max-w-3xl space-y-4">
        <p class="text-base leading-relaxed">
          This is body text in the default size. It uses the Geist font family with relaxed line height for optimal readability. Body text should be clear and
          comfortable to read for extended periods.
        </p>
        <p class="text-sm leading-relaxed text-muted-foreground">
          This is secondary body text, often used for descriptions, captions, or less prominent information. It uses a muted foreground color to create visual hierarchy.
        </p>
      </div>
      <div class="space-y-2">
        <p class="font-mono text-sm text-primary">0x7f3a9b2c8e1d4f6a9c2b5e8d1a4f7c3b9e2a5d8c1f4a7b3e6d9c2a5f8e1b4d7</p>
        <p class="text-xs font-mono text-muted-foreground">Technical data like hashes, addresses, and code snippets use monospace font.</p>
      </div>
      <div class="space-y-2 rounded-lg border border-border bg-card p-4">
        <p class="text-sm font-semibold">Available Font Classes:</p>
        <ul class="space-y-1 text-sm text-muted-foreground">
          <li>
            <code class="rounded bg-muted px-1 py-0.5 text-xs">font-sans</code> - Default UI font (Geist Sans)
          </li>
          <li>
            <code class="rounded bg-muted px-1 py-0.5 text-xs">font-mono</code> - Monospace font (Geist Mono)
          </li>
        </ul>
      </div>
    </div>
  )
}
