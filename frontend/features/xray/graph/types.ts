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
  language?: "rust" | "typescript" | "javascript" | "python" | "go" | "java" | "c" | "unknown"
  source?: {
    file?: string
    line?: number
    symbol?: string
  }
  x?: number
  y?: number
  z?: number
  prevX?: number
  prevY?: number
  prevZ?: number
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

export type XrayFilterKey =
  | "file"
  | "function"
  | "ui"
  | "runtime"
  | "storage"
  | "network"
  | "crypto"
  | "agent"
  | "rust"
  | "typescript"
  | "javascript"
  | "go"
  | "python"
  | "c"
  | "unknown"

export type XrayState = {
  nodes: Map<string, XrayNode>
  edges: XrayEdge[]
  runtimeStats: Map<string, RuntimeNodeStats>
  selectedId: string | null
  hoveredId: string | null
  highlightedIds: Set<string>
  hiddenFilterKeys: Set<XrayFilterKey>
  layout: LayoutType
  runtimeMode: boolean
  zoom: number
  panX: number
  panY: number
  rotation: number
  loading: boolean
  error: string | null
}
