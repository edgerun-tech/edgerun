export type ResourceSource = "browser" | "node" | "mesh" | "manual"

export interface NodeCpuResource {
  cores: number
  utilization: number
}

export interface NodeResource {
  nodeId: string
  label?: string
  source: ResourceSource
  health: "healthy" | "degraded" | "unhealthy" | "failed"
  lastHeartbeat: number
  cpu: NodeCpuResource
  memoryBytes?: number
  storageBytes?: number
}
