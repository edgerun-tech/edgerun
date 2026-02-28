import { For, Show, createEffect, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import {
  TbOutlineArchive,
  TbOutlineBrandGithub,
  TbOutlineBrandGoogleDrive,
  TbOutlineClipboard,
  TbOutlineCopy,
  TbOutlineCut,
  TbOutlineDeviceDesktop,
  TbOutlineFileText,
  TbOutlineFolder,
  TbOutlineFolderOpen,
  TbOutlineFolderPlus,
  TbOutlineInfoCircle,
  TbOutlinePencil,
  TbOutlineSend,
  TbOutlineTrash
} from "solid-icons/tb";
import { openWindow } from "../../stores/windows";
import { openWorkflowIntegrations } from "../../stores/workflow-ui";
import { publishEvent } from "../../stores/eventbus";
import { knownDevices } from "../../stores/devices";
import { getFsNodeTargetId, setFsNodeTarget } from "../../stores/fs-node-target";
import { localBridgeHttpUrl } from "../../lib/local-bridge-origin";
import {
  copyFsPath,
  deleteFsPath,
  getFsMounts,
  lastFsError,
  listFsDir,
  mkdirFsPath,
  moveFsPath,
  openFsFile
} from "../../stores/fs";

const archivePattern = /\.(zip|tar|tar\.gz|tgz|tar\.bz2|tbz2)$/i;

function normalizePath(path) {
  if (!path) return "/";
  const cleaned = String(path).replace(/\/{2,}/g, "/");
  if (!cleaned.startsWith("/")) return `/${cleaned}`;
  return cleaned.length > 1 && cleaned.endsWith("/") ? cleaned.slice(0, -1) : cleaned;
}

function dirname(path) {
  const normalized = normalizePath(path);
  if (normalized === "/") return "/";
  const index = normalized.lastIndexOf("/");
  return index <= 0 ? "/" : normalized.slice(0, index);
}

function basename(path) {
  const normalized = normalizePath(path);
  if (normalized === "/") return "/";
  const index = normalized.lastIndexOf("/");
  return normalized.slice(index + 1);
}

function withName(dirPath, name) {
  const cleanDir = normalizePath(dirPath);
  const cleanName = String(name || "").trim().replace(/^\//, "");
  if (!cleanName) return cleanDir;
  return normalizePath(cleanDir === "/" ? `/${cleanName}` : `${cleanDir}/${cleanName}`);
}

function toLocalProviderPath(path) {
  const normalized = normalizePath(path);
  if (!normalized.startsWith("/local")) return "/";
  const providerPath = normalized.slice("/local".length) || "/";
  return normalizePath(providerPath);
}

const mountMeta = {
  ram: { icon: TbOutlineDeviceDesktop, tone: "text-violet-200", integrationId: "" },
  github: { icon: TbOutlineBrandGithub, tone: "text-neutral-200", integrationId: "github" },
  local: { icon: TbOutlineFolder, tone: "text-amber-200", integrationId: "" },
  drive: { icon: TbOutlineBrandGoogleDrive, tone: "text-neutral-200", integrationId: "google" }
};

function FileRow(props) {
  const isDir = () => props.node.kind === "dir";
  const isSelected = () => props.selectedPath() === props.node.path;
  const expanded = createMemo(() => props.expandedDirs().has(props.node.path));
  const children = createMemo(() => props.childrenByPath()[props.node.path] || []);
  const iconToneClass = createMemo(() => {
    if (isDir()) return expanded() ? "text-amber-300" : "text-amber-400";
    const name = String(props.node.name || "").toLowerCase();
    if (/\.(tsx?|jsx?)$/.test(name)) return "text-sky-400";
    if (/\.(json|ya?ml|toml|ini|env)$/.test(name)) return "text-emerald-400";
    if (/\.(md|txt)$/.test(name)) return "text-indigo-300";
    if (/\.(png|jpe?g|gif|webp|svg)$/.test(name)) return "text-violet-400";
    if (/\.(css|scss|sass)$/.test(name)) return "text-cyan-400";
    return "text-neutral-400";
  });

  return (
    <div>
      <div
        class={`group flex items-center justify-between rounded-md border px-2 py-2 text-xs transition-colors ${
          isSelected() ? "border-[hsl(var(--primary)/0.4)] bg-[hsl(var(--primary)/0.14)] text-[hsl(var(--primary))]" : "border-transparent bg-neutral-900/40 text-neutral-300 hover:bg-neutral-800/80"
        }`}
        onContextMenu={(event) => props.onContextRequest(props.node, event)}
      >
        <button type="button" onClick={() => props.onActivate(props.node)} class="flex min-w-0 flex-1 items-center gap-2 text-left">
          <span class={iconToneClass()}>
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
        <div class="mt-1 space-y-1 pl-1.5">
          <For each={children()}>
            {(child) => (
              <FileRow
                node={child}
                selectedPath={props.selectedPath}
                setSelectedPath={props.setSelectedPath}
                expandedDirs={props.expandedDirs}
                childrenByPath={props.childrenByPath}
                onActivate={props.onActivate}
                onCreateFolder={props.onCreateFolder}
                onRename={props.onRename}
                onDelete={props.onDelete}
                onCut={props.onCut}
                onCopy={props.onCopy}
                onPaste={props.onPaste}
                canPasteTo={props.canPasteTo}
                canExtract={props.canExtract}
                onExtract={props.onExtract}
                canArchive={props.canArchive}
                onArchive={props.onArchive}
                onSendToEditor={props.onSendToEditor}
                onSendToTerminal={props.onSendToTerminal}
                onCopyPath={props.onCopyPath}
                onProperties={props.onProperties}
                onContextRequest={props.onContextRequest}
              />
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}

function FileManager(props) {
  const [mounts, setMounts] = createSignal(getFsMounts());
  const [selectedPath, setSelectedPath] = createSignal("/");
  const [selectedMountId, setSelectedMountId] = createSignal("");
  const [expandedDirs, setExpandedDirs] = createSignal(new Set());
  const [childrenByPath, setChildrenByPath] = createSignal({});
  const [localFsRoot, setLocalFsRoot] = createSignal("");
  const [actionStatus, setActionStatus] = createSignal("");
  const [propertiesNode, setPropertiesNode] = createSignal(null);
  const [clipboard, setClipboard] = createSignal(null);
  const [contextMenu, setContextMenu] = createSignal({ open: false, x: 0, y: 0, node: null });
  const compact = () => Boolean(props.compact);
  let refreshTimer = null;
  let onStorage = null;
  let onFocus = null;
  let onKeyDown = null;

  const activeMount = createMemo(() => mounts().find((mount) => mount.id === selectedMountId()) || null);
  const selectedLocalRoot = createMemo(() => {
    const selected = selectedPath();
    if (selected !== "/local" && !selected.startsWith("/local/")) return "";
    return localFsRoot();
  });
  const roots = createMemo(() => {
    const mount = activeMount();
    if (!mount) return [];
    return [{
      id: `mount:${mount.id}`,
      path: mount.root,
      name: mount.label,
      kind: "dir",
      mountId: mount.id
    }];
  });
  const fsNodes = createMemo(() => knownDevices().filter((device) => device?.metadata?.capabilities?.fileSystem));
  const selectedFsNode = createMemo(() => fsNodes().find((node) => node.id === getFsNodeTargetId()) || null);
  const localMountUnavailable = createMemo(() => Boolean(activeMount() && activeMount().id === "local" && !selectedFsNode()));

  const refreshMounts = () => {
    setMounts(getFsMounts());
  };

  const loadChildren = async (path) => {
    if (path === "/local" || path.startsWith("/local/")) {
      if (!selectedFsNode()) {
        setChildrenByPath((prev) => ({ ...prev, [path]: [] }));
        return;
      }
    }
    const mount = mounts().find((item) => path === item.root || path.startsWith(`${item.root}/`));
    if (mount && mount.auth !== "ready") {
      setChildrenByPath((prev) => ({ ...prev, [path]: [] }));
      return;
    }
    const entries = await listFsDir(path);
    setChildrenByPath((prev) => ({ ...prev, [path]: entries }));
  };

  const refreshDirectory = async (path) => {
    const dir = normalizePath(path);
    await loadChildren(dir);
  };

  const ensureDirExpanded = async (path) => {
    const normalized = normalizePath(path);
    const nextSet = new Set(expandedDirs());
    nextSet.add(normalized);
    setExpandedDirs(nextSet);
    if (!childrenByPath()[normalized]) {
      await loadChildren(normalized);
    }
  };

  const selectMount = async (mountId) => {
    const mount = mounts().find((item) => item.id === mountId);
    if (!mount) return;
    setSelectedMountId(mount.id);
    setSelectedPath(mount.root);
    setPropertiesNode(null);
    if (mount.auth === "ready") {
      await ensureDirExpanded(mount.root);
    }
    publishEvent("intent.ui.file-manager.mount.selected", { mountId: mount.id }, { source: "intent-ui.file-manager" });
  };

  const selectFsNode = async (nodeId) => {
    const next = String(nodeId || "").trim();
    setFsNodeTarget(next);
    setChildrenByPath({});
    setExpandedDirs(new Set());
    setPropertiesNode(null);
    setLocalFsRoot("");
    setActionStatus(next ? `Selected node ${next}` : "Select a node to access local filesystem.");
    publishEvent("intent.ui.file-manager.node.selected", { nodeId: next }, { source: "intent-ui.file-manager" });
    if (activeMount()?.id === "local" && selectedMountId()) {
      await selectMount(selectedMountId());
    }
  };

  const resolveTargetDir = (node) => node.kind === "dir" ? node.path : dirname(node.path);

  const canPasteTo = (node) => {
    const clip = clipboard();
    if (!clip) return false;
    if (!node?.mountId || clip.mountId !== node.mountId) return false;
    const targetDir = resolveTargetDir(node);
    if (clip.mode === "cut" && (targetDir === clip.path || targetDir.startsWith(`${clip.path}/`))) return false;
    return true;
  };

  const openIntegrationForMount = () => {
    const mount = activeMount();
    if (!mount) return;
    if (mount.id === "github") openWorkflowIntegrations("github");
    if (mount.id === "drive") openWorkflowIntegrations("google");
  };

  const activateNode = async (node) => {
    if (!node) return;
    setSelectedPath(node.path);
    setPropertiesNode(node);
    if (node.kind === "dir") {
      await ensureDirExpanded(node.path);
      return;
    }
    await openFsFile(node.path);
    openWindow("editor");
  };

  const createFolder = async (node) => {
    const targetDir = resolveTargetDir(node);
    const name = window.prompt("Folder name", "new-folder");
    if (!name) return;
    await mkdirFsPath(withName(targetDir, name));
    setActionStatus(`Created folder ${name}`);
    await refreshDirectory(targetDir);
  };

  const renameNode = async (node) => {
    const currentName = basename(node.path);
    const nextName = window.prompt("Rename", currentName);
    if (!nextName || nextName === currentName) return;
    const targetPath = withName(dirname(node.path), nextName);
    await moveFsPath(node.path, targetPath);
    setActionStatus(`Renamed to ${nextName}`);
    setSelectedPath(targetPath);
    await refreshDirectory(dirname(node.path));
  };

  const deleteNode = async (node) => {
    if (!window.confirm(`Delete ${node.name}?`)) return;
    await deleteFsPath(node.path);
    setActionStatus(`Deleted ${node.name}`);
    await refreshDirectory(dirname(node.path));
  };

  const pasteToNode = async (node) => {
    const clip = clipboard();
    if (!clip) return;
    const targetDir = resolveTargetDir(node);
    const destination = withName(targetDir, clip.name);
    if (clip.mode === "cut") {
      await moveFsPath(clip.path, destination);
      setClipboard(null);
      setActionStatus(`Moved ${clip.name}`);
    } else {
      await copyFsPath(clip.path, destination);
      setActionStatus(`Copied ${clip.name}`);
    }
    await refreshDirectory(targetDir);
    await refreshDirectory(dirname(clip.path));
  };

  const canArchive = (node) => node?.mountId === "local";
  const canExtract = (node) => node?.mountId === "local" && node.kind === "file" && archivePattern.test(String(node.name || ""));

  const archiveNode = async (node) => {
    if (!canArchive(node)) {
      setActionStatus("Compress is only available for Local filesystem.");
      return;
    }
    const providerPath = toLocalProviderPath(node.path);
    const response = await fetch(localBridgeHttpUrl("/v1/local/fs/archive"), {
      method: "POST",
      headers: { "content-type": "application/json; charset=utf-8" },
      body: JSON.stringify({ path: providerPath, format: "tar.gz", node_id: getFsNodeTargetId() })
    });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || payload?.ok === false) {
      throw new Error(payload?.error || "Failed to create archive.");
    }
    setActionStatus(`Compressed ${node.name}`);
    await refreshDirectory(dirname(node.path));
  };

  const extractNode = async (node) => {
    if (!canExtract(node)) {
      setActionStatus("Extract is only available for local archive files.");
      return;
    }
    const providerPath = toLocalProviderPath(node.path);
    const response = await fetch(localBridgeHttpUrl("/v1/local/fs/extract"), {
      method: "POST",
      headers: { "content-type": "application/json; charset=utf-8" },
      body: JSON.stringify({ path: providerPath, node_id: getFsNodeTargetId() })
    });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || payload?.ok === false) {
      throw new Error(payload?.error || "Failed to extract archive.");
    }
    setActionStatus(`Extracted ${node.name}`);
    await refreshDirectory(dirname(node.path));
  };

  const copyPathToClipboard = async (node) => {
    const text = String(node?.path || "");
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      setActionStatus("Path copied to clipboard.");
    } catch {
      setActionStatus("Clipboard write failed.");
    }
  };

  const onAction = async (callback) => {
    try {
      await callback();
      publishEvent("intent.ui.file-manager.fs-operation.succeeded", { selectedPath: selectedPath() }, { source: "intent-ui.file-manager" });
    } catch (error) {
      const message = error instanceof Error ? error.message : "File operation failed.";
      setActionStatus(message);
      publishEvent("intent.ui.file-manager.fs-operation.failed", { error: message, selectedPath: selectedPath() }, { source: "intent-ui.file-manager" });
    }
  };

  const closeContextMenu = () => {
    setContextMenu((prev) => ({ ...prev, open: false, node: null }));
  };

  const openContextMenu = (node, event) => {
    if (!node) return;
    event.preventDefault();
    event.stopPropagation();
    setSelectedPath(node.path);
    setPropertiesNode(node);
    const menuWidth = 220;
    const menuHeight = 380;
    const clientWidth = typeof window !== "undefined" ? window.innerWidth : 1280;
    const clientHeight = typeof window !== "undefined" ? window.innerHeight : 720;
    const x = Math.max(8, Math.min(event.clientX, clientWidth - menuWidth - 8));
    const y = Math.max(8, Math.min(event.clientY, clientHeight - menuHeight - 8));
    setContextMenu({ open: true, x, y, node });
  };

  const menuNode = createMemo(() => contextMenu().node || null);
  const menuDisabled = createMemo(() => ({
    paste: !menuNode() || !canPasteTo(menuNode()),
    extract: !menuNode() || !canExtract(menuNode()),
    archive: !menuNode() || !canArchive(menuNode()),
    editor: !menuNode() || menuNode().kind === "dir"
  }));

  onMount(() => {
    const initialTarget = getFsNodeTargetId();
    if (!initialTarget) {
      const firstFsNode = fsNodes()[0];
      if (firstFsNode?.id) setFsNodeTarget(firstFsNode.id);
    }
    fetch(localBridgeHttpUrl(`/v1/local/fs/meta?node_id=${encodeURIComponent(getFsNodeTargetId())}`), { cache: "no-store" })
      .then((response) => response.json().catch(() => ({})))
      .then((payload) => {
        if (payload?.ok && typeof payload.localFsRoot === "string") setLocalFsRoot(payload.localFsRoot);
      })
      .catch(() => setLocalFsRoot(""));
    refreshMounts();
    const firstReady = getFsMounts().find((mount) => mount.auth === "ready");
    if (firstReady) {
      void selectMount(firstReady.id);
    }
    refreshTimer = window.setInterval(refreshMounts, 3000);
    onStorage = () => refreshMounts();
    onFocus = () => refreshMounts();
    window.addEventListener("storage", onStorage);
    window.addEventListener("focus", onFocus);
    window.addEventListener("click", closeContextMenu);
    window.addEventListener("contextmenu", closeContextMenu);
    onKeyDown = (event) => {
      if (event.key === "Escape") closeContextMenu();
    };
    window.addEventListener("keydown", onKeyDown);
  });
  onCleanup(() => {
    if (refreshTimer) window.clearInterval(refreshTimer);
    if (onStorage) window.removeEventListener("storage", onStorage);
    if (onFocus) window.removeEventListener("focus", onFocus);
    if (typeof window !== "undefined") {
      window.removeEventListener("click", closeContextMenu);
      window.removeEventListener("contextmenu", closeContextMenu);
      if (onKeyDown) window.removeEventListener("keydown", onKeyDown);
    }
  });

  createEffect(() => {
    const devices = fsNodes();
    if (!devices.length) {
      if (getFsNodeTargetId()) {
        setFsNodeTarget("");
        setLocalFsRoot("");
      }
      return;
    }
    const current = getFsNodeTargetId();
    const exists = devices.some((device) => device.id === current);
    if (!exists) {
      setFsNodeTarget(devices[0].id);
      setLocalFsRoot("");
    }
  });

  createEffect(() => {
    const selectedNodeId = getFsNodeTargetId();
    if (!selectedNodeId) {
      setLocalFsRoot("");
      return;
    }
    fetch(localBridgeHttpUrl(`/v1/local/fs/meta?node_id=${encodeURIComponent(selectedNodeId)}`), { cache: "no-store" })
      .then((response) => response.json().catch(() => ({})))
      .then((payload) => {
        if (payload?.ok && typeof payload.localFsRoot === "string") {
          setLocalFsRoot(payload.localFsRoot);
        } else {
          setLocalFsRoot("");
        }
      })
      .catch(() => setLocalFsRoot(""));
  });

  return (
    <div class={`flex h-full min-h-0 flex-col text-sm text-neutral-300 ${compact() ? "" : "bg-[#111] p-4"}`}>
      <div class="shrink-0 border-b border-neutral-800 px-3 py-2">
        <h3 class="text-xs font-medium uppercase tracking-wide text-neutral-300">File Manager</h3>
        <div class="mt-2">
          <label class="mb-1 block text-[10px] uppercase tracking-wide text-neutral-500" for="file-manager-node-selector">Node</label>
          <select
            id="file-manager-node-selector"
            data-testid="file-manager-node-selector"
            class="h-8 w-full rounded-md border border-neutral-800 bg-neutral-900/70 px-2 text-xs text-neutral-200 outline-none focus:border-[hsl(var(--primary)/0.45)]"
            value={getFsNodeTargetId()}
            onChange={(event) => { void selectFsNode(event.currentTarget.value); }}
          >
            <Show when={fsNodes().length > 0} fallback={<option value="">No filesystem nodes available</option>}>
              <For each={fsNodes()}>
                {(node) => (
                  <option value={node.id}>
                    {node.name || node.id}
                  </option>
                )}
              </For>
            </Show>
          </select>
        </div>
      </div>

      <div class="grid grid-cols-4 gap-1.5 px-3 py-2">
        <For each={mounts()}>
          {(mount) => {
            const meta = mountMeta[mount.id] || mountMeta.local;
            const Icon = meta.icon;
            const active = () => selectedMountId() === mount.id;
            return (
              <button
                type="button"
                onClick={() => void selectMount(mount.id)}
                class={`h-8 rounded-md border px-2 text-xs transition-colors ${
                  active() ? "border-[hsl(var(--primary)/0.45)] bg-[hsl(var(--primary)/0.15)] text-[hsl(var(--primary))]" : "border-neutral-800 bg-neutral-900/65 text-neutral-300 hover:bg-neutral-800/80"
                }`}
                title={mount.label}
              >
                <div class="flex items-center justify-center gap-1.5">
                  <Icon size={14} class={meta.tone} />
                  <span class={`inline-block h-2 w-2 rounded-full ${mount.auth === "ready" ? "bg-[hsl(var(--primary))]" : "bg-neutral-600"}`} />
                </div>
              </button>
            );
          }}
        </For>
      </div>

      <Show when={activeMount() && activeMount().auth !== "ready"}>
        <div class="mx-3 mb-2 rounded-md border border-neutral-700 bg-neutral-900/55 px-2.5 py-2 text-xs text-neutral-300">
          <span>{activeMount().label} is not linked.</span>
          <button
            type="button"
            class="ml-2 inline-flex h-7 items-center rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
            onClick={openIntegrationForMount}
          >
            Link integration
          </button>
        </div>
      </Show>
      <Show when={localMountUnavailable()}>
        <div class="mx-3 mb-2 rounded-md border border-rose-500/40 bg-rose-950/20 px-2.5 py-2 text-xs text-rose-200" data-testid="file-manager-node-unavailable">
          Local filesystem unavailable for selected node.
        </div>
      </Show>

      <div class="min-h-0 flex-1 overflow-auto px-3">
        <Show when={roots().length > 0} fallback={<p class="px-2 py-3 text-xs text-neutral-500">Select a filesystem.</p>}>
          <div class="space-y-1">
            <For each={roots()}>
              {(node) => (
                <FileRow
                  node={node}
                  selectedPath={selectedPath}
                  setSelectedPath={setSelectedPath}
                  expandedDirs={expandedDirs}
                  childrenByPath={childrenByPath}
                  onActivate={(entry) => onAction(() => activateNode(entry))}
                  onCreateFolder={(entry) => onAction(() => createFolder(entry))}
                  onRename={(entry) => onAction(() => renameNode(entry))}
                  onDelete={(entry) => onAction(() => deleteNode(entry))}
                  onCut={(entry) => {
                    setClipboard({ mode: "cut", path: entry.path, name: entry.name, mountId: entry.mountId, kind: entry.kind });
                    setActionStatus(`Cut ${entry.name}`);
                  }}
                  onCopy={(entry) => {
                    setClipboard({ mode: "copy", path: entry.path, name: entry.name, mountId: entry.mountId, kind: entry.kind });
                    setActionStatus(`Copied ${entry.name}`);
                  }}
                  onPaste={(entry) => onAction(() => pasteToNode(entry))}
                  canPasteTo={canPasteTo}
                  canExtract={canExtract}
                  onExtract={(entry) => onAction(() => extractNode(entry))}
                  canArchive={canArchive}
                  onArchive={(entry) => onAction(() => archiveNode(entry))}
                  onSendToEditor={(entry) => onAction(async () => {
                    if (entry.kind === "dir") return;
                    await openFsFile(entry.path);
                    openWindow("editor");
                    setActionStatus(`Opened ${entry.name} in editor.`);
                  })}
                  onSendToTerminal={(entry) => {
                    setSelectedPath(entry.path);
                    openWindow("terminal");
                    setActionStatus("Opened terminal.");
                  }}
                  onCopyPath={(entry) => onAction(() => copyPathToClipboard(entry))}
                  onProperties={(entry) => {
                    setSelectedPath(entry.path);
                    setPropertiesNode(entry);
                  }}
                  onContextRequest={openContextMenu}
                />
              )}
            </For>
          </div>
        </Show>
      </div>

      <div class="mt-2 shrink-0 rounded-md border border-neutral-800 bg-neutral-900/55 px-2.5 py-2 text-xs text-neutral-400 mx-3 mb-3">
        <p class="truncate">Selected: {selectedPath()}</p>
        <Show when={selectedLocalRoot()}>
          <p class="mt-1 truncate text-[10px] text-neutral-500">/local root: {selectedLocalRoot()}</p>
        </Show>
        <Show when={clipboard()}>
          <p class="mt-1 truncate text-[10px] text-neutral-500">
            Clipboard: {clipboard().mode} {clipboard().name}
          </p>
        </Show>
        <Show when={propertiesNode()}>
          <p class="mt-1 truncate text-[10px] text-neutral-500">
            {propertiesNode().kind} {propertiesNode().name}
            <Show when={propertiesNode().size !== undefined}> · {propertiesNode().size} B</Show>
          </p>
        </Show>
        <Show when={actionStatus()}>
          <p class="mt-1 truncate text-[10px] text-[hsl(var(--primary))]">{actionStatus()}</p>
        </Show>
        <Show when={selectedFsNode()}>
          <p class="mt-1 truncate text-[10px] text-neutral-500">Node: {selectedFsNode().name || selectedFsNode().id}</p>
        </Show>
        <Show when={lastFsError()}>
          <p class="mt-1 truncate text-rose-300">FS: {lastFsError()}</p>
        </Show>
      </div>

      <Show when={contextMenu().open && menuNode()}>
        <div
          class="fixed inset-0 z-[10040]"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) closeContextMenu();
          }}
        >
          <div
            class="absolute w-[220px] rounded-md border border-neutral-700 bg-[#10131a] p-1.5 shadow-2xl"
            style={{ left: `${contextMenu().x}px`, top: `${contextMenu().y}px` }}
            onMouseDown={(event) => event.stopPropagation()}
          >
            <button type="button" class="menu-item" onClick={() => { closeContextMenu(); void onAction(() => activateNode(menuNode())); }}>
              <TbOutlineFileText size={13} /> Open
            </button>
            <Show when={menuNode().kind === "dir"}>
              <button type="button" class="menu-item" onClick={() => { closeContextMenu(); void onAction(() => createFolder(menuNode())); }}>
                <TbOutlineFolderPlus size={13} /> New folder
              </button>
            </Show>
            <button type="button" class="menu-item" onClick={() => { closeContextMenu(); void onAction(() => renameNode(menuNode())); }}>
              <TbOutlinePencil size={13} /> Rename
            </button>
            <button type="button" class="menu-item" onClick={() => { closeContextMenu(); setClipboard({ mode: "cut", path: menuNode().path, name: menuNode().name, mountId: menuNode().mountId, kind: menuNode().kind }); setActionStatus(`Cut ${menuNode().name}`); }}>
              <TbOutlineCut size={13} /> Cut
            </button>
            <button type="button" class="menu-item" onClick={() => { closeContextMenu(); setClipboard({ mode: "copy", path: menuNode().path, name: menuNode().name, mountId: menuNode().mountId, kind: menuNode().kind }); setActionStatus(`Copied ${menuNode().name}`); }}>
              <TbOutlineCopy size={13} /> Copy
            </button>
            <button type="button" class="menu-item" disabled={menuDisabled().paste} onClick={() => { closeContextMenu(); void onAction(() => pasteToNode(menuNode())); }}>
              <TbOutlineClipboard size={13} /> Paste
            </button>
            <button type="button" class="menu-item" disabled={menuDisabled().extract} onClick={() => { closeContextMenu(); void onAction(() => extractNode(menuNode())); }}>
              <TbOutlineArchive size={13} /> Extract
            </button>
            <button type="button" class="menu-item" disabled={menuDisabled().archive} onClick={() => { closeContextMenu(); void onAction(() => archiveNode(menuNode())); }}>
              <TbOutlineArchive size={13} /> Compress
            </button>
            <div class="my-1 h-px bg-neutral-700" />
            <p class="px-2 py-1 text-[10px] uppercase tracking-wide text-neutral-500">Send to</p>
            <button type="button" class="menu-item" disabled={menuDisabled().editor} onClick={() => { closeContextMenu(); void onAction(async () => { await openFsFile(menuNode().path); openWindow("editor"); setActionStatus(`Opened ${menuNode().name} in editor.`); }); }}>
              <TbOutlineSend size={13} /> Editor
            </button>
            <button type="button" class="menu-item" onClick={() => { closeContextMenu(); setSelectedPath(menuNode().path); openWindow("terminal"); setActionStatus("Opened terminal."); }}>
              <TbOutlineSend size={13} /> Terminal
            </button>
            <button type="button" class="menu-item" onClick={() => { closeContextMenu(); void onAction(() => copyPathToClipboard(menuNode())); }}>
              <TbOutlineSend size={13} /> Clipboard path
            </button>
            <button type="button" class="menu-item" onClick={() => { closeContextMenu(); setSelectedPath(menuNode().path); setPropertiesNode(menuNode()); }}>
              <TbOutlineInfoCircle size={13} /> Properties
            </button>
            <div class="my-1 h-px bg-neutral-700" />
            <button type="button" class="menu-item text-rose-300 hover:bg-rose-600/15" onClick={() => { closeContextMenu(); void onAction(() => deleteNode(menuNode())); }}>
              <TbOutlineTrash size={13} /> Delete
            </button>
          </div>
        </div>
      </Show>
      <style>{`
        .menu-item {
          width: 100%;
          display: flex;
          align-items: center;
          gap: 8px;
          border-radius: 6px;
          padding: 6px 8px;
          font-size: 12px;
          color: rgb(216 220 227);
        }
        .menu-item:hover { background: rgba(64, 71, 88, 0.45); }
        .menu-item:disabled {
          opacity: 0.45;
          pointer-events: none;
        }
      `}</style>
    </div>
  );
}

export { FileManager as default };
