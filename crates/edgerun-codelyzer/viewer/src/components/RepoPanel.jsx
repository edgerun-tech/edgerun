/** @jsxImportSource solid-js **/
/** @jsxImportSource solid-js **/
/**
 * Repository panel — discover, add, switch, remove repos.
 */
import { createSignal, createEffect, For, Show } from "solid-js";
import { repos, setRepos, activeRepoPath, setActiveRepoPath, repoPanelOpen, setRepoPanelOpenLocal, setLoading, setLoadingMessage } from "../store.js";
import { X, Search, Plus, FolderSearch } from "lucide-solid";
import { getRepos } from "../ws.js";
const [filter, setFilter] = createSignal("");
const [showAddModal, setShowAddModal] = createSignal(false);
const [addPath, setAddPath] = createSignal("");
export function RepoPanel() {
  async function loadRepos() {
    try {
      const data = await getRepos();
      setRepos(data.repos);
      setActiveRepoPath(data.active);
    } catch (e) {
      console.error("[repos] Failed to load:", e);
    }
  }

  // Load on mount
  createEffect(() => {
    if (repoPanelOpen()) {
      void loadRepos();
    }
  });
  async function discoverRepos() {
    setLoading(true);
    setLoadingMessage("Discovering repositories…");
    try {
      await wsDiscoverRepos();
      await loadRepos();
    } catch (e) {
      console.error("[repos] Discover failed:", e);
    } finally {
      setLoading(false);
    }
  }
  async function addRepo() {
    const path = addPath().trim();
    if (!path) return;
    try {
      const result = await wsAddRepo(path);
      if (result.error) {
        alert(`Failed to add: ${result.error}`);
        return;
      }
      setShowAddModal(false);
      setAddPath("");
      await loadRepos();
    } catch (e) {
      console.error("[repos] Add failed:", e);
    }
  }
  async function switchRepo(path) {
    if (path === activeRepoPath()) return;
    setLoading(true);
    setLoadingMessage("Switching repository…");
    try {
      await wsSwitchRepo(path);
      setActiveRepoPath(path);
      await loadRepos();
    } catch (e) {
      console.error("[repos] Switch error:", e);
    } finally {
      setLoading(false);
    }
  }
  async function removeRepo(path) {
    if (!confirm(`Remove "${path.split("/").pop()}" from the registry?`)) return;
    try {
      await wsRemoveRepo(path);
      setRepos(prev => prev.filter(r => r.path !== path));
      if (activeRepoPath() === path) setActiveRepoPath(null);
    } catch (e) {
      console.error("[repos] Remove failed:", e);
    }
  }
  const filteredRepos = () => {
    const f = filter().toLowerCase();
    if (!f) return repos();
    return repos().filter(r => r.name.toLowerCase().includes(f) || r.path.toLowerCase().includes(f) || (r.languages ?? []).some(l => l.toLowerCase().includes(f)));
  };
  return <div class="fixed top-4 right-4 w-96 max-h-[80vh] bg-bg-secondary border border-border-default rounded-lg shadow-xl z-50 flex flex-col">
      {/* Header */}
      <div class="flex items-center justify-between px-4 py-3 border-b border-border-default">
        <h4 class="text-sm font-semibold text-text-heading">📂 Repositories</h4>
        <button class="btn px-1.5 py-0.5 text-sm" onClick={() => setRepoPanelOpenLocal(false)}>
          <X class="w-4 h-4" />
        </button>
      </div>

      {/* Current repo */}
      <div class="px-4 py-2 border-b border-border-default text-xs">
        {activeRepoPath() ? <span>
            <span class="text-accent-green">Active:</span>{" "}
            <strong>{activeRepoPath()?.split("/").pop()}</strong>
          </span> : <span class="text-text-muted">No repository selected</span>}
      </div>

      {/* Actions */}
      <div class="flex gap-2 px-4 py-2 border-b border-border-default">
        <button class="btn text-xs flex-1" onClick={discoverRepos}>
          <FolderSearch class="w-3.5 h-3.5 inline mr-1" />
          Discover
        </button>
        <button class="btn text-xs flex-1" onClick={() => setShowAddModal(true)}>
          <Plus class="w-3.5 h-3.5 inline mr-1" />
          Add
        </button>
      </div>

      {/* Filter */}
      <div class="px-4 py-2 border-b border-border-default">
        <div class="relative">
          <Search class="absolute left-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-muted" />
          <input type="text" class="input pl-8 text-xs" placeholder="Filter repositories…" value={filter()} onInput={e => setFilter(e.currentTarget.value)} />
        </div>
      </div>

      {/* List */}
      <div class="flex-1 overflow-y-auto p-2 max-h-80">
        {filteredRepos().length === 0 ? <div class="text-center py-8 text-text-muted text-xs">
            {repos().length === 0 ? 'No repositories found. Click "Discover" or "Add" to get started.' : "No matches."}
          </div> : <div class="space-y-1">
            <For each={filteredRepos()}>
              {repo => <div class="flex items-start gap-2 px-3 py-2 rounded-md text-xs hover:bg-bg-hover cursor-pointer group" classList={{
            "bg-accent-blue-bg": repo.path === activeRepoPath()
          }} onClick={() => switchRepo(repo.path)}>
                  <div class="flex-1 min-w-0">
                    <div class="font-medium text-text-heading truncate">
                      {repo.is_git_repo && <span class="mr-1">⎇</span>}
                      {repo.name}
                    </div>
                    <div class="text-text-muted truncate text-[11px]" title={repo.path}>
                      {repo.path}
                    </div>
                    <div class="flex items-center gap-2 mt-1 text-[11px] text-text-muted">
                      <span>📄 {repo.file_count?.toLocaleString() ?? "—"}</span>
                      {(repo.languages ?? []).slice(0, 3).map(l => <span class="badge bg-bg-elevated text-text-secondary border border-border-default">{l}</span>)}
                    </div>
                  </div>
                  {repo.path !== activeRepoPath() && <button class="opacity-0 group-hover:opacity-100 text-text-muted hover:text-accent-red text-sm leading-none transition-opacity" onClick={e => {
              e.stopPropagation();
              void removeRepo(repo.path);
            }}>
                      ×
                    </button>}
                </div>}
            </For>
          </div>}
      </div>

      {/* Add modal */}
      <Show when={showAddModal()}>
        <div class="fixed inset-0 bg-bg-primary/60 flex items-center justify-center z-[60]" onClick={() => setShowAddModal(false)}>
          <div class="bg-bg-secondary border border-border-default rounded-lg p-6 w-96" onClick={e => e.stopPropagation()}>
            <h4 class="text-sm font-semibold text-text-heading mb-3">Add Repository</h4>
            <input type="text" class="input mb-3" placeholder="/path/to/repository" value={addPath()} onInput={e => setAddPath(e.currentTarget.value)} onKeyDown={e => {
            if (e.key === "Enter") void addRepo();
          }} />
            <div class="flex gap-2 justify-end">
              <button class="btn btn-primary" onClick={addRepo}>
                Add
              </button>
              <button class="btn" onClick={() => setShowAddModal(false)}>
                Cancel
              </button>
            </div>
          </div>
        </div>
      </Show>
    </div>;
}