import { createStore, cloneState } from '../state/store.js';
import { createIndexedDbAdapter } from '../state/indexedDbAdapter.js';

const TERM_MIN_RATIO = 0.20;
const TERM_MAX_RATIO = 0.85;
const TERM_DEFAULT_RATIO = 0.35;

function termId() {
  if (window.crypto?.randomUUID) return window.crypto.randomUUID();
  return `id-${Date.now()}-${Math.floor(Math.random() * 1e6)}`;
}

function defaultTermTab(index, baseUrl) {
  return {
    id: termId(),
    title: `Terminal ${index + 1}`,
    split: 'none',
    panes: [{ id: termId(), baseUrl: baseUrl || '/term/' }],
  };
}

function normalizeState(raw) {
  const base = raw || {};
  const tabs = Array.isArray(base.tabs) && base.tabs.length > 0
    ? base.tabs.map((tab, i) => ({
        id: tab.id || termId(),
        title: tab.title || `Terminal ${i + 1}`,
        split: tab.split === 'split-cols' || tab.split === 'split-rows' ? tab.split : 'none',
        panes: Array.isArray(tab.panes) && tab.panes.length > 0
          ? tab.panes.slice(0, 2).map((p) => ({ id: p.id || termId(), baseUrl: String(p.baseUrl || base.baseUrl || '/term/') }))
          : [{ id: termId(), baseUrl: String(base.baseUrl || '/term/') }],
      }))
    : [defaultTermTab(0, String(base.baseUrl || '/term/'))];

  const activeTabId = tabs.some((t) => t.id === base.activeTabId) ? base.activeTabId : tabs[0].id;

  return {
    open: !!base.open,
    heightRatio: Math.min(TERM_MAX_RATIO, Math.max(TERM_MIN_RATIO, Number(base.heightRatio) || TERM_DEFAULT_RATIO)),
    baseUrl: String(base.baseUrl || '/term/'),
    tabs,
    activeTabId,
  };
}

function buildTermSrc(baseUrl, paneId) {
  let url;
  try {
    url = new URL(baseUrl || '/term/', window.location.origin);
  } catch {
    url = new URL('/term/', window.location.origin);
  }
  url.searchParams.set('embed', '1');
  url.searchParams.set('sid', paneId);
  return url.toString();
}

export function createTerminalDrawer() {
  const stateAdapter = createIndexedDbAdapter({
    dbName: 'edgerun-control-ui',
    storeName: 'ui_state',
    key: 'edgerun.control.termDrawer.v1',
  });

  const store = createStore(normalizeState(null));
  const runtime = new Map();
  let persistTimer = null;

  function queuePersist() {
    if (persistTimer) clearTimeout(persistTimer);
    persistTimer = setTimeout(() => {
      stateAdapter.set(store.get());
      persistTimer = null;
    }, 100);
  }

  function update(updater) {
    store.set((prev) => {
      const draft = cloneState(prev);
      const next = updater(draft) || draft;
      return normalizeState(next);
    });
    queuePersist();
  }

  function activeTab(state) {
    return state.tabs.find((t) => t.id === state.activeTabId) || state.tabs[0];
  }

  function ensureRuntime(tab, state) {
    const paneHost = document.getElementById('termPaneHost');
    let tabRuntime = runtime.get(tab.id);
    if (!tabRuntime) {
      const root = document.createElement('div');
      root.className = 'term-tab-root';
      tabRuntime = { root, panes: new Map() };
      runtime.set(tab.id, tabRuntime);
      paneHost.appendChild(root);
    }

    tabRuntime.root.className = `term-tab-root ${tab.split}`;
    const keep = new Set(tab.panes.map((p) => p.id));

    for (const [paneId, frame] of tabRuntime.panes.entries()) {
      if (!keep.has(paneId)) {
        frame.remove();
        tabRuntime.panes.delete(paneId);
      }
    }

    for (const pane of tab.panes) {
      let frame = tabRuntime.panes.get(pane.id);
      if (!frame) {
        frame = document.createElement('iframe');
        frame.className = 'term-pane-frame';
        frame.setAttribute('loading', 'eager');
        frame.setAttribute('allow', 'clipboard-read; clipboard-write');
        tabRuntime.panes.set(pane.id, frame);
        tabRuntime.root.appendChild(frame);
      }
      const nextSrc = buildTermSrc(pane.baseUrl || state.baseUrl, pane.id);
      if (frame.src !== nextSrc) frame.src = nextSrc;
    }
  }

  function applyGeometry(state) {
    const drawer = document.getElementById('termDrawer');
    drawer.style.height = `${Math.round(window.innerHeight * state.heightRatio)}px`;
    drawer.classList.toggle('closed', !state.open);
  }

  function render() {
    const state = store.get();
    const tabsEl = document.getElementById('termTabs');
    const baseInput = document.getElementById('termBaseUrl');
    const toggleBtn = document.getElementById('toggleTerminalBtn');
    const paneHost = document.getElementById('termPaneHost');

    tabsEl.innerHTML = '';

    const liveTabIds = new Set(state.tabs.map((t) => t.id));
    for (const [tabId, tabRuntime] of runtime.entries()) {
      if (!liveTabIds.has(tabId)) {
        tabRuntime.root.remove();
        runtime.delete(tabId);
      }
    }

    for (const tab of state.tabs) ensureRuntime(tab, state);

    for (const tab of state.tabs) {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = `term-tab${tab.id === state.activeTabId ? ' active' : ''}`;
      btn.textContent = tab.title;
      btn.onclick = () => update((draft) => {
        draft.activeTabId = tab.id;
        return draft;
      });
      tabsEl.appendChild(btn);

      const tabRuntime = runtime.get(tab.id);
      if (tabRuntime) {
        tabRuntime.root.classList.toggle('active', tab.id === state.activeTabId);
        paneHost.appendChild(tabRuntime.root);
      }
    }

    baseInput.value = state.baseUrl;
    toggleBtn.textContent = state.open ? 'Hide Terminal' : 'Terminal';
    applyGeometry(state);
  }

  function setSplit(mode) {
    update((state) => {
      const tab = activeTab(state);
      if (!tab) return state;
      if (mode === 'none') {
        tab.split = 'none';
        tab.panes = [tab.panes[0] || { id: termId(), baseUrl: state.baseUrl }];
      } else {
        tab.split = mode;
        if (tab.panes.length < 2) tab.panes.push({ id: termId(), baseUrl: tab.panes[0]?.baseUrl || state.baseUrl });
        tab.panes = tab.panes.slice(0, 2);
      }
      return state;
    });
  }

  function mount() {
    store.subscribe(render);

    document.getElementById('toggleTerminalBtn').onclick = () => {
      update((state) => {
        state.open = !state.open;
        return state;
      });
    };

    document.getElementById('termNewTab').onclick = () => {
      update((state) => {
        const tab = defaultTermTab(state.tabs.length, state.baseUrl);
        state.tabs.push(tab);
        state.activeTabId = tab.id;
        state.open = true;
        return state;
      });
    };

    document.getElementById('termCloseTab').onclick = () => {
      const state = store.get();
      if (state.tabs.length <= 1) return;
      update((draft) => {
        const idx = draft.tabs.findIndex((t) => t.id === draft.activeTabId);
        if (idx < 0) return draft;
        const [removed] = draft.tabs.splice(idx, 1);
        const tabRuntime = runtime.get(removed.id);
        if (tabRuntime) {
          tabRuntime.root.remove();
          runtime.delete(removed.id);
        }
        draft.activeTabId = draft.tabs[Math.max(0, idx - 1)].id;
        return draft;
      });
    };

    document.getElementById('termSplitCols').onclick = () => setSplit('split-cols');
    document.getElementById('termSplitRows').onclick = () => setSplit('split-rows');
    document.getElementById('termSingle').onclick = () => setSplit('none');

    document.getElementById('termBaseUrl').onchange = (ev) => {
      const val = String(ev.target.value || '').trim();
      if (!val) return;
      update((state) => {
        state.baseUrl = val;
        const tab = activeTab(state);
        if (tab) for (const pane of tab.panes) pane.baseUrl = val;
        return state;
      });
    };

    const resizer = document.getElementById('termResizer');
    let dragging = false;

    const onMove = (clientY) => {
      const newHeight = window.innerHeight - clientY;
      const minPx = Math.round(window.innerHeight * TERM_MIN_RATIO);
      const maxPx = Math.round(window.innerHeight * TERM_MAX_RATIO);
      const clamped = Math.max(minPx, Math.min(maxPx, newHeight));
      update((state) => {
        state.heightRatio = clamped / window.innerHeight;
        state.open = true;
        return state;
      });
    };

    resizer.addEventListener('pointerdown', (ev) => {
      dragging = true;
      resizer.setPointerCapture(ev.pointerId);
      onMove(ev.clientY);
    });

    resizer.addEventListener('pointermove', (ev) => {
      if (!dragging) return;
      onMove(ev.clientY);
    });

    const endDrag = () => { dragging = false; };
    resizer.addEventListener('pointerup', endDrag);
    resizer.addEventListener('pointercancel', endDrag);
    window.addEventListener('resize', () => applyGeometry(store.get()));

    stateAdapter.get().then((saved) => {
      if (saved) store.set(normalizeState(saved));
      else render();
    });
  }

  return { mount };
}
