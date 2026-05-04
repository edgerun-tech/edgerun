/**
 * Xray feature barrel export.
 * Clean EdgeRun xray architecture extracted from codeanalyzer.
 */

export { XrayWorkspace } from "./XrayWorkspace"
export { XrayViewport } from "./XrayViewport"
export { XrayCommandSurface } from "./XrayCommandSurface"
export { XrayInspector } from "./XrayInspector"

// Types
export type { XrayGraph, XrayNode, XrayEdge } from "../../lib/xray/types/XrayGraph";

// Adapter
export { adaptCodeanalyzerToXray, adaptNodes, adaptEdges } from "../../lib/xray/adapter/codeanalyzerAdapter";

// Renderer
export { createGraphRenderer } from "../../lib/xray/render/GraphRenderer";
export { buildNodeProgram, buildEdgeProgram, getLangColor, screenToGraph, graphToScreen } from "../../lib/xray/render/gl";

// Layout
export { createLayoutState, stepLayout, buildSpatialGrid, initPositions } from "../../lib/xray/layout/forceLayout";

// Input
export { hitTest, boxSelect } from "../../lib/xray/input/hitTest";

// Color policy
export { getNodeColor, getNodeSize, getEdgeColor } from "../../lib/xray/render/colorPolicy";
