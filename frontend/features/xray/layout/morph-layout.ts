import type { XrayNode } from "./types"

export function morphPositions(nodes: Map<string, XrayNode>, targetPositions: Map<string, { x: number; y: number }>, steps = 30) {
  for (const node of nodes.values()) {
    const target = targetPositions.get(node.id)
    if (!target) continue

    node.prevX = node.x ?? 0
    node.prevY = node.y ?? 0

    const dx = target.x - (node.x ?? 0)
    const dy = target.y - (node.y ?? 0)
    node.x = target.x
    node.y = target.y
  }
}

export function lerpNodePositions(nodes: Map<string, XrayNode>, t: number) {
  for (const node of nodes.values()) {
    if (node.prevX !== undefined && node.prevY !== undefined) {
      node.x = (node.prevX ?? 0) + ((node.x ?? 0) - (node.prevX ?? 0)) * t
      node.y = (node.prevY ?? 0) + ((node.y ?? 0) - (node.prevY ?? 0)) * t
    }
  }
}
