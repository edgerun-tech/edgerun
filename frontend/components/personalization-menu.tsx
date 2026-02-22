import { For, Show, createSignal, onCleanup, onMount } from 'solid-js'
import { Button } from './ui/button'
import {
  ACCENT_OPTIONS,
  DEFAULT_PERSONALIZATION,
  RADIUS_OPTIONS,
  readPersonalizationSettings,
  savePersonalizationSettings,
  type PersonalizationSettings
} from '../lib/personalization'

export function PersonalizationMenu() {
  const [open, setOpen] = createSignal(false)
  const [settings, setSettings] = createSignal<PersonalizationSettings>(DEFAULT_PERSONALIZATION)
  let rootRef: HTMLDivElement | undefined

  onMount(() => {
    setSettings(readPersonalizationSettings())
    const onPointerDown = (event: PointerEvent) => {
      if (!open()) return
      const target = event.target as Node | null
      if (!rootRef || !target || rootRef.contains(target)) return
      setOpen(false)
    }
    const onEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false)
    }
    window.addEventListener('pointerdown', onPointerDown)
    window.addEventListener('keydown', onEscape)
    onCleanup(() => {
      window.removeEventListener('pointerdown', onPointerDown)
      window.removeEventListener('keydown', onEscape)
    })
  })

  const applyAccent = (accentId: string) => {
    const next = savePersonalizationSettings({ ...settings(), accentId })
    setSettings(next)
  }

  const applyRadius = (radiusId: string) => {
    const next = savePersonalizationSettings({ ...settings(), radiusId })
    setSettings(next)
  }

  const accentSwatchClass = (accentId: string): string => `accent-swatch accent-swatch-${accentId}`

  return (
    <div ref={rootRef} class="relative">
      <Button
        variant="outline"
        size="sm"
        class="h-9 w-9 p-0"
        aria-label="Open personalization settings"
        aria-haspopup="dialog"
        aria-expanded={open()}
        onClick={() => setOpen((value) => !value)}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true" class="h-4 w-4">
          <path fill="currentColor" d="M12 3a3 3 0 0 1 3 3c0 .5-.12.97-.33 1.39l5.61 5.62a2.25 2.25 0 0 1-3.18 3.18l-1.2-1.2a4 4 0 0 1-4.9 4.9l-4.6 4.6-1.4-1.4 4.6-4.6a4 4 0 0 1-4.9-4.9L3.52 11.4a2.25 2.25 0 0 1 3.18-3.18L12.32 13.8A3 3 0 1 1 12 3Zm0 2a1 1 0 1 0 .01 2.01A1 1 0 0 0 12 5Zm6.87 9.42-.46.46.82.82a.25.25 0 0 0 .35-.36zm-13.74-3.7a.25.25 0 0 0 0 .35l.82.82.47-.47-.82-.82a.25.25 0 0 0-.47.12Zm6.86 5.28a2 2 0 1 0 0 .01Zm0-4a2 2 0 1 0 0 .01Z" />
        </svg>
      </Button>

      <Show when={open()}>
        <section
          role="dialog"
          aria-label="Personalization settings"
          class="absolute right-0 top-11 z-[90] w-72 rounded-lg border border-border bg-popover p-3 text-popover-foreground shadow-xl"
        >
          <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Personalization</p>

          <div class="mt-3">
            <p class="text-xs font-medium text-muted-foreground">Accent Color</p>
            <div class="mt-2 grid grid-cols-3 gap-2">
              <For each={ACCENT_OPTIONS}>{(option) => (
                <button
                  type="button"
                  class={`rounded-md border px-2 py-1 text-xs transition-colors ${
                    settings().accentId === option.id
                      ? 'border-primary text-foreground'
                      : 'border-border text-muted-foreground hover:text-foreground'
                  }`}
                  onClick={() => applyAccent(option.id)}
                >
                  <span class={`mr-1 inline-block h-2.5 w-2.5 rounded-full align-middle ${accentSwatchClass(option.id)}`} />
                  {option.label}
                </button>
              )}</For>
            </div>
          </div>

          <div class="mt-4">
            <p class="text-xs font-medium text-muted-foreground">Corner Radius</p>
            <div class="mt-2 grid grid-cols-2 gap-2">
              <For each={RADIUS_OPTIONS}>{(option) => (
                <button
                  type="button"
                  class={`rounded-md border px-2 py-1 text-xs transition-colors ${
                    settings().radiusId === option.id
                      ? 'border-primary text-foreground'
                      : 'border-border text-muted-foreground hover:text-foreground'
                  }`}
                  onClick={() => applyRadius(option.id)}
                >
                  {option.label}
                </button>
              )}</For>
            </div>
          </div>
        </section>
      </Show>
    </div>
  )
}
