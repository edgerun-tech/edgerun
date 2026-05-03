import type { RuntimeNodeStats } from "../graph/types"

const LAYER_COLORS: Record<string, [number, number, number, number]> = {
  ui: [0.55, 0.8, 0.95, 0.9],
  api: [0.6, 0.75, 0.4, 0.9],
  runtime: [0.95, 0.55, 0.15, 0.9],
  protocol: [0.7, 0.5, 0.85, 0.9],
  storage: [0.85, 0.75, 0.2, 0.9],
  network: [0.2, 0.65, 0.8, 0.9],
  crypto: [0.9, 0.3, 0.3, 0.9],
  agent: [0.5, 0.5, 0.55, 0.7],
}

export function getNodeColor(layer: string | undefined, _language: string | undefined, highlighted: boolean, selected: boolean, runtime: RuntimeNodeStats | undefined, runtimeMode: boolean): [number, number, number, number] {
  const base: [number, number, number, number] = LAYER_COLORS[layer ?? "unknown"] ?? LAYER_COLORS["agent"]

  if (selected) return [1, 1, 1, 1]
  if (highlighted) return [1, 0.85, 0.18, 1]

  if (runtimeMode && runtime) {
    return getRuntimeNodeColor(base, runtime)
  }

  return base
}

function getRuntimeNodeColor(base: [number, number, number, number], runtime: RuntimeNodeStats): [number, number, number, number] {
  const [r, g, b, a] = base

  if (runtime.errors > 0) {
    const intensity = Math.min(runtime.errors / 10, 1.0)
    return [
      r + (1.0 - r) * intensity,
      g * (1.0 - intensity),
      b * (1.0 - intensity),
      a,
    ]
  }

  if (runtime.openSpans > 0) {
    const pulse = 0.5 + 0.5 * Math.sin(Date.now() / 200)
    return [
      1.0 - (1.0 - r) * (1 - pulse * 0.3),
      0.3 + 0.7 * (1 - pulse * 0.5),
      0.3 + 0.7 * (1 - pulse * 0.5),
      a,
    ]
  }

  if (runtime.hits > 0) {
    const boost = Math.min(runtime.hits / 200, 0.5)
    return [
      Math.min(r + boost, 1.0),
      Math.min(g + boost, 1.0),
      Math.min(b + boost, 1.0),
      a,
    ]
  }

  return base
}

export function getNodeSize(_connections: number, highlighted: boolean, selected: boolean, runtime: RuntimeNodeStats | undefined, runtimeMode: boolean): number {
  let base = 7

  if (runtimeMode && runtime && runtime.maxUs > 0) {
    const sizeBoost = Math.min(runtime.maxUs / 50000, 3.0)
    base = base * (1.0 + sizeBoost)
  }

  if (selected) return base * 1.6
  if (highlighted) return base * 1.3
  return base
}

export function getEdgeColor(kind: string, highlighted: boolean): [number, number, number] {
  if (highlighted) return [0.9, 0.8, 0.2]

  const kinds: Record<string, [number, number, number]> = {
    calls: [0.3, 0.3, 0.4],
    imports: [0.35, 0.35, 0.4],
    owns: [0.4, 0.3, 0.3],
    implements: [0.3, 0.4, 0.3],
    sends: [0.25, 0.3, 0.45],
    receives: [0.25, 0.3, 0.45],
    stores: [0.45, 0.35, 0.2],
    signs: [0.6, 0.2, 0.2],
    verifies: [0.2, 0.6, 0.2],
    tests: [0.8, 0.7, 0.2],
    observed_flow: [0.4, 0.25, 0.5],
  }
  return kinds[kind] ?? [0.25, 0.25, 0.3]
}
