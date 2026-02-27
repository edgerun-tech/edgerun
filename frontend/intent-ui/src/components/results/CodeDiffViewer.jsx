import { createSignal, Show, For, onMount } from "solid-js";
import { Motion } from "solid-motionone";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import {
  TbOutlineGitCommit,
  TbOutlineColumns,
  TbOutlineLayoutGrid
} from "solid-icons/tb";
function cn(...classes) {
  return twMerge(clsx(classes));
}
function parseDiffData(data) {
  if (Array.isArray(data)) {
    return data;
  }
  if (data?.hunks) {
    return [data];
  }
  if (typeof data === "string") {
    return parseUnifiedDiff(data);
  }
  return [];
}
function parseUnifiedDiff(diffText) {
  const files = [];
  let currentFile = null;
  let currentHunk = null;
  const lines = diffText.split("\n");
  for (const line of lines) {
    if (line.startsWith("diff --git")) {
      if (currentFile) {
        files.push(currentFile);
      }
      currentFile = {
        path: "",
        additions: 0,
        deletions: 0,
        hunks: []
      };
      currentHunk = null;
    }
    if (line.startsWith("--- a/")) {
      if (currentFile) currentFile.oldPath = line.slice(6);
    }
    if (line.startsWith("+++ b/")) {
      if (currentFile) currentFile.path = line.slice(6);
    }
    if (line.startsWith("@@")) {
      if (currentHunk && currentFile) {
        currentFile.hunks.push(currentHunk);
      }
      const match = line.match(/@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/);
      currentHunk = {
        oldStart: match ? parseInt(match[1]) : void 0,
        oldLines: match && match[2] ? parseInt(match[2]) : void 0,
        newStart: match ? parseInt(match[3]) : void 0,
        newLines: match && match[4] ? parseInt(match[4]) : void 0,
        lines: []
      };
    }
    if (currentHunk) {
      if (line.startsWith("+") && !line.startsWith("+++")) {
        currentHunk.lines.push({
          type: "add",
          content: line.slice(1),
          newNumber: currentHunk.newLines ? (currentHunk.newStart || 0) + currentHunk.newLines : void 0
        });
        if (currentFile) currentFile.additions++;
      } else if (line.startsWith("-") && !line.startsWith("---")) {
        currentHunk.lines.push({
          type: "remove",
          content: line.slice(1),
          oldNumber: currentHunk.oldLines ? (currentHunk.oldStart || 0) + currentHunk.oldLines : void 0
        });
        if (currentFile) currentFile.deletions++;
      } else if (line.startsWith(" ")) {
        currentHunk.lines.push({
          type: "context",
          content: line.slice(1)
        });
      }
    }
  }
  if (currentFile) {
    files.push(currentFile);
  }
  return files;
}
function CodeDiffViewer(props) {
  const ui = () => props.response.ui;
  const [viewMode, setViewMode] = createSignal("unified");
  const [expandedFiles, setExpandedFiles] = createSignal(/* @__PURE__ */ new Set());
  const [diffs, setDiffs] = createSignal([]);
  onMount(() => {
    const parsed = parseDiffData(props.response.data);
    setDiffs(parsed);
    setExpandedFiles(new Set(parsed.map((d) => d.path)));
  });
  const toggleFile = (path) => {
    const current = expandedFiles();
    const newSet = new Set(current);
    if (newSet.has(path)) {
      newSet.delete(path);
    } else {
      newSet.add(path);
    }
    setExpandedFiles(newSet);
  };
  const totalAdditions = () => diffs().reduce((sum, d) => sum + d.additions, 0);
  const totalDeletions = () => diffs().reduce((sum, d) => sum + d.deletions, 0);
  return <Motion.div
    initial={{ opacity: 0, y: 8 }}
    animate={{ opacity: 1, y: 0 }}
    exit={{ opacity: 0, y: -8 }}
    transition={{ duration: 0.2 }}
    class={cn(
      "bg-neutral-800/50 rounded-xl border border-neutral-700 overflow-hidden",
      props.class
    )}
  >
      {
    /* Header */
  }
      <div class="px-4 py-3 border-b border-neutral-700 bg-neutral-800/50">
        <div class="flex items-center justify-between gap-3">
          <div class="flex items-center gap-3">
            <TbOutlineGitCommit size={18} class="text-blue-400" />
            <Show when={ui()?.title}>
              <h3 class="text-sm font-medium text-white">{ui().title}</h3>
            </Show>
            <div class="flex items-center gap-2 text-xs">
              <span class="text-green-400">+{totalAdditions()}</span>
              <span class="text-red-400">-{totalDeletions()}</span>
            </div>
          </div>
          
          <div class="flex items-center gap-2">
            {
    /* View mode toggle */
  }
            <div class="flex items-center gap-1 bg-neutral-900 rounded-lg p-1" role="group" aria-label="Diff view mode">
              <button
    type="button"
    onClick={() => setViewMode("unified")}
    class={cn(
      "p-1.5 rounded transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900",
      viewMode() === "unified" ? "bg-neutral-700 text-white" : "text-neutral-400 hover:text-white"
    )}
    title="Unified view"
    aria-label="Unified view"
    aria-pressed={viewMode() === "unified"}
  >
                <TbOutlineColumns size={16} />
              </button>
              <button
    type="button"
    onClick={() => setViewMode("split")}
    class={cn(
      "p-1.5 rounded transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900",
      viewMode() === "split" ? "bg-neutral-700 text-white" : "text-neutral-400 hover:text-white"
    )}
    title="Split view"
    aria-label="Split view"
    aria-pressed={viewMode() === "split"}
  >
                <TbOutlineLayoutGrid size={16} />
              </button>
            </div>
          </div>
        </div>
      </div>

      {
    /* File list */
  }
      <div class="border-b border-neutral-700 bg-neutral-900/30">
        <For each={diffs()}>
          {(fileDiff) => <div>
              <button
    type="button"
    onClick={() => toggleFile(fileDiff.path)}
    class="w-full px-4 py-2 flex items-center gap-3 hover:bg-neutral-800/50 transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900"
    aria-expanded={expandedFiles().has(fileDiff.path)}
    aria-label={`Toggle diff for ${fileDiff.path}`}
  >
                <span class={cn(
    "w-1.5 h-1.5 rounded-full",
    fileDiff.additions > 0 ? "bg-green-500" : "bg-neutral-600"
  )} />
                <span class="text-sm text-neutral-300 flex-1 text-left font-mono truncate">
                  {fileDiff.path}
                </span>
                <span class="text-xs text-green-400">+{fileDiff.additions}</span>
                <span class="text-xs text-red-400">-{fileDiff.deletions}</span>
              </button>
            </div>}
        </For>
      </div>

      {
    /* Diff content */
  }
      <div class="overflow-auto max-h-[600px]">
        <For each={diffs()}>
          {(fileDiff) => <Show when={expandedFiles().has(fileDiff.path)}>
              <div class="border-b border-neutral-700 last:border-b-0">
                <For each={fileDiff.hunks}>
                  {(hunk) => <div class="font-mono text-sm">
                      {
    /* Hunk header */
  }
                      <div class="bg-neutral-900/50 px-4 py-1 text-xs text-neutral-500">
                        @@ -{hunk.oldStart},{hunk.oldLines} +{hunk.newStart},{hunk.newLines} @@
                      </div>
                      
                      {
    /* Lines */
  }
                      <Show
    when={viewMode() === "unified"}
    fallback={
      /* Split view */
      <div class="flex">
                            {
        /* Old (removed) */
      }
                            <div class="flex-1 border-r border-neutral-700 overflow-x-auto">
                              <For each={hunk.lines.filter((l) => l.type === "remove" || l.type === "context")}>
                                {(line) => <div class={cn(
        "flex",
        line.type === "context" ? "bg-transparent" : "bg-red-900/20"
      )}>
                                    <span class="w-12 flex-shrink-0 text-right pr-2 text-neutral-600 select-none">
                                      {line.oldNumber || ""}
                                    </span>
                                    <pre class={cn(
        "flex-1 py-0.5 px-2 whitespace-pre overflow-x-auto",
        line.type === "remove" ? "text-red-300" : "text-neutral-400"
      )}>
                                      {line.type === "remove" && <span class="select-none">-</span>}
                                      {line.content}
                                    </pre>
                                  </div>}
                              </For>
                            </div>
                            
                            {
        /* New (added) */
      }
                            <div class="flex-1 overflow-x-auto">
                              <For each={hunk.lines.filter((l) => l.type === "add" || l.type === "context")}>
                                {(line) => <div class={cn(
        "flex",
        line.type === "context" ? "bg-transparent" : "bg-green-900/20"
      )}>
                                    <span class="w-12 flex-shrink-0 text-right pr-2 text-neutral-600 select-none">
                                      {line.newNumber || ""}
                                    </span>
                                    <pre class={cn(
        "flex-1 py-0.5 px-2 whitespace-pre overflow-x-auto",
        line.type === "add" ? "text-green-300" : "text-neutral-400"
      )}>
                                      {line.type === "add" && <span class="select-none">+</span>}
                                      {line.content}
                                    </pre>
                                  </div>}
                              </For>
                            </div>
                          </div>
    }
  >
                        {
    /* Unified view */
  }
                        <For each={hunk.lines}>
                          {(line) => <div class={cn(
    "flex",
    line.type === "add" ? "bg-green-900/20" : line.type === "remove" ? "bg-red-900/20" : "bg-transparent"
  )}>
                              <span class="w-12 flex-shrink-0 text-right pr-2 text-neutral-600 select-none text-xs py-0.5">
                                {line.type === "add" ? line.newNumber : line.type === "remove" ? line.oldNumber : ""}
                              </span>
                              <pre class={cn(
    "flex-1 py-0.5 px-2 whitespace-pre overflow-x-auto",
    line.type === "add" ? "text-green-300" : line.type === "remove" ? "text-red-300" : "text-neutral-400"
  )}>
                                {line.type === "add" && <span class="select-none">+</span>}
                                {line.type === "remove" && <span class="select-none">-</span>}
                                {line.type === "context" && <span class="select-none"> </span>}
                                {line.content}
                              </pre>
                            </div>}
                        </For>
                      </Show>
                    </div>}
                </For>
              </div>
            </Show>}
        </For>
        
        <Show when={diffs().length === 0}>
          <div class="text-center py-12 text-neutral-500">
            <TbOutlineGitCommit size={48} class="mx-auto mb-3 opacity-50" />
            <p class="text-sm">No changes to display</p>
          </div>
        </Show>
      </div>
    </Motion.div>;
}
export {
  CodeDiffViewer
};
