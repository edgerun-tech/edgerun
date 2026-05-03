import { atom } from "nanostores"
import type { RuntimeNodeStats } from "../graph/types"

interface RuntimeOverlayState {
  enabled: boolean
  stats: Map<string, RuntimeNodeStats>
  tick: number
}

export const runtimeOverlay = atom<RuntimeOverlayState>({
  enabled: false,
  stats: new Map(),
  tick: 0,
})

export function setRuntimeOverlayEnabled(enabled: boolean) {
  const s = runtimeOverlay.get()
  runtimeOverlay.set({ ...s, enabled })
}

export function updateRuntimeStats(stats: Map<string, RuntimeNodeStats>) {
  const s = runtimeOverlay.get()
  runtimeOverlay.set({ ...s, stats, tick: s.tick + 1 })
}

export function tickRuntime() {
  const s = runtimeOverlay.get()
  runtimeOverlay.set({ ...s, tick: s.tick + 1 })
}
