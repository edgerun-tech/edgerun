import type { XrayNode, XrayEdge, RuntimeNodeStats } from "./types"

function n(id: string, kind: XrayNode["kind"], label: string, layer: XrayNode["layer"], language: XrayNode["language"], tags: string[], source?: XrayNode["source"]): XrayNode {
  return { id, kind, label, layer, language, tags, source: source ?? { file: `crates/${id}/src/lib.rs`, line: 1, symbol: label } }
}

function e(id: string, source: string, target: string, kind: XrayEdge["kind"], tags: string[] = []): XrayEdge {
  return { id, source, target, kind, tags }
}

const uiNodes: XrayNode[] = [
  n("ui-desktop", "function", "Desktop", "ui", "typescript", ["ui", "desktop"]),
  n("ui-window", "function", "Window", "ui", "typescript", ["ui", "window"]),
  n("ui-topbar", "function", "TopBar", "ui", "typescript", ["ui", "topbar"]),
  n("ui-widget-panel", "function", "WidgetPanel", "ui", "typescript", ["ui", "widgets"]),
  n("ui-stage-manager", "function", "StageManager", "ui", "typescript", ["ui", "stage-manager"]),
  n("ui-command-input", "function", "CommandInput", "ui", "typescript", ["ui", "command"]),
  n("ui-auth-overlay", "function", "AuthOverlay", "ui", "typescript", ["ui", "auth"]),
  n("ui-context-menu", "function", "ContextMenuProvider", "ui", "typescript", ["ui", "context"]),
  n("ui-tooltip", "function", "TooltipProvider", "ui", "typescript", ["ui", "tooltip"]),
  n("ui-page", "file", "app/page.tsx", "ui", "typescript", ["ui", "page"]),
]

const apiNodes: XrayNode[] = [
  n("api-assistant", "function", "AssistantRoute", "api", "typescript", ["api", "assistant"]),
  n("api-chat", "function", "ChatRoute", "api", "typescript", ["api", "chat"]),
  n("api-gmail-auth", "function", "GmailAuth", "api", "typescript", ["api", "gmail"]),
  n("api-gmail-callback", "function", "GmailCallback", "api", "typescript", ["api", "gmail"]),
  n("api-gmail-emails", "function", "GmailEmails", "api", "typescript", ["api", "gmail"]),
  n("api-terminal", "function", "TerminalRoute", "api", "typescript", ["api", "terminal"]),
]

const runtimeNodes: XrayNode[] = [
  n("rt-edgerun-router", "function", "EdgeRunRouter", "runtime", "typescript", ["runtime", "router"]),
  n("rt-platform-provider", "function", "PlatformProvider", "runtime", "typescript", ["runtime", "provider"]),
  n("rt-capability-registry", "function", "CapabilityRegistry", "runtime", "typescript", ["runtime", "capabilities"]),
  n("rt-auth-state", "function", "AuthState", "runtime", "typescript", ["runtime", "auth"]),
  n("rt-wasm-loader", "function", "WasmLoader", "runtime", "rust", ["runtime", "wasm"]),
  n("rt-process", "process", "edgerun-dash", "runtime", "unknown", ["runtime", "process"]),
  n("rt-machine", "machine", "dashboard-host", "runtime", "unknown", ["runtime", "machine"]),
]

const protocolNodes: XrayNode[] = [
  n("proto-common", "protocol", "common.proto", "protocol", "rust", ["protocol", "common"]),
  n("proto-identity", "protocol", "identity.proto", "protocol", "rust", ["protocol", "identity"]),
  n("proto-trust", "protocol", "trust.proto", "protocol", "rust", ["protocol", "trust"]),
  n("proto-stream", "protocol", "stream.proto", "protocol", "rust", ["protocol", "stream"]),
  n("proto-object", "protocol", "object.proto", "protocol", "rust", ["protocol", "object"]),
  n("proto-access", "protocol", "access.proto", "protocol", "rust", ["protocol", "access"]),
  n("proto-network", "protocol", "network.proto", "protocol", "rust", ["protocol", "network"]),
]

const storageNodes: XrayNode[] = [
  n("store-lib", "function", "StorageCore", "storage", "rust", ["storage", "core"]),
  n("store-fs", "function", "FsStore", "storage", "rust", ["storage", "fs"]),
  n("store-event-log", "function", "EventLog", "storage", "rust", ["storage", "event-log"]),
  n("store-file-index", "function", "FileIndex", "storage", "rust", ["storage", "index"]),
  n("store-blobs", "function", "BlobStore", "storage", "rust", ["storage", "blobs"]),
]

const networkNodes: XrayNode[] = [
  n("net-mesh", "network", "MeshTransport", "network", "rust", ["network", "mesh"]),
  n("net-mesh-link", "network", "MeshLink", "network", "rust", ["network", "link"]),
  n("net-mesh-session", "network", "MeshSession", "network", "rust", ["network", "session"]),
  n("net-mesh-daemon", "network", "MeshDaemon", "network", "rust", ["network", "daemon"]),
  n("net-tcp-server", "function", "TcpServer", "network", "rust", ["network", "tcp"]),
  n("net-quic", "network", "QuicTransport", "network", "rust", ["network", "quic"]),
  n("net-tls", "network", "TlsAdapter", "network", "rust", ["network", "tls"]),
]

const cryptoNodes: XrayNode[] = [
  n("crypto-core", "function", "CryptoCore", "crypto", "rust", ["crypto", "core"]),
  n("crypto-signing", "function", "HardwareSigning", "crypto", "rust", ["crypto", "signing"]),
  n("crypto-tpm", "function", "TpmProvider", "crypto", "rust", ["crypto", "tpm"]),
  n("crypto-yubikey", "function", "YubikeyProvider", "crypto", "rust", ["crypto", "yubikey"]),
  n("crypto-keystore", "function", "AndroidKeystore", "crypto", "rust", ["crypto", "keystore"]),
]

const agentNodes: XrayNode[] = [
  n("agent-capability", "agent", "CapabilityAgent", "agent", "typescript", ["agent", "capability"]),
  n("agent-policy", "agent", "PolicyEngine", "agent", "rust", ["agent", "policy"]),
  n("agent-registry", "agent", "AgentRegistry", "agent", "typescript", ["agent", "registry"]),
]

const testNodes: XrayNode[] = [
  n("test-core", "test", "edgerun-core tests", "runtime", "rust", ["test", "core"]),
  n("test-stream", "test", "edgerun-stream tests", "runtime", "rust", ["test", "stream"]),
  n("test-storage", "test", "edgerun-storage tests", "runtime", "rust", ["test", "storage"]),
  n("test-node", "test", "edgerun-node tests", "runtime", "rust", ["test", "node"]),
]

const eventNodes: XrayNode[] = [
  n("event-stream", "event", "EventStream", "protocol", "rust", ["event", "stream"]),
  n("event-genesis", "event", "GenesisEvent", "protocol", "rust", ["event", "genesis"]),
  n("event-command", "event", "CommandEvent", "protocol", "rust", ["event", "command"]),
]

const allNodes = [...uiNodes, ...apiNodes, ...runtimeNodes, ...protocolNodes, ...storageNodes, ...networkNodes, ...cryptoNodes, ...agentNodes, ...testNodes, ...eventNodes]

const edges: XrayEdge[] = [
  // UI -> API
  e("ui-desktop->api-chat", "ui-desktop", "api-chat", "calls"),
  e("ui-command-input->api-assistant", "ui-command-input", "api-assistant", "calls"),
  e("ui-auth-overlay->api-gmail-auth", "ui-auth-overlay", "api-gmail-auth", "calls"),
  e("ui-widget-panel->api-terminal", "ui-widget-panel", "api-terminal", "calls"),

  // API -> Protocol
  e("api-assistant->proto-identity", "api-assistant", "proto-identity", "imports"),
  e("api-chat->proto-stream", "api-chat", "proto-stream", "imports"),
  e("api-gmail-auth->proto-access", "api-gmail-auth", "proto-access", "imports"),

  // Protocol -> Crypto
  e("proto-identity->crypto-core", "proto-identity", "crypto-core", "imports"),
  e("proto-trust->crypto-signing", "proto-trust", "crypto-signing", "imports"),
  e("proto-stream->crypto-tpm", "proto-stream", "crypto-tpm", "imports"),
  e("proto-trust->crypto-yubikey", "proto-trust", "crypto-yubikey", "imports"),
  e("proto-identity->crypto-keystore", "proto-identity", "crypto-keystore", "imports"),

  // Runtime -> Storage
  e("rt-wasm-loader->store-blobs", "rt-wasm-loader", "store-blobs", "imports"),
  e("rt-capability-registry->store-event-log", "rt-capability-registry", "store-event-log", "imports"),
  e("rt-auth-state->store-lib", "rt-auth-state", "store-lib", "imports"),
  e("store-lib->store-fs", "store-lib", "store-fs", "owns"),
  e("store-lib->store-file-index", "store-lib", "store-file-index", "owns"),

  // Runtime -> Network
  e("rt-edgerun-router->net-mesh", "rt-edgerun-router", "net-mesh", "imports"),
  e("rt-platform-provider->net-tcp-server", "rt-platform-provider", "net-tcp-server", "imports"),
  e("net-mesh->net-mesh-link", "net-mesh", "net-mesh-link", "owns"),
  e("net-mesh->net-mesh-session", "net-mesh", "net-mesh-session", "owns"),
  e("net-mesh-daemon->net-mesh", "net-mesh-daemon", "net-mesh", "owns"),
  e("net-quic->net-tls", "net-quic", "net-tls", "imports"),

  // Tests -> Protocol/Runtime
  e("test-core->proto-common", "test-core", "proto-common", "tests"),
  e("test-stream->proto-stream", "test-stream", "proto-stream", "tests"),
  e("test-storage->store-event-log", "test-storage", "store-event-log", "tests"),
  e("test-node->rt-platform-provider", "test-node", "rt-platform-provider", "tests"),
  e("test-core->crypto-core", "test-core", "crypto-core", "tests"),

  // Agent -> UI/Runtime
  e("agent-capability->rt-capability-registry", "agent-capability", "rt-capability-registry", "calls"),
  e("agent-capability->ui-command-input", "agent-capability", "ui-command-input", "sends"),
  e("agent-policy->agent-capability", "agent-policy", "agent-capability", "owns"),
  e("agent-registry->agent-capability", "agent-registry", "agent-capability", "owns"),
  e("agent-policy->proto-access", "agent-policy", "proto-access", "imports"),

  // Event flow
  e("event-stream->proto-stream", "event-stream", "proto-stream", "implements"),
  e("event-genesis->proto-identity", "event-genesis", "proto-identity", "implements"),
  e("event-command->proto-access", "event-command", "proto-access", "implements"),

  // Cross-layer
  e("rt-edgerun-router->ui-desktop", "rt-edgerun-router", "ui-desktop", "observed_flow"),
  e("api-terminal->rt-process", "api-terminal", "rt-process", "calls"),
  e("net-mesh-daemon->rt-machine", "net-mesh-daemon", "rt-machine", "owns"),
  e("crypto-core->store-lib", "crypto-core", "store-lib", "stores"),
  e("crypto-signing->proto-identity", "crypto-signing", "proto-identity", "signs"),
  e("proto-trust->crypto-core", "proto-trust", "crypto-core", "verifies"),

  // UI internal
  e("ui-desktop->ui-window", "ui-desktop", "ui-window", "owns"),
  e("ui-desktop->ui-topbar", "ui-desktop", "ui-topbar", "owns"),
  e("ui-desktop->ui-widget-panel", "ui-desktop", "ui-widget-panel", "owns"),
  e("ui-desktop->ui-stage-manager", "ui-desktop", "ui-stage-manager", "owns"),
  e("ui-page->ui-desktop", "ui-page", "ui-desktop", "calls"),
  e("ui-context-menu->ui-tooltip", "ui-context-menu", "ui-tooltip", "imports"),

  // Protocol internal
  e("proto-common->proto-identity", "proto-common", "proto-identity", "imports"),
  e("proto-identity->proto-trust", "proto-identity", "proto-trust", "imports"),
  e("proto-object->proto-stream", "proto-object", "proto-stream", "imports"),
  e("proto-network->proto-access", "proto-network", "proto-access", "imports"),
]

function generateRuntimeStats(): Map<string, RuntimeNodeStats> {
  const stats = new Map<string, RuntimeNodeStats>()

  const now = Date.now() * 1000

  for (const node of allNodes) {
    stats.set(node.id, {
      hits: Math.floor(Math.random() * 50),
      openSpans: 0,
      errors: 0,
      totalUs: Math.floor(Math.random() * 5000),
      maxUs: Math.floor(Math.random() * 500),
      lastTsUs: now,
    })
  }

  // Hot node: rt-edgerun-router with high hits
  stats.set("rt-edgerun-router", { hits: 15420, openSpans: 2, errors: 0, totalUs: 890_000, maxUs: 12_400, lastTsUs: now })

  // Hot node: api-chat with high hits
  stats.set("api-chat", { hits: 8930, openSpans: 0, errors: 0, totalUs: 2_340_000, maxUs: 45_200, lastTsUs: now })

  // Hot node: ui-desktop
  stats.set("ui-desktop", { hits: 6200, openSpans: 1, errors: 0, totalUs: 1_100_000, maxUs: 8_900, lastTsUs: now })

  // Slow node: net-mesh-session
  stats.set("net-mesh-session", { hits: 340, openSpans: 0, errors: 0, totalUs: 5_800_000, maxUs: 890_000, lastTsUs: now })

  // Error node: api-gmail-auth
  stats.set("api-gmail-auth", { hits: 120, openSpans: 0, errors: 14, totalUs: 450_000, maxUs: 120_000, lastTsUs: now })

  // Open span / stuck node: rt-wasm-loader
  stats.set("rt-wasm-loader", { hits: 5, openSpans: 3, errors: 0, totalUs: 12_000_000, maxUs: 12_000_000, lastTsUs: now - 30_000_000 })

  return stats
}

export function createMockGraph(): { nodes: Map<string, XrayNode>; edges: XrayEdge[] } {
  const nodeMap = new Map<string, XrayNode>()
  for (const node of allNodes) {
    nodeMap.set(node.id, { ...node, x: 0, y: 0 })
  }
  return { nodes: nodeMap, edges }
}

export function createMockRuntimeStats(): Map<string, RuntimeNodeStats> {
  return generateRuntimeStats()
}
