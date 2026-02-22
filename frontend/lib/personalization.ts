export type AccentOption = {
  id: string
  label: string
  primary: string
  foreground: string
}

export type RadiusOption = {
  id: string
  label: string
  value: string
}

export type PersonalizationSettings = {
  accentId: string
  radiusId: string
}

const STORAGE_KEY = 'edgerun.personalization.v1'

export const ACCENT_OPTIONS: AccentOption[] = [
  { id: 'blue', label: 'Blue', primary: 'oklch(0.62 0.15 248)', foreground: 'oklch(0.98 0 0)' },
  { id: 'cyan', label: 'Cyan', primary: 'oklch(0.72 0.12 210)', foreground: 'oklch(0.08 0 0)' },
  { id: 'teal', label: 'Teal', primary: 'oklch(0.7 0.13 180)', foreground: 'oklch(0.08 0 0)' },
  { id: 'green', label: 'Green', primary: 'oklch(0.74 0.15 145)', foreground: 'oklch(0.08 0 0)' },
  { id: 'amber', label: 'Amber', primary: 'oklch(0.8 0.14 85)', foreground: 'oklch(0.08 0 0)' },
  { id: 'rose', label: 'Rose', primary: 'oklch(0.67 0.18 18)', foreground: 'oklch(0.98 0 0)' }
]

export const RADIUS_OPTIONS: RadiusOption[] = [
  { id: 'tight', label: 'Tight', value: '0.45rem' },
  { id: 'default', label: 'Default', value: '0.625rem' },
  { id: 'soft', label: 'Soft', value: '0.8rem' },
  { id: 'rounded', label: 'Rounded', value: '1rem' }
]

export const DEFAULT_PERSONALIZATION: PersonalizationSettings = {
  accentId: 'blue',
  radiusId: 'default'
}

function safeWindow(): Window | null {
  return typeof window === 'undefined' ? null : window
}

function sanitizeSettings(input: Partial<PersonalizationSettings> | null | undefined): PersonalizationSettings {
  const accent = ACCENT_OPTIONS.find((option) => option.id === input?.accentId) ?? ACCENT_OPTIONS.find((option) => option.id === DEFAULT_PERSONALIZATION.accentId)!
  const radius = RADIUS_OPTIONS.find((option) => option.id === input?.radiusId) ?? RADIUS_OPTIONS.find((option) => option.id === DEFAULT_PERSONALIZATION.radiusId)!
  return {
    accentId: accent.id,
    radiusId: radius.id
  }
}

export function readPersonalizationSettings(): PersonalizationSettings {
  const w = safeWindow()
  if (!w) return DEFAULT_PERSONALIZATION
  try {
    const raw = w.localStorage.getItem(STORAGE_KEY)
    if (!raw) return DEFAULT_PERSONALIZATION
    return sanitizeSettings(JSON.parse(raw) as Partial<PersonalizationSettings>)
  } catch {
    return DEFAULT_PERSONALIZATION
  }
}

export function applyPersonalizationSettings(settings: PersonalizationSettings): void {
  const w = safeWindow()
  if (!w) return
  const root = w.document.documentElement
  const safe = sanitizeSettings(settings)
  const accent = ACCENT_OPTIONS.find((option) => option.id === safe.accentId)!
  const radius = RADIUS_OPTIONS.find((option) => option.id === safe.radiusId)!

  root.style.setProperty('--primary', accent.primary)
  root.style.setProperty('--primary-foreground', accent.foreground)
  root.style.setProperty('--accent', accent.primary)
  root.style.setProperty('--accent-foreground', accent.foreground)
  root.style.setProperty('--ring', accent.primary)
  root.style.setProperty('--radius', radius.value)
}

export function savePersonalizationSettings(settings: PersonalizationSettings): PersonalizationSettings {
  const w = safeWindow()
  const safe = sanitizeSettings(settings)
  if (w) {
    w.localStorage.setItem(STORAGE_KEY, JSON.stringify(safe))
  }
  applyPersonalizationSettings(safe)
  return safe
}

