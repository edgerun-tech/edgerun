import type { XrayNode } from "../graph/types"

const LAYER_ORDER = ["ui", "api", "runtime", "protocol", "storage", "network", "crypto", "agent"] as const

export function runLayerLayout(nodes: Map<string, XrayNode>) {
  const byLayer = new Map<string, XrayNode[]>()
  for (const node of nodes.values()) {
    const layer = node.layer ?? "unknown"
    if (!byLayer.has(layer)) byLayer.set(layer, [])
    byLayer.get(layer)!.push(node)
  }

  const bandWidth = 160
  const bandGap = 40
  let x = 0

  for (const layerName of LAYER_ORDER) {
    const layerNodes = byLayer.get(layerName)
    if (!layerNodes) continue

    const count = layerNodes.length
    const spacing = 60
    for (let i = 0; i < count; i++) {
      const node = layerNodes[i]
      node.x = x + (i - (count - 1) / 2) * spacing * 0.5
      node.y = (i - (count - 1) / 2) * spacing
    }

    x += bandWidth + bandGap
  }

  for (const [_, layerNodes] of byLayer) {
    if (layerNodes.length > 0 && !LAYER_ORDER.includes(layerNodes[0].layer as any)) {
      const spacing = 60
      for (let i = 0; i < layerNodes.length; i++) {
        const node = layerNodes[i]
        node.x = x + (i - (layerNodes.length - 1) / 2) * spacing * 0.5
        node.y = (i - (layerNodes.length - 1) / 2) * spacing
      }
    }
  }
}
