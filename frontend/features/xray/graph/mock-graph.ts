import type { RuntimeNodeStats, XrayEdge, XrayNode } from "./types"

export function createMockGraph(): { nodes: Map<string, XrayNode>; edges: XrayEdge[] } {
  const nodes = new Map<string, XrayNode>([
    ["identity", { id: "identity", kind: "protocol", label: "Identity", tags: ["root"], layer: "protocol", language: "typescript", x: -260, y: 40 }],
    ["xray", { id: "xray", kind: "agent", label: "Xray Graph", tags: ["visual"], layer: "ui", language: "typescript", x: 0, y: 0 }],
    ["apps", { id: "apps", kind: "process", label: "App Surfaces", tags: ["overlay", "slots"], layer: "ui", language: "typescript", x: 260, y: 40 }],
    ["runtime", { id: "runtime", kind: "process", label: "Runtime", tags: ["wasm", "worker"], layer: "runtime", language: "rust", x: 0, y: -220 }],
    ["storage", { id: "storage", kind: "storage", label: "Event Log", tags: ["signed", "append-only"], layer: "storage", language: "rust", x: -220, y: -170 }],
    ["network", { id: "network", kind: "network", label: "Mesh", tags: ["relay", "sealed"], layer: "network", language: "rust", x: 220, y: -170 }],
    ["payments", { id: "payments", kind: "protocol", label: "Payments", tags: ["settlement"], layer: "protocol", language: "typescript", x: -180, y: 230 }],
    ["compute", { id: "compute", kind: "machine", label: "Compute Nodes", tags: ["market"], layer: "runtime", language: "rust", x: 180, y: 230 }],
  ])

  const edges: XrayEdge[] = [
    { id: "identity->apps", source: "identity", target: "apps", kind: "verifies", tags: [] },
    { id: "identity->storage", source: "identity", target: "storage", kind: "signs", tags: [] },
    { id: "xray->apps", source: "xray", target: "apps", kind: "observed_flow", tags: [] },
    { id: "runtime->compute", source: "runtime", target: "compute", kind: "sends", tags: [] },
    { id: "runtime->storage", source: "runtime", target: "storage", kind: "stores", tags: [] },
    { id: "network->compute", source: "network", target: "compute", kind: "receives", tags: [] },
    { id: "payments->compute", source: "payments", target: "compute", kind: "owns", tags: [] },
    { id: "apps->payments", source: "apps", target: "payments", kind: "calls", tags: [] },
    { id: "xray->runtime", source: "xray", target: "runtime", kind: "observed_flow", tags: [] },
  ]

  return { nodes, edges }
}

export function createMockRuntimeStats(nodes: Map<string, XrayNode>): Map<string, RuntimeNodeStats> {
  const stats = new Map<string, RuntimeNodeStats>()
  let i = 0
  for (const id of nodes.keys()) {
    stats.set(id, {
      hits: 1200 + i * 371,
      openSpans: i % 3,
      errors: i === 5 ? 2 : 0,
      totalUs: 180_000 + i * 25_000,
      maxUs: 12_000 + i * 1500,
      lastTsUs: Date.now() * 1000,
    })
    i++
  }
  return stats
}
