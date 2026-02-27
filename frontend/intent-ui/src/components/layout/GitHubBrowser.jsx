import { For, Show, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import {
  TbOutlineChevronRight,
  TbOutlineExternalLink,
  TbOutlineFileText,
  TbOutlineFolder,
  TbOutlineFolderOpen,
  TbOutlineReload
} from "solid-icons/tb";
import { getFsMounts, lastFsError, listFsDir, openFsFile } from "../../stores/fs";
import { openWindow } from "../../stores/windows";

const ROOT_PATH = "/github";

function GitHubTreeNode(props) {
  const isDir = () => props.node.kind === "dir";
  const isSelected = () => props.selectedPath() === props.node.path;
  const expanded = createMemo(() => props.expandedDirs().has(props.node.path));
  const children = createMemo(() => props.childrenByPath()[props.node.path] || []);

  const handleActivate = async () => {
    await props.activateNode(props.node);
  };

  return (
    <div>
      <div class="group rounded-md border border-transparent px-1 py-0.5 hover:bg-neutral-800/70">
        <button
          type="button"
          class={`flex w-full items-center gap-1.5 rounded px-1 py-1 text-left text-xs ${
            isSelected() ? "bg-blue-600/20 text-blue-100" : "text-neutral-300"
          }`}
          style={{ "padding-left": `${Math.max(props.depth, 0) * 12 + 4}px` }}
          onClick={handleActivate}
        >
          <span class="text-neutral-500">
            <Show when={isDir()} fallback={<TbOutlineFileText size={14} />}>
              <Show when={expanded()} fallback={<TbOutlineChevronRight size={14} />}>
                <TbOutlineChevronRight size={14} class="rotate-90" />
              </Show>
            </Show>
          </span>
          <span class="text-neutral-400">
            <Show when={isDir()} fallback={<TbOutlineFileText size={14} />}>
              <Show when={expanded()} fallback={<TbOutlineFolder size={14} />}>
                <TbOutlineFolderOpen size={14} />
              </Show>
            </Show>
          </span>
          <span class="truncate">{props.node.name}</span>
        </button>
      </div>

      <Show when={isDir() && expanded()}>
        <For each={children()}>
          {(child) => (
            <GitHubTreeNode
              node={child}
              depth={props.depth + 1}
              selectedPath={props.selectedPath}
              expandedDirs={props.expandedDirs}
              childrenByPath={props.childrenByPath}
              activateNode={props.activateNode}
            />
          )}
        </For>
      </Show>
    </div>
  );
}

function GitHubBrowser() {
  const [authState, setAuthState] = createSignal("needs-auth");
  const [searchQuery, setSearchQuery] = createSignal("");
  const [selectedPath, setSelectedPath] = createSignal(ROOT_PATH);
  const [expandedDirs, setExpandedDirs] = createSignal(new Set([ROOT_PATH]));
  const [childrenByPath, setChildrenByPath] = createSignal({});
  const [isRefreshing, setIsRefreshing] = createSignal(false);
  let onStorageHandler;

  const matches = createMemo(() => {
    const needle = searchQuery().trim().toLowerCase();
    if (!needle) return [];
    const allEntries = Object.values(childrenByPath()).flat();
    return allEntries
      .filter((entry) => entry.path !== ROOT_PATH)
      .filter((entry) => entry.name.toLowerCase().includes(needle) || entry.path.toLowerCase().includes(needle))
      .slice(0, 16);
  });

  const refreshAuth = () => {
    const githubMount = getFsMounts().find((mount) => mount.id === "github");
    setAuthState(githubMount?.auth || "error");
  };

  const loadChildren = async (path, force = false) => {
    if (!force && childrenByPath()[path]) return;
    const entries = await listFsDir(path);
    setChildrenByPath((prev) => ({ ...prev, [path]: entries }));
    refreshAuth();
  };

  const refreshTree = async () => {
    setIsRefreshing(true);
    await loadChildren(ROOT_PATH, true);
    setIsRefreshing(false);
  };

  const toggleDir = async (path) => {
    const next = new Set(expandedDirs());
    if (next.has(path)) {
      next.delete(path);
      setExpandedDirs(next);
      return;
    }
    next.add(path);
    setExpandedDirs(next);
    await loadChildren(path);
  };

  const activateNode = async (node) => {
    setSelectedPath(node.path);
    if (node.kind === "dir") {
      await toggleDir(node.path);
      return;
    }
    await openFsFile(node.path);
    openWindow("editor");
  };

  const openPath = async (entry) => {
    if (entry.kind === "dir") {
      setSelectedPath(entry.path);
      const next = new Set(expandedDirs());
      next.add(ROOT_PATH);
      next.add(entry.path);
      setExpandedDirs(next);
      await loadChildren(ROOT_PATH);
      await loadChildren(entry.path);
      return;
    }
    setSelectedPath(entry.path);
    await openFsFile(entry.path);
    openWindow("editor");
  };

  onMount(async () => {
    refreshAuth();
    await loadChildren(ROOT_PATH, true);
    onStorageHandler = (event) => {
      if (event.key && event.key !== "github_token") return;
      refreshAuth();
    };
    window.addEventListener("storage", onStorageHandler);
  });
  onCleanup(() => {
    if (onStorageHandler) {
      window.removeEventListener("storage", onStorageHandler);
    }
  });

  return (
    <div class="flex h-full flex-col bg-[#111] p-4 text-sm text-neutral-300">
      <div class="mb-3 flex items-center justify-between gap-2">
        <div>
          <h3 class="text-sm font-medium text-white">GitHub Browser</h3>
          <p class="text-xs text-neutral-500">{authState() === "ready" ? "Connected" : "Not connected"}</p>
        </div>
        <div class="flex items-center gap-2">
          <button
            type="button"
            class="inline-flex items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2 py-1 text-xs text-neutral-200 hover:bg-neutral-800"
            onClick={refreshTree}
          >
            <TbOutlineReload size={12} />
            {isRefreshing() ? "Refreshing..." : "Refresh"}
          </button>
          <Show when={authState() !== "ready"}>
            <button
              type="button"
              class="inline-flex items-center gap-1 rounded-md border border-amber-500/40 bg-amber-500/10 px-2 py-1 text-xs text-amber-200 hover:bg-amber-500/20"
              onClick={() => openWindow("integrations")}
            >
              <TbOutlineExternalLink size={12} />
              Connect
            </button>
          </Show>
        </div>
      </div>

      <input
        type="text"
        value={searchQuery()}
        onInput={(event) => setSearchQuery(event.currentTarget.value)}
        placeholder="Search files and folders..."
        class="mb-3 w-full rounded-md border border-neutral-700 bg-[#0b0c0f] px-2 py-2 text-xs text-neutral-100 placeholder:text-neutral-500 focus:border-neutral-500 focus:outline-none"
      />

      <Show when={searchQuery().trim()}>
        <div class="mb-3 max-h-36 space-y-1 overflow-y-auto rounded-md border border-neutral-800 bg-neutral-900/50 p-2">
          <For each={matches()}>
            {(entry) => (
              <button
                type="button"
                class="flex w-full items-center gap-2 rounded px-1 py-1 text-left text-xs text-neutral-300 hover:bg-neutral-800/80"
                onClick={() => openPath(entry)}
              >
                <span class="text-neutral-400">
                  <Show when={entry.kind === "dir"} fallback={<TbOutlineFileText size={13} />}>
                    <TbOutlineFolder size={13} />
                  </Show>
                </span>
                <span class="truncate">{entry.path}</span>
              </button>
            )}
          </For>
          <Show when={matches().length === 0}>
            <p class="px-1 py-1 text-xs text-neutral-500">No matches in loaded GitHub tree.</p>
          </Show>
        </div>
      </Show>

      <div class="min-h-0 flex-1 overflow-auto rounded-md border border-neutral-800 bg-neutral-950/40 p-2">
        <For each={childrenByPath()[ROOT_PATH] || []}>
          {(node) => (
            <GitHubTreeNode
              node={node}
              depth={0}
              selectedPath={selectedPath}
              expandedDirs={expandedDirs}
              childrenByPath={childrenByPath}
              activateNode={activateNode}
            />
          )}
        </For>
      </div>

      <div class="mt-3 rounded-md border border-neutral-800 bg-neutral-900/50 p-2 text-[11px] text-neutral-400">
        <p class="truncate">Selected: {selectedPath()}</p>
        <Show when={lastFsError()}>
          <p class="mt-1 truncate text-rose-300">FS: {lastFsError()}</p>
        </Show>
      </div>
    </div>
  );
}
export {
  GitHubBrowser as default
};
