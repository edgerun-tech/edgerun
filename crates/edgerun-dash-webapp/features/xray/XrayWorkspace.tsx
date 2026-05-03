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
  mode?: "full" | "bg";
}) {
  const bgMode = props.mode === "bg";
  const [xrayGraph, setXrayGraph] = useState<any>(null);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [runtimeMode, setRuntimeMode] = useState(false);
  const [layoutType, setLayoutType] = useState<"force" | "grid" | "hierarchical">("force");
  const [highlightedNodeIds, setHighlightedNodeIds] = useState<Set<string>>(new Set());
  const [viewportCommand, setViewportCommand] = useState<any>(null);

  useEffect(() => {
    (window as any).xray = {
      getState: () => ({
        selectedNode: selectedNodeId,
        runtimeMode,
        layoutType,
        highlightedNodeIds: Array.from(highlightedNodeIds),
        graphNodeCount: xrayGraph?.nodes.size ?? 0,
        graphEdgeCount: xrayGraph?.edges.length ?? 0,
      }),
      command: (cmd: any) => handleCommand(cmd),
      getVisibleGraph: () => xrayGraph,
      getSelection: () => selectedNodeId,
    };
  }, [selectedNodeId, runtimeMode, layoutType, highlightedNodeIds, xrayGraph]);

  useEffect(() => {
    if (props.initialGraph) {
      const graph = props.initialGraph.nodes instanceof Map
        ? adaptCodeanalyzerToXray(props.initialGraph)
        : props.initialGraph;
      setXrayGraph(graph);
    }
  }, [props.initialGraph]);

  function sendViewportCommand(cmd: any) {
    setViewportCommand({ ...cmd, nonce: Date.now() + Math.random() });
  }

  function handleCommand(cmd: any) {
    switch (cmd.type) {
      case "focus_node":
        setSelectedNodeId(cmd.nodeId);
        sendViewportCommand(cmd);
        break;
      case "set_layout":
        setLayoutType(cmd.layout);
        break;
      case "set_runtime_mode":
        setRuntimeMode(Boolean(cmd.enabled));
        break;
      case "highlight_nodes":
        setHighlightedNodeIds(new Set(cmd.nodeIds ?? []));
        sendViewportCommand(cmd);
        break;
      case "clear_highlight":
        setHighlightedNodeIds(new Set());
        break;
      case "fit_view":
      case "reset_view":
        sendViewportCommand(cmd);
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
      <div className="flex flex-1 overflow-hidden">
        {!bgMode && (
          <div className="w-64 border-r border-border bg-card/50 overflow-y-auto hidden md:block">
            <div className="p-4 space-y-4">
              <div>
                <h3 className="text-sm font-semibold text-foreground mb-2">Graph</h3>
                <div className="space-y-1 text-xs text-muted-foreground">
                  <p>{xrayGraph?.nodes.size ?? 0} nodes</p>
                  <p>{xrayGraph?.edges.length ?? 0} edges</p>
                  <p>{highlightedNodeIds.size} highlighted</p>
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
              <div className="text-xs text-muted-foreground space-y-1">
                <p>Console:</p>
                <code className="block rounded border border-border bg-background/70 p-2 text-[10px]">
                  window.xray.command({`{ type: "fit_view" }`})
                </code>
              </div>
            </div>
          </div>
        )}

        <div className="flex-1 relative">
          <XrayViewport
            graph={xrayGraph}
            runtimeMode={runtimeMode}
            layoutType={layoutType}
            externalHighlightedIds={highlightedNodeIds}
            viewportCommand={viewportCommand}
            onNodeSelect={handleNodeSelect}
          />
        </div>

        {!bgMode && (
          <div className="w-80 border-l border-border bg-card/50 overflow-y-auto">
            <XrayInspector
              nodeId={selectedNodeId}
              graph={xrayGraph}
            />
          </div>
        )}
      </div>

      {!bgMode && (
        <XrayCommandSurface
          runtimeMode={runtimeMode}
          onRuntimeModeChange={setRuntimeMode}
          layoutType={layoutType}
          onLayoutTypeChange={setLayoutType}
        />
      )}
    </div>
  );
}
