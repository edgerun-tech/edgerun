import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { setAttribute as _$setAttribute } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { memo as _$memo } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div class="fixed inset-0 bg-bg-primary/60 flex items-center justify-center z-[60]"><div class="bg-bg-secondary border border-border-default rounded-lg p-6 w-96"><h4 class="text-sm font-semibold text-text-heading mb-3">Add Repository</h4><input type=text class="input mb-3"placeholder=/path/to/repository><div class="flex gap-2 justify-end"><button class="btn btn-primary">Add</button><button class=btn>Cancel`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div class="fixed top-4 right-4 w-96 max-h-[80vh] bg-bg-secondary border border-border-default rounded-lg shadow-xl z-50 flex flex-col"><div class="flex items-center justify-between px-4 py-3 border-b border-border-default"><h4 class="text-sm font-semibold text-text-heading">📂 Repositories</h4><button class="btn px-1.5 py-0.5 text-sm"></button></div><div class="px-4 py-2 border-b border-border-default text-xs"></div><div class="flex gap-2 px-4 py-2 border-b border-border-default"><button class="btn text-xs flex-1">Discover</button><button class="btn text-xs flex-1">Add</button></div><div class="px-4 py-2 border-b border-border-default"><div class=relative><input type=text class="input pl-8 text-xs"placeholder="Filter repositories…"></div></div><div class="flex-1 overflow-y-auto p-2 max-h-80">`),
  _tmpl$3 = /*#__PURE__*/_$template(`<span><span class=text-accent-green>Active:</span> <strong>`),
  _tmpl$4 = /*#__PURE__*/_$template(`<span class=text-text-muted>No repository selected`),
  _tmpl$5 = /*#__PURE__*/_$template(`<div class="text-center py-8 text-text-muted text-xs">`),
  _tmpl$6 = /*#__PURE__*/_$template(`<div class=space-y-1>`),
  _tmpl$7 = /*#__PURE__*/_$template(`<div class="flex items-start gap-2 px-3 py-2 rounded-md text-xs hover:bg-bg-hover cursor-pointer group"><div class="flex-1 min-w-0"><div class="font-medium text-text-heading truncate"></div><div class="text-text-muted truncate text-[11px]"></div><div class="flex items-center gap-2 mt-1 text-[11px] text-text-muted"><span>📄 `),
  _tmpl$8 = /*#__PURE__*/_$template(`<span class=mr-1>⎇`),
  _tmpl$9 = /*#__PURE__*/_$template(`<span class="badge bg-bg-elevated text-text-secondary border border-border-default">`),
  _tmpl$0 = /*#__PURE__*/_$template(`<button class="opacity-0 group-hover:opacity-100 text-text-muted hover:text-accent-red text-sm leading-none transition-opacity">×`);
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
  return (() => {
    var _el$ = _tmpl$2(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.firstChild,
      _el$4 = _el$3.nextSibling,
      _el$5 = _el$2.nextSibling,
      _el$6 = _el$5.nextSibling,
      _el$7 = _el$6.firstChild,
      _el$8 = _el$7.firstChild,
      _el$9 = _el$7.nextSibling,
      _el$0 = _el$9.firstChild,
      _el$1 = _el$6.nextSibling,
      _el$10 = _el$1.firstChild,
      _el$11 = _el$10.firstChild,
      _el$12 = _el$1.nextSibling;
    _el$4.$$click = () => setRepoPanelOpenLocal(false);
    _$insert(_el$4, _$createComponent(X, {
      "class": "w-4 h-4"
    }));
    _$insert(_el$5, (() => {
      var _c$ = _$memo(() => !!activeRepoPath());
      return () => _c$() ? (() => {
        var _el$20 = _tmpl$3(),
          _el$21 = _el$20.firstChild,
          _el$22 = _el$21.nextSibling,
          _el$23 = _el$22.nextSibling;
        _$insert(_el$23, () => activeRepoPath()?.split("/").pop());
        return _el$20;
      })() : _tmpl$4();
    })());
    _el$7.$$click = discoverRepos;
    _$insert(_el$7, _$createComponent(FolderSearch, {
      "class": "w-3.5 h-3.5 inline mr-1"
    }), _el$8);
    _el$9.$$click = () => setShowAddModal(true);
    _$insert(_el$9, _$createComponent(Plus, {
      "class": "w-3.5 h-3.5 inline mr-1"
    }), _el$0);
    _$insert(_el$10, _$createComponent(Search, {
      "class": "absolute left-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-muted"
    }), _el$11);
    _el$11.$$input = e => setFilter(e.currentTarget.value);
    _$insert(_el$12, (() => {
      var _c$2 = _$memo(() => filteredRepos().length === 0);
      return () => _c$2() ? (() => {
        var _el$25 = _tmpl$5();
        _$insert(_el$25, () => repos().length === 0 ? 'No repositories found. Click "Discover" or "Add" to get started.' : "No matches.");
        return _el$25;
      })() : (() => {
        var _el$26 = _tmpl$6();
        _$insert(_el$26, _$createComponent(For, {
          get each() {
            return filteredRepos();
          },
          children: repo => (() => {
            var _el$27 = _tmpl$7(),
              _el$28 = _el$27.firstChild,
              _el$29 = _el$28.firstChild,
              _el$30 = _el$29.nextSibling,
              _el$31 = _el$30.nextSibling,
              _el$32 = _el$31.firstChild,
              _el$33 = _el$32.firstChild;
            _el$27.$$click = () => switchRepo(repo.path);
            _$insert(_el$29, (() => {
              var _c$3 = _$memo(() => !!repo.is_git_repo);
              return () => _c$3() && _tmpl$8();
            })(), null);
            _$insert(_el$29, () => repo.name, null);
            _$insert(_el$30, () => repo.path);
            _$insert(_el$32, () => repo.file_count?.toLocaleString() ?? "—", null);
            _$insert(_el$31, () => (repo.languages ?? []).slice(0, 3).map(l => (() => {
              var _el$35 = _tmpl$9();
              _$insert(_el$35, l);
              return _el$35;
            })()), null);
            _$insert(_el$27, (() => {
              var _c$4 = _$memo(() => repo.path !== activeRepoPath());
              return () => _c$4() && (() => {
                var _el$36 = _tmpl$0();
                _el$36.$$click = e => {
                  e.stopPropagation();
                  void removeRepo(repo.path);
                };
                return _el$36;
              })();
            })(), null);
            _$effect(_p$ => {
              var _v$ = !!(repo.path === activeRepoPath()),
                _v$2 = repo.path;
              _v$ !== _p$.e && _el$27.classList.toggle("bg-accent-blue-bg", _p$.e = _v$);
              _v$2 !== _p$.t && _$setAttribute(_el$30, "title", _p$.t = _v$2);
              return _p$;
            }, {
              e: undefined,
              t: undefined
            });
            return _el$27;
          })()
        }));
        return _el$26;
      })();
    })());
    _$insert(_el$, _$createComponent(Show, {
      get when() {
        return showAddModal();
      },
      get children() {
        var _el$13 = _tmpl$(),
          _el$14 = _el$13.firstChild,
          _el$15 = _el$14.firstChild,
          _el$16 = _el$15.nextSibling,
          _el$17 = _el$16.nextSibling,
          _el$18 = _el$17.firstChild,
          _el$19 = _el$18.nextSibling;
        _el$13.$$click = () => setShowAddModal(false);
        _el$14.$$click = e => e.stopPropagation();
        _el$16.$$keydown = e => {
          if (e.key === "Enter") void addRepo();
        };
        _el$16.$$input = e => setAddPath(e.currentTarget.value);
        _el$18.$$click = addRepo;
        _el$19.$$click = () => setShowAddModal(false);
        _$effect(() => _el$16.value = addPath());
        return _el$13;
      }
    }), null);
    _$effect(() => _el$11.value = filter());
    return _el$;
  })();
}
_$delegateEvents(["click", "input", "keydown"]);