import type { LayoutType } from "./types"
import {
  setLayout,
  setRuntimeMode,
  focusNode,
  highlightNodes,
  clearHighlight,
  resetView,
  getXrayState,
} from "./graph-store"

export type XrayCommand =
  | { type: "set_layout"; layout: LayoutType }
  | { type: "set_runtime_mode"; enabled: boolean }
  | { type: "focus_node"; nodeId: string }
  | { type: "highlight_nodes"; nodeIds: string[] }
  | { type: "clear_highlight" }
  | { type: "reset_view" }

export function executeCommand(cmd: XrayCommand) {
  switch (cmd.type) {
    case "set_layout":
      setLayout(cmd.layout)
      break
    case "set_runtime_mode":
      setRuntimeMode(cmd.enabled)
      break
    case "focus_node":
      focusNode(cmd.nodeId)
      break
    case "highlight_nodes":
      highlightNodes(cmd.nodeIds)
      break
    case "clear_highlight":
      clearHighlight()
      break
    case "reset_view":
      resetView()
      break
  }
}

export function registerGlobalApi() {
  if (typeof window !== "undefined") {
    ;(window as any).xray = {
      getState: getXrayState,
      command: executeCommand,
    }
  }
}
