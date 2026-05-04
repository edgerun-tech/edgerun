/**
 * Canonical Xray feature barrel.
 *
 * Use this module for production xray imports. The older JS implementation
 * under `lib/xray/*` is legacy/scaffolding and should not be imported from new code.
 */

export { XrayWorkspace } from "./XrayWorkspace"
export { XrayViewport } from "./XrayViewport"
export { XrayCommandSurface } from "./XrayCommandSurface"
export { XrayInspector } from "./XrayInspector"

export { xrayState, resetView, setLayout, setRuntimeMode, setViewTransform, selectNode } from "./graph/graph-store"
export type { XrayNode, XrayEdge, RuntimeNodeStats, LayoutType, XrayState } from "./graph/types"

export { runForceLayout } from "./layout/force-layout"
export { runGlobeLayout } from "./layout/globe-layout"
export { runLayerLayout } from "./layout/layer-layout"
export { WebGLRenderer } from "./render/webgl-renderer"
export { getNodeColor, getNodeSize, getEdgeColor } from "./render/color-policy"
export { CodeAnalyzerWsService } from "./services/codeanalyzer-ws"
export type { CodeAnalyzerConfig, GraphData } from "./services/codeanalyzer-ws"
