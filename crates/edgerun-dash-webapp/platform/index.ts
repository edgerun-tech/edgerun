/**
 * Platform exports - main entry point for all platform services.
 */

// Protocol
export { protocolClient, ProtocolError } from "./protocol/client"
export type { NodeRegistration, ProtocolRequest, ProtocolResponse } from "./protocol/client"
export { encodeBase64, decodeBase64, sha256, computeCanonical } from "./protocol/codec"
export { formatObjectRef, parseObjectRef, type ObjectRef, type ProtocolRef } from "./protocol/refs"
export { buildCommandEnvelope, sendCommand } from "./protocol/commands"
export type { CommandEnvelope, CommandResult } from "./protocol/commands"
export { fetchObject, fetchObjectMetadata } from "./protocol/objects"
export type { StoredObject, ObjectMetadata } from "./protocol/objects"
export { fetchAppPackage, listInstalledApps } from "./protocol/apps"
export type { AppPackage, AppRoute, AppAction, AppPipeline } from "./protocol/apps"
export { listAvailableCapabilities, canSatisfy } from "./protocol/capabilities"
export type { CapabilityDescriptor, CapabilityGrant, CapabilitySatisfaction } from "./protocol/capabilities"

// State stores
export { nodeStore, refreshNodeStatus } from "./state/node-store"
export type { NodeStatus } from "./state/node-store"
export { appStore, getApp, listApps, loadApps } from "./state/app-store"
export { capabilityStore, loadCapabilities } from "./state/capability-store"
export { connectionStore, addConnection, removeConnection } from "./state/connection-store"
export { permissionStore, hasPermission, requiresApproval } from "./state/permission-store"
export { objectStore, fetchObjectMetadata as fetchObjectMeta } from "./state/object-store"
export { commandStore, addPendingCommand } from "./state/command-store"
export { runtimeStore, cacheWasm, removeWasmCache } from "./state/runtime-store"

// Registries
export { capabilityRegistry, registerCapability } from "./registries/capability-registry"
export { appRegistry, registerApp } from "./registries/app-registry"
export { routeRegistry, registerRoute } from "./registries/route-registry"
export { componentRegistry, registerComponent } from "./registries/component-registry"
export { connectionRegistry, registerConnection } from "./registries/connection-registry"
export { toolRegistry, registerTool } from "./registries/tool-registry"

// Auth
export { permissionTracker, hasPermission as hasPerm } from "./auth/permission-tracker"
export { externalConnectionTracker } from "./auth/external-connection-tracker"
export { sessionTracker, isAuthenticated, isGuest } from "./auth/session-tracker"
export { approvalTracker, addApproval, approve, reject } from "./auth/approval-tracker"

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
export { useNode } from "./ui/useNode"
export { useApps } from "./ui/useApps"
export { useCapabilities } from "./ui/useCapabilities"
export { useConnections } from "./ui/useConnections"
export { usePermissions } from "./ui/usePermissions"
export { useEdgeRunRouter } from "./ui/useEdgeRunRouter"
export { useObjects } from "./ui/useObjects"
export { useCommands } from "./ui/useCommands"
export { useRuntime } from "./ui/useRuntime"
