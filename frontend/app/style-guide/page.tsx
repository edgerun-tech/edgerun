// SPDX-License-Identifier: Apache-2.0
import { Nav } from '../../components/nav'
import { Footer } from '../../components/footer'
import { ColorSwatch } from '../../components/style-guide/color-swatch'
import { TypographyDemo } from '../../components/style-guide/typography-demo'
import { ComponentGallery } from '../../components/style-guide/component-gallery'
import { SpacingDemo } from '../../components/style-guide/spacing-demo'

export default function StyleGuidePage() {
  return (
    <div class="flex min-h-screen flex-col">
      <Nav />
      <main class="flex-1">
        <section class="border-b border-border bg-card">
          <div class="mx-auto max-w-7xl px-4 py-16 sm:px-6 lg:px-8">
            <h1 class="mb-4 text-4xl font-bold">Style Guide</h1>
            <p class="max-w-3xl text-lg text-muted-foreground">
              Complete design system and brand guidelines for Edgerun. This page serves as the single source of truth for colors, typography, components, and spacing.
            </p>
          </div>
        </section>

        <div class="mx-auto max-w-7xl space-y-16 px-4 py-12 sm:px-6 lg:px-8">
          <section id="colors">
            <h2 class="mb-6 text-3xl font-bold">Color Palette</h2>
            <p class="mb-8 text-muted-foreground">Edgerun uses a dark theme with purple and blue accents. All colors are defined as design tokens.</p>

            <div class="space-y-8">
              <div>
                <h3 class="mb-4 text-xl font-semibold">Backgrounds</h3>
                <div class="grid grid-cols-2 gap-4 md:grid-cols-4">
                  <ColorSwatch name="Background" variable="--background" value="oklch(0 0 0)" textColor="text-white" />
                  <ColorSwatch name="Card" variable="--card" value="oklch(0.12 0 0)" textColor="text-white" />
                  <ColorSwatch name="Popover" variable="--popover" value="oklch(0.15 0 0)" textColor="text-white" />
                  <ColorSwatch name="Muted" variable="--muted" value="oklch(0.20 0 0)" textColor="text-white" />
                </div>
              </div>

              <div>
                <h3 class="mb-4 text-xl font-semibold">Accent Colors</h3>
                <div class="grid grid-cols-2 gap-4 md:grid-cols-4">
                  <ColorSwatch name="Primary (Purple)" variable="--primary" value="oklch(0.65 0.22 285)" textColor="text-white" />
                  <ColorSwatch name="Accent (Blue)" variable="--accent" value="oklch(0.60 0.18 250)" textColor="text-white" />
                  <ColorSwatch name="Secondary" variable="--secondary" value="oklch(0.25 0 0)" textColor="text-white" />
                  <ColorSwatch name="Destructive (Red)" variable="--destructive" value="oklch(0.55 0.22 25)" textColor="text-white" />
                </div>
              </div>

              <div>
                <h3 class="mb-4 text-xl font-semibold">Text Colors</h3>
                <div class="grid grid-cols-2 gap-4 md:grid-cols-4">
                  <ColorSwatch name="Foreground" variable="--foreground" value="oklch(0.98 0 0)" textColor="text-black" />
                  <ColorSwatch name="Muted Foreground" variable="--muted-foreground" value="oklch(0.60 0 0)" textColor="text-white" />
                  <ColorSwatch name="Primary Foreground" variable="--primary-foreground" value="oklch(0.98 0 0)" textColor="text-black" />
                  <ColorSwatch name="Card Foreground" variable="--card-foreground" value="oklch(0.98 0 0)" textColor="text-black" />
                </div>
              </div>

              <div>
                <h3 class="mb-4 text-xl font-semibold">UI Elements</h3>
                <div class="grid grid-cols-2 gap-4 md:grid-cols-4">
                  <ColorSwatch name="Border" variable="--border" value="oklch(0.22 0 0)" textColor="text-white" />
                  <ColorSwatch name="Input" variable="--input" value="oklch(0.22 0 0)" textColor="text-white" />
                  <ColorSwatch name="Ring" variable="--ring" value="oklch(0.65 0.22 285)" textColor="text-white" />
                  <div class="rounded-lg border border-border bg-card p-4">
                    <p class="mb-2 text-sm font-semibold">Border Radius</p>
                    <p class="text-xs font-mono text-muted-foreground">--radius: 0.625rem</p>
                    <div class="mt-2 h-12 w-full bg-primary" style={{ 'border-radius': 'var(--radius)' }} />
                  </div>
                </div>
              </div>
            </div>
          </section>

          <hr class="border-border" />
          <section id="typography">
            <h2 class="mb-6 text-3xl font-bold">Typography</h2>
            <p class="mb-8 text-muted-foreground">Edgerun uses Geist for UI text and Geist Mono for technical data like hashes and code.</p>
            <TypographyDemo />
          </section>

          <hr class="border-border" />
          <section id="components">
            <h2 class="mb-6 text-3xl font-bold">Components</h2>
            <p class="mb-8 text-muted-foreground">Standard UI components built with shadcn/ui and customized for the Edgerun brand.</p>
            <ComponentGallery />
          </section>

          <hr class="border-border" />
          <section id="spacing">
            <h2 class="mb-6 text-3xl font-bold">Spacing</h2>
            <p class="mb-8 text-muted-foreground">Consistent spacing using Tailwind's spacing scale (4px base unit).</p>
            <SpacingDemo />
          </section>

          <hr class="border-border" />
          <section id="guidelines">
            <h2 class="mb-6 text-3xl font-bold">Usage Guidelines</h2>
            <div class="space-y-6">
              <div class="space-y-4 rounded-lg border border-border bg-card p-6">
                <h4 class="text-lg font-semibold">Brand Principles</h4>
                <ul class="space-y-2 text-muted-foreground">
                  <li class="flex gap-2">
                    <span class="text-primary">•</span>
                    <span><strong class="text-foreground">Professional:</strong> Clean, technical aesthetic that conveys trust and reliability</span>
                  </li>
                  <li class="flex gap-2">
                    <span class="text-primary">•</span>
                    <span><strong class="text-foreground">Dark-First:</strong> Pure black hero sections with near-black cards for depth</span>
                  </li>
                  <li class="flex gap-2">
                    <span class="text-primary">•</span>
                    <span><strong class="text-foreground">Monospace for Technical Data:</strong> Always use monospace font for hashes, addresses, and IDs</span>
                  </li>
                  <li class="flex gap-2">
                    <span class="text-primary">•</span>
                    <span><strong class="text-foreground">Subtle Accents:</strong> Purple primary and blue accent colors used sparingly for CTAs and highlights</span>
                  </li>
                  <li class="flex gap-2">
                    <span class="text-primary">•</span>
                    <span><strong class="text-foreground">No Crypto Hype:</strong> Avoid overly promotional language or flashy design elements</span>
                  </li>
                </ul>
              </div>

              <div class="space-y-4 rounded-lg border border-border bg-card p-6">
                <h4 class="text-lg font-semibold">Design Patterns</h4>
                <ul class="space-y-2 text-muted-foreground">
                  <li class="flex gap-2">
                    <span class="text-accent">•</span>
                    <span><strong class="text-foreground">Hero Sections:</strong> Pure black background (`--background`) with large typography</span>
                  </li>
                  <li class="flex gap-2">
                    <span class="text-accent">•</span>
                    <span><strong class="text-foreground">Content Cards:</strong> Use `--card` background with subtle borders</span>
                  </li>
                  <li class="flex gap-2">
                    <span class="text-accent">•</span>
                    <span><strong class="text-foreground">Data Display:</strong> Tables and lists with clear hierarchy and monospace values</span>
                  </li>
                  <li class="flex gap-2">
                    <span class="text-accent">•</span>
                    <span><strong class="text-foreground">Status Indicators:</strong> Use badges with appropriate variants for job/worker status</span>
                  </li>
                  <li class="flex gap-2">
                    <span class="text-accent">•</span>
                    <span><strong class="text-foreground">CTAs:</strong> Primary buttons for main actions, outline for secondary actions</span>
                  </li>
                </ul>
              </div>
            </div>
          </section>
        </div>
      </main>
      <Footer />
    </div>
  )
}
