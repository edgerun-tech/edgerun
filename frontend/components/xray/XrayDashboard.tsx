"use client";

import { useState } from "react";
import dynamic from "next/dynamic";
import { LayoutDashboard, Globe } from "lucide-react";

/**
 * XrayDashboard - Dashboard component that can replace the globe.
 * Allows toggling between globe and xray graph visualization.
 * This is the integration point for replacing the globe with xray.
 */
const XrayWorkspace = dynamic(
  () => import("@/features/xray/XrayWorkspace").then(mod => ({ default: mod.XrayWorkspace })),
  { ssr: false }
);

export default function XrayDashboard(props: {
  globeNodeCount?: number;
  initialGraph?: any;
  className?: string;
}) {
  const [mode, setMode] = useState<"globe" | "xray">("globe");

  return (
    <div className={`relative w-full h-full ${props.className || ""}`}>
      {/* Mode toggle */}
      <div className="absolute top-4 right-4 z-20 flex gap-2">
        <button
          onClick={() => setMode("globe")}
          className={`p-2 rounded-lg border transition-colors ${
            mode === "globe"
              ? "bg-blue-600 border-blue-500 text-white"
              : "bg-gray-800/80 border-gray-600 text-gray-400 hover:text-white"
          }`}
          title="Globe view"
        >
          <Globe className="h-4 w-4" />
        </button>
        <button
          onClick={() => setMode("xray")}
          className={`p-2 rounded-lg border transition-colors ${
            mode === "xray"
              ? "bg-blue-600 border-blue-500 text-white"
              : "bg-gray-800/80 border-gray-600 text-gray-400 hover:text-white"
          }`}
          title="Xray graph view"
        >
          <LayoutDashboard className="h-4 w-4" />
        </button>
      </div>
      {/* Xray Workspace (new) */}
      {mode === "xray" && (
        <div className="absolute inset-0">
          <XrayWorkspace />
        </div>
      )}
    </div>
  );
}
