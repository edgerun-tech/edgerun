import type { XrayNode, XrayEdge } from "../graph/types"

export function runForceLayout(nodes: Map<string, XrayNode>, edges: XrayEdge[], iterations = 120) {
  const nodeArray = Array.from(nodes.values())
  const n = nodeArray.length
  if (n === 0) return

  const spread = Math.sqrt(n) * 35
  for (const node of nodeArray) {
    if (node.x === undefined || node.x === 0) {
      node.x = (Math.random() - 0.5) * spread
      node.y = (Math.random() - 0.5) * spread
    }
  }

  const repulsion = 3000
  const attraction = 0.003
  const gravity = 0.002
  const damping = 0.85
  const cellSize = 80

  for (let iter = 0; iter < iterations; iter++) {
    const grid = new Map<string, { x: number; y: number; node: XrayNode }[]>()
    for (const node of nodeArray) {
      const cx = Math.floor((node.x ?? 0) / cellSize)
      const cy = Math.floor((node.y ?? 0) / cellSize)
      const key = `${cx},${cy}`
      if (!grid.has(key)) grid.set(key, [])
      grid.get(key)!.push({ x: node.x ?? 0, y: node.y ?? 0, node })
    }

    for (const node of nodeArray) {
      let fx = 0
      let fy = 0
      const cx = Math.floor((node.x ?? 0) / cellSize)
      const cy = Math.floor((node.y ?? 0) / cellSize)
      for (let dx = -1; dx <= 1; dx++) {
        for (let dy = -1; dy <= 1; dy++) {
          const key = `${cx + dx},${cy + dy}`
          const cell = grid.get(key)
          if (!cell) continue
          for (const other of cell) {
            if (other.node === node) continue
            const ddx = (node.x ?? 0) - other.x
            const ddy = (node.y ?? 0) - other.y
            const distSq = ddx * ddx + ddy * ddy + 1
            const dist = Math.sqrt(distSq)
            const force = repulsion / distSq
            fx += (ddx / dist) * force
            fy += (ddy / dist) * force
          }
        }
      }
      ;(node as any).vx = ((node as any).vx ?? 0) * damping + fx * 0.5
      ;(node as any).vy = ((node as any).vy ?? 0) * damping + fy * 0.5
    }

    for (const edge of edges) {
      const src = nodes.get(edge.source)
      const tgt = nodes.get(edge.target)
      if (!src || !tgt) continue
      const ddx = (tgt.x ?? 0) - (src.x ?? 0)
      const ddy = (tgt.y ?? 0) - (src.y ?? 0)
      const dist = Math.sqrt(ddx * ddx + ddy * ddy) + 0.1
      const force = dist * attraction
      const ffx = (ddx / dist) * force
      const ffy = (ddy / dist) * force
      ;(src as any).vx = ((src as any).vx ?? 0) + ffx
      ;(src as any).vy = ((src as any).vy ?? 0) + ffy
      ;(tgt as any).vx = ((tgt as any).vx ?? 0) - ffx
      ;(tgt as any).vy = ((tgt as any).vy ?? 0) - ffy
    }

    for (const node of nodeArray) {
      ;(node as any).vx = ((node as any).vx ?? 0) - (node.x ?? 0) * gravity
      ;(node as any).vy = ((node as any).vy ?? 0) - (node.y ?? 0) * gravity
      const vx = Math.max(-8, Math.min(8, (node as any).vx ?? 0))
      const vy = Math.max(-8, Math.min(8, (node as any).vy ?? 0))
      node.x = (node.x ?? 0) + vx
      node.y = (node.y ?? 0) + vy
      ;(node as any).vx = vx * damping
      ;(node as any).vy = vy * damping
    }
  }
}
