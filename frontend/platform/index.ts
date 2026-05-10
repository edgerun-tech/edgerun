/**
 * Platform exports - main entry point for all platform services.
 */

// Protocol
export { protocolClient, ProtocolError } from "./protocol/client"
export type { NodeRegistration, ProtocolRequest, ProtocolResponse } from "./protocol/client"
export { encodeBase64, decodeBase64, sha256, computeCanonical } from "./protocol/codec"
export { formatObjectRef, formatEventRef, formatCommandRef, formatProtocolRef } from "./protocol/refs"
export { buildCommandEnvelope, serializeCommandEnvelope, deserializeCommandEnvelope } from "./protocol/commands"
export { fetchAppPackage, listInstalledApps } from "./protocol/apps"
export { listAvailableCapabilities, listGrantsForApp } from "./protocol/capabilities"
export { listProtocolApprovals, syncProtocolApprovals, approveProtocolApproval, rejectProtocolApproval } from "./protocol/approvals"
export { invokeNodeTool, invokeCodelyzerTool, invokeXrayCommand } from "./protocol/tools"
export type { NodeToolResult } from "./protocol/tools"

// State stores
export { nodeStore, reachableNodes, refreshNodeStatus } from "../stores/node-store"
export type { NodeStatus, ReachableNode } from "../stores/node-store"
export { appStore, getApp, listApps, loadApps } from "../stores/app-store"
export { capabilityStore, loadCapabilities } from "../stores/capability-store"
export { connectionStore, addConnection, removeConnection } from "../stores/connection-store"
export { permissionStore, hasPermission, requiresApproval } from "../stores/permission-store"
export { objectStore, fetchObjectMetadata as fetchObjectMeta } from "../stores/object-store"
export { commandStore, addPendingCommand } from "../stores/command-store"
export { getWasmUrl, getWasmBytes } from "./runtime/wasm-registry"

// Registries
export { capabilityRegistry, registerCapability } from "./registries/capability-registry"
export { appRegistry, registerApp } from "./registries/app-registry"
export { appCatalogRegistry, catalogApps, registerCatalogApp, getCatalogApp, listCatalogApps, verifyCatalogApp, installCatalogApp, seedBuiltinCatalogApps } from "./registries/app-catalog-registry"
export { routeRegistry, registerRoute } from "./registries/route-registry"
export { componentRegistry, registerComponent } from "./registries/component-registry"
export { toolRegistry, registerTool } from "./registries/tool-registry"

// Auth
export { permissionTracker, hasPermission as hasPerm } from "../stores/permission-tracker"
export { sessionTracker, isAuthenticated, isGuest } from "../stores/session-tracker"
export { approvalTracker, addApproval, approve, reject } from "../stores/approval-tracker"

// Router
export { router, navigate, registerAppRoutes, goBack } from "./router/edgerun-router"
export type { RouteDefinition } from "./router/route-types"
export { resolveRoute, matchRoutePattern } from "./router/route-resolver"

// Runtime
export { wasmRegistry, loadWasmForApp, removeWasm } from "./runtime/wasm-registry"
export { appRuntime, startApp, stopApp } from "./runtime/app-runtime"
export { previewRuntime, startPreview } from "./runtime/preview-runtime"
export { resolveCapabilityForAction, resolveCapabilityForPipeline } from "./runtime/capability-resolver"

// UI hooks
export { PlatformProvider, usePlatform } from "./ui/PlatformProvider"
export { useNode } from "../hooks/useNode"
export { useApps } from "../hooks/useApps"
export { useCapabilities } from "../hooks/useCapabilities"
export { useConnections } from "../hooks/useConnections"
export { usePermissions } from "../hooks/usePermissions"
export { useCommands } from "../hooks/useCommands"
export { useObjects } from "../hooks/useObjects"
export { useRuntime } from "../hooks/useRuntime"
export { useEdgeRunRouter } from "../hooks/useEdgeRunRouter"
