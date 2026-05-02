"use client";

import { useState } from "react";

/**
 * XrayCommandSurface - Bottom control surface for layout/runtime/timeline.
 * Provides controls for graph visualization modes and runtime overlay.
 */
export default function XrayCommandSurface(props: {
  runtimeMode: boolean;
  onRuntimeModeChange: (enabled: boolean) => void;
  layoutType: "force" | "grid" | "hierarchical";
  onLayoutTypeChange: (layout: "force" | "grid" | "hierarchical") => void;
}) {
  return (
    <div className="h-12 border-t border-border flex items-center px-4 gap-4 bg-background/50">
      {/* Layout controls */}
      <div className="flex items-center gap-2">
        <span className="text-xs text-muted-foreground">Layout:</span>
        <select
          value={props.layoutType}
          onChange={(e) => props.onLayoutTypeChange(e.currentTarget.value as any)}
          className="text-xs bg-card text-card-foreground rounded px-2 py-1 border border-border"
        >
          <option value="force">Force-Directed</option>
          <option value="grid">Grid</option>
          <option value="hierarchical">Hierarchical</option>
        </select>
      </div>

      {/* Runtime mode toggle */}
      <div className="flex items-center gap-2">
        <span className="text-xs text-muted-foreground">Runtime:</span>
        <button
          onClick={() => props.onRuntimeModeChange(!props.runtimeMode)}
          className={`text-xs px-3 py-1 rounded border ${
            props.runtimeMode
              ? "bg-green-600 border-green-500 text-white"
              : "bg-card border-border text-muted-foreground"
          }`}
        >
          {props.runtimeMode ? "ON" : "OFF"}
        </button>
        {props.runtimeMode && (
          <span className="text-xs text-green-400 animate-pulse">
            Runtime overlay active
          </span>
        )}
      </div>

      {/* Placeholder for timeline controls */}
      <div className="flex-1" />

      <div className="text-xs text-muted-foreground">
        Xray v0.1
      </div>
    </div>
  );
}
