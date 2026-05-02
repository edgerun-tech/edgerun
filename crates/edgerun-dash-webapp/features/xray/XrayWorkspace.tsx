"use client";

import { useState, useEffect } from "react";
import XrayViewport from "./XrayViewport";
import XrayCommandSurface from "./XrayCommandSurface";
import XrayInspector from "./XrayInspector";
import { adaptCodeanalyzerToXray } from "../../lib/xray/adapter/codeanalyzerAdapter";

/**
 * XrayWorkspace - Main workspace component for EdgeRun xray feature.
 * Center graph view with left/right panels and bottom command surface.
 */
export default function XrayWorkspace(props: {
  initialGraph?: any;
  className?: string;
}) {
  const [xrayGraph, setXrayGraph] = useState<any>(null);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [runtimeMode, setRuntimeMode] = useState(false);
  const [layoutType, setLayoutType] = useState<"force" | "grid" | "hierarchical">("force");

  // Semantic command API exposed on window.xray
  useEffect(() => {
    (window as any).xray = {
      getState: () => ({
        selectedNode: selectedNodeId,
        runtimeMode,
        layoutType,
        graphNodeCount: xrayGraph?.nodes.size ?? 0,
        graphEdgeCount: xrayGraph?.edges.length ?? 0,
      }),
      command: (cmd: any) => handleCommand(cmd),
    };
  }, [selectedNodeId, runtimeMode, layoutType, xrayGraph]);

  // Adapt initial graph data
  useEffect(() => {
    if (props.initialGraph) {
      const graph = props.initialGraph.nodes instanceof Map
        ? adaptCodeanalyzerToXray(props.initialGraph)
        : props.initialGraph;
      setXrayGraph(graph);
    }
  }, [props.initialGraph]);

  function handleCommand(cmd: any) {
    switch (cmd.type) {
      case "focus_node":
        setSelectedNodeId(cmd.nodeId);
        break;
      case "set_layout":
        setLayoutType(cmd.layout);
        break;
      case "set_runtime_mode":
        setRuntimeMode(cmd.enabled);
        break;
      case "highlight_nodes":
        break;
      case "clear_highlight":
        break;
      case "open_code_popup":
        break;
      default:
        console.warn("Unknown xray command:", cmd);
    }
  }

  function handleNodeSelect(nodeId: string | null) {
    setSelectedNodeId(nodeId);
  }

  return (
    <div className={`flex flex-col w-full h-full ${props.className || ""}`}>
      {/* Main content area */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left panel: graph context */}
        <div className="w-64 border-r border-border bg-card/50 overflow-y-auto hidden md:block">
          <div className="p-4 space-y-4">
            <div>
              <h3 className="text-sm font-semibold text-foreground mb-2">Graph</h3>
              <div className="space-y-1 text-xs text-muted-foreground">
                <p>{xrayGraph?.nodes.size ?? 0} nodes</p>
                <p>{xrayGraph?.edges.length ?? 0} edges</p>
              </div>
            </div>
            <div>
              <h3 className="text-sm font-semibold text-foreground mb-2">Layout</h3>
              <p className="text-xs text-muted-foreground capitalize">{layoutType}</p>
            </div>
            <div>
              <h3 className="text-sm font-semibold text-foreground mb-2">Mode</h3>
              <p className="text-xs text-muted-foreground">
                {runtimeMode ? "Runtime" : "Static"}
              </p>
            </div>
          </div>
        </div>

        {/* Center: XrayViewport */}
        <div className="flex-1 relative">
          <XrayViewport
            graph={xrayGraph}
            runtimeMode={runtimeMode}
            onNodeSelect={handleNodeSelect}
          />
        </div>

        {/* Right panel: inspector */}
        <div className="w-80 border-l border-border bg-card/50 overflow-y-auto">
          <XrayInspector
            nodeId={selectedNodeId}
            graph={xrayGraph}
          />
        </div>
      </div>

      {/* Bottom: Command surface */}
      <XrayCommandSurface
        runtimeMode={runtimeMode}
        onRuntimeModeChange={setRuntimeMode}
        layoutType={layoutType}
        onLayoutTypeChange={setLayoutType}
      />
    </div>
  );
}
