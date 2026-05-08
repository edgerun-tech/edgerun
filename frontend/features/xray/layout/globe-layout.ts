import type { XrayNode } from "../graph/types"

const LAYER_ORDER = ["ui", "api", "runtime", "protocol", "storage", "network", "crypto", "agent"] as const

export function runGlobeLayout(nodes: Map<string, XrayNode>) {
  const nodeArray = Array.from(nodes.values())
  const n = nodeArray.length
  if (n === 0) return

  const radius = Math.max(200, Math.sqrt(n) * 25)

  for (let i = 0; i < n; i++) {
    const node = nodeArray[i]
    const layerIdx = node.layer ? LAYER_ORDER.indexOf(node.layer as any) : -1
    const layerFraction = layerIdx >= 0 ? (layerIdx + 1) / LAYER_ORDER.length : 0.5
    const r = radius * (0.3 + layerFraction * 0.7)

    const angle = (i / n) * Math.PI * 2 + layerIdx * 0.5
    const z = (Math.random() - 0.5) * radius * 0.4
    const xy = Math.sqrt(Math.max(0, r * r - z * z))

    node.x = xy * Math.cos(angle)
    node.y = xy * Math.sin(angle)
  }
}
