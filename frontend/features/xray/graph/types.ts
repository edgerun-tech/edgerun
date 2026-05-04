export type XrayNode = {
  id: string
  kind:
    | "function"
    | "file"
    | "process"
    | "machine"
    | "protocol"
    | "event"
    | "test"
    | "storage"
    | "network"
    | "agent"
  label: string
  tags: string[]
  layer?: "ui" | "api" | "runtime" | "protocol" | "storage" | "network" | "crypto" | "agent"
  language?: "rust" | "typescript" | "c" | "unknown"
  source?: {
    file?: string
    line?: number
    symbol?: string
  }
  x?: number
  y?: number
  prevX?: number
  prevY?: number
}

export type XrayEdge = {
  id: string
  source: string
  target: string
  kind:
    | "calls"
    | "imports"
    | "owns"
    | "implements"
    | "sends"
    | "receives"
    | "stores"
    | "signs"
    | "verifies"
    | "tests"
    | "observed_flow"
  tags: string[]
}

export type RuntimeNodeStats = {
  hits: number
  openSpans: number
  errors: number
  totalUs: number
  maxUs: number
  lastTsUs: number
}

export type LayoutType = "force" | "globe" | "layers"

export type XrayState = {
  nodes: Map<string, XrayNode>
  edges: XrayEdge[]
  runtimeStats: Map<string, RuntimeNodeStats>
  selectedId: string | null
  highlightedIds: Set<string>
  layout: LayoutType
  runtimeMode: boolean
  zoom: number
  panX: number
  panY: number
  rotation: number
  loading: boolean
  error: string | null
}
