"use client"

import { persistentAtom } from "@nanostores/persistent"

export type UiAccent = "green" | "blue" | "orange" | "pink" | "cyan"

export interface UiSettings {
  accent: UiAccent
  windowBlur: boolean
  animations: boolean
  fontSize: number
  compactMode: boolean
  systemAlerts: boolean
  nodeEvents: boolean
  sound: boolean
  maxCpuUsage: number
  maxRamUsage: number
  backgroundTasks: boolean
  powerSaver: boolean
}

export const DEFAULT_UI_SETTINGS: UiSettings = {
  accent: "green",
  windowBlur: true,
  animations: true,
  fontSize: 13,
  compactMode: false,
  systemAlerts: true,
  nodeEvents: false,
  sound: true,
  maxCpuUsage: 80,
  maxRamUsage: 60,
  backgroundTasks: true,
  powerSaver: false,
}

const ACCENTS: Record<UiAccent, { primary: string; warning: string; terminal: string }> = {
  green: { primary: "oklch(0.65 0.2 145)", warning: "oklch(0.75 0.18 80)", terminal: "oklch(0.65 0.2 145)" },
  blue: { primary: "oklch(0.62 0.18 250)", warning: "oklch(0.78 0.16 85)", terminal: "oklch(0.7 0.16 230)" },
  orange: { primary: "oklch(0.72 0.18 60)", warning: "oklch(0.78 0.16 85)", terminal: "oklch(0.76 0.15 70)" },
  pink: { primary: "oklch(0.66 0.21 340)", warning: "oklch(0.76 0.16 75)", terminal: "oklch(0.7 0.18 335)" },
  cyan: { primary: "oklch(0.68 0.16 200)", warning: "oklch(0.76 0.16 80)", terminal: "oklch(0.74 0.14 205)" },
}

function sanitize(value: unknown): UiSettings {
  const input = value && typeof value === "object" ? value as Partial<UiSettings> : {}
  return {
    ...DEFAULT_UI_SETTINGS,
    ...input,
    accent: input.accent && input.accent in ACCENTS ? input.accent : DEFAULT_UI_SETTINGS.accent,
    fontSize: clampNumber(input.fontSize, 11, 18, DEFAULT_UI_SETTINGS.fontSize),
    maxCpuUsage: clampNumber(input.maxCpuUsage, 10, 100, DEFAULT_UI_SETTINGS.maxCpuUsage),
    maxRamUsage: clampNumber(input.maxRamUsage, 10, 100, DEFAULT_UI_SETTINGS.maxRamUsage),
  }
}

function clampNumber(value: unknown, min: number, max: number, fallback: number): number {
  if (typeof value !== "number" || !Number.isFinite(value)) return fallback
  return Math.min(max, Math.max(min, Math.round(value)))
}

export const uiSettingsStore = persistentAtom<UiSettings>(
  "edgerun:ui-settings",
  DEFAULT_UI_SETTINGS,
  {
    encode: JSON.stringify,
    decode: (value) => sanitize(JSON.parse(value)),
  },
)

export function updateUiSettings(patch: Partial<UiSettings>): void {
  uiSettingsStore.set(sanitize({ ...uiSettingsStore.get(), ...patch }))
}

export function resetUiSettings(): void {
  uiSettingsStore.set(DEFAULT_UI_SETTINGS)
}

export function applyUiSettings(settings: UiSettings): void {
  if (typeof document === "undefined") return
  const root = document.documentElement
  const accent = ACCENTS[settings.accent]
  root.style.setProperty("--primary", accent.primary)
  root.style.setProperty("--accent", accent.primary)
  root.style.setProperty("--ring", accent.primary)
  root.style.setProperty("--chart-1", accent.primary)
  root.style.setProperty("--sidebar-primary", accent.primary)
  root.style.setProperty("--window-glow", accent.primary.replace(")", " / 0.3)"))
  root.style.setProperty("--terminal-text", accent.terminal)
  root.style.setProperty("--status-online", accent.primary)
  root.style.setProperty("--status-warning", accent.warning)
  root.style.setProperty("--edgerun-ui-font-size", `${settings.fontSize}px`)
  root.dataset.edgerunAnimations = settings.animations ? "on" : "off"
  root.dataset.edgerunWindowBlur = settings.windowBlur ? "on" : "off"
  root.dataset.edgerunCompact = settings.compactMode ? "on" : "off"
}
