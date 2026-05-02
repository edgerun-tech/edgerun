"use client";

import { useMemo } from "react";

/**
 * XrayInspector - Right panel inspector for selected node.
 */
export default function XrayInspector(props: {
  nodeId: string | null;
  graph: any;
}) {
  const node = useMemo(() => {
    if (!props.nodeId || !props.graph) return null;
    return props.graph.nodes.get(props.nodeId) ?? null;
  }, [props.nodeId, props.graph]);

  function handleOpenCode() {
    if (!props.nodeId) return;
    if (typeof window !== "undefined" && (window as any).xray) {
      (window as any).xray.command({
        type: "open_code_popup",
        nodeId: props.nodeId,
      });
    }
  }

  if (!node) {
    return (
      <div className="p-4">
        <h3 className="text-sm font-semibold text-gray-300 mb-2">Inspector</h3>
        <p className="text-xs text-gray-500">Select a node to inspect</p>
      </div>
    );
  }

  return (
    <div className="p-4">
      <h3 className="text-sm font-semibold text-gray-300 mb-3">Inspector</h3>

      <div className="space-y-3">
        {/* Node info */}
        <div>
          <div className="text-xs text-gray-400">ID</div>
          <div className="text-sm text-gray-200 font-mono">{node.id}</div>
        </div>

        <div>
          <div className="text-xs text-gray-400">Label</div>
          <div className="text-sm text-gray-200">{node.label}</div>
        </div>

        <div className="flex gap-4">
          <div>
            <div className="text-xs text-gray-400">Kind</div>
            <div className="text-sm text-gray-200">{node.kind}</div>
          </div>
          <div>
            <div className="text-xs text-gray-400">Language</div>
            <div className="text-sm text-gray-200">{node.language}</div>
          </div>
        </div>

        {node.layer && (
          <div>
            <div className="text-xs text-gray-400">Layer</div>
            <div className="text-sm text-gray-200">{node.layer}</div>
          </div>
        )}

        {/* Tags */}
        {node.tags && node.tags.length > 0 && (
          <div>
            <div className="text-xs text-gray-400 mb-1">Tags</div>
            <div className="flex flex-wrap gap-1">
              {node.tags.map((tag: string) => (
                <span key={tag} className="text-xs bg-gray-700 text-gray-300 px-2 py-0.5 rounded">
                  {tag}
                </span>
              ))}
            </div>
          </div>
        )}

        {/* Runtime stats */}
        {node.runtime && (
          <div className="border-t border-gray-700 pt-3">
            <div className="text-xs text-gray-400 mb-2">Runtime Stats</div>
            <div className="grid grid-cols-2 gap-2 text-xs">
              <div className="text-gray-500">Hits:</div>
              <div className="text-gray-200">{node.runtime.hits ?? 0}</div>
              <div className="text-gray-500">Errors:</div>
              <div className="text-red-400">{node.runtime.errors ?? 0}</div>
              <div className="text-gray-500">Open Spans:</div>
              <div className="text-yellow-400">{node.runtime.openSpans ?? 0}</div>
              <div className="text-gray-500">Max (μs):</div>
              <div className="text-gray-200">{node.runtime.maxUs ?? 0}</div>
            </div>
          </div>
        )}

        {/* Source file */}
        {node.source && (
          <div>
            <div className="text-xs text-gray-400">Source</div>
            <div className="text-xs text-gray-400 font-mono break-all">{node.source}</div>
          </div>
        )}

        {/* Open code button */}
        <button
          onClick={handleOpenCode}
          className="w-full text-xs bg-blue-600 hover:bg-blue-500 text-white py-1.5 px-3 rounded"
        >
          View Code
        </button>
      </div>
    </div>
  );
}
