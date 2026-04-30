
const DASH_SURFACES = [
  {
    key: 'build-log',
    label: 'Build Log',
    pathPrefix: '/surface/blog',
    slotId: 'dashSurfaceBuildLog',
    fallback: 'Build log is unavailable right now.',
  },
  {
    key: 'code',
    label: 'Code',
    pathPrefix: '/surface/git',
    slotId: 'dashSurfaceCode',
    fallback: 'Code surface is unavailable right now.',
  },
  {
    key: 'mail',
    label: 'Mail',
    pathPrefix: '/surface/mail',
    slotId: 'dashSurfaceMail',
    fallback: 'Mail surface is unavailable right now.',
  },
  {
    key: 'apps',
    label: 'Apps',
    pathPrefix: '/surface/apps',
    slotId: 'dashSurfaceApps',
    fallback: 'Apps surface is unavailable right now.',
  },
];

const DASH_WORKSPACE_STATE_KEY = 'edgerun-dashboard-workspace-v1';
const DASH_CHAT_STATE_KEY = 'edgerun-dashboard-chat-state-v1';
const DASH_CHAT_MIN_DELAY_MS = 2400;
const DASH_CHAT_DUP_WINDOW_MS = 12000;
const DASH_CHAT_REPEAT_WINDOW_LIMIT = 4;
const DASH_CHAT_MESSAGE_MAX = 800;
const DASH_MODULE_MIN_WIDTH = 260;
const DASH_MODULE_MIN_HEIGHT = 220;
const DASH_WORKSPACE_PADDING = 12;
const DASH_MODULE_EDGE_GAP = 12;

function clamp(value, min, max) {
  return Math.min(Math.max(value, min), max);
}

function clampModuleGeometry(left, top, width, height, workspace = getWorkspaceRoot()) {
  if (!workspace) {
    return null;
  }
  const workspaceWidth = Math.max(0, workspace.clientWidth - DASH_WORKSPACE_PADDING * 2);
  const workspaceHeight = Math.max(0, workspace.clientHeight - DASH_WORKSPACE_PADDING * 2);
  const safeWidth = clamp(
    width,
    DASH_MODULE_MIN_WIDTH,
    Math.max(DASH_MODULE_MIN_WIDTH, workspaceWidth),
  );
  const safeHeight = clamp(
    height,
    DASH_MODULE_MIN_HEIGHT,
    Math.max(DASH_MODULE_MIN_HEIGHT, workspaceHeight),
  );
  const maxLeft = Math.max(
    DASH_WORKSPACE_PADDING,
    workspace.clientWidth - safeWidth - DASH_WORKSPACE_PADDING,
  );
  const maxTop = Math.max(
    DASH_WORKSPACE_PADDING,
    workspace.clientHeight - safeHeight - DASH_WORKSPACE_PADDING,
  );
  return {
    left: clamp(left, DASH_WORKSPACE_PADDING, maxLeft),
    top: clamp(top, DASH_WORKSPACE_PADDING, maxTop),
    width: safeWidth,
    height: safeHeight,
  };
}

function parseGeometry(raw = {}, fallback = {}) {
  const toNumber = (value) => {
    const parsed = Number.parseFloat(value);
    return Number.isFinite(parsed) ? parsed : null;
  };
  const left = toNumber(raw.left);
  const top = toNumber(raw.top);
  const width = toNumber(raw.width);
  const height = toNumber(raw.height);
  if (left === null || top === null || width === null || height === null) {
    return null;
  }
  return { left, top, width, height };
}

function applyModuleGeometry(module, geometry) {
  if (!module || !geometry) {
    return;
  }
  const clamped = clampModuleGeometry(
    geometry.left,
    geometry.top,
    geometry.width,
    geometry.height,
  );
  if (!clamped) {
    return;
  }
  module.style.left = `${clamped.left}px`;
  module.style.top = `${clamped.top}px`;
  module.style.width = `${clamped.width}px`;
  module.style.height = `${clamped.height}px`;
}

function getModuleGeometry(module) {
  if (!module) {
    return null;
  }
  return parseGeometry({
    left: module.style.left,
    top: module.style.top,
    width: module.style.width,
    height: module.style.height,
  });
}

function applyDefaultLayout() {
  const workspace = getWorkspaceRoot();
  if (!workspace) return;
  const modules = getWorkspaceModules();
  if (!modules.length) {
    return;
  }
  const width = Math.max(0, workspace.clientWidth - DASH_WORKSPACE_PADDING * 2);
  const columns = width >= 980 ? 2 : 1;
  const moduleWidth = clamp(
    columns === 2 ? (width - DASH_MODULE_EDGE_GAP) / 2 : width,
    DASH_MODULE_MIN_WIDTH,
    Math.min(520, width),
  );
  const moduleHeight = clamp(
    Math.max(DASH_MODULE_MIN_HEIGHT, Math.floor(workspace.clientHeight * 0.43)),
    DASH_MODULE_MIN_HEIGHT,
    460,
  );
  for (const module of modules) {
    const geometry = getModuleGeometry(module);
    if (geometry) {
      applyModuleGeometry(module, geometry);
      continue;
    }
    const index = modules.indexOf(module);
    const left = DASH_WORKSPACE_PADDING + (index % columns) * (moduleWidth + DASH_MODULE_EDGE_GAP);
    const top = DASH_WORKSPACE_PADDING + Math.floor(index / columns) * (moduleHeight + DASH_MODULE_EDGE_GAP);
    applyModuleGeometry(module, { left, top, width: moduleWidth, height: moduleHeight });
  }
}

function focusModule(module) {
  if (!module) return;
  const workspace = getWorkspaceRoot();
  if (!workspace) return;
  let topLayer = 1000;
  for (const next of getWorkspaceModules()) {
    if (next === module) continue;
    const z = Number.parseInt(next.style.zIndex, 10);
    if (Number.isFinite(z) && z > topLayer) {
      topLayer = z;
    }
    next.classList.remove('dash-module-focused');
  }
  module.style.zIndex = String(topLayer + 1);
  module.classList.add('dash-module-focused');
}

function getWorkspaceRoot() {
  return document.getElementById('dashWorkspace');
}

function getSurfaceForKey(key) {
  return DASH_SURFACES.find((surface) => surface.key === key);
}

function getModuleByKey(key) {
  const workspace = getWorkspaceRoot();
  if (!workspace || !key) {
    return null;
  }
  return workspace.querySelector(`[data-surface-module="${key}"]`);
}

function getWorkspaceModules() {
  const workspace = getWorkspaceRoot();
  if (!workspace) {
    return [];
  }
  return [...workspace.querySelectorAll('[data-surface-module]')];
}

function getModuleKeyFromElement(element) {
  return element ? element.getAttribute('data-surface-module') : null;
}

function currentMaximizedModule() {
  const workspace = getWorkspaceRoot();
  return workspace?.getAttribute('data-maximized-module') || '';
}

function loadWorkspaceState() {
  try {
    const raw = localStorage.getItem(DASH_WORKSPACE_STATE_KEY);
    if (!raw) {
      return {};
    }
    const state = JSON.parse(raw);
    return state && typeof state === 'object' ? state : {};
  } catch (_error) {
    return {};
  }
}

function persistWorkspaceState() {
  const workspace = getWorkspaceRoot();
  if (!workspace) return;
  const modules = getWorkspaceModules();
  const geometry = {};
  for (const module of modules) {
    const key = module.dataset.moduleKey;
    if (!key) continue;
    const moduleGeometry = getModuleGeometry(module);
    if (moduleGeometry) {
      geometry[key] = moduleGeometry;
    }
  }
  const state = {
    order: modules.map((module) => module.dataset.moduleKey).filter(Boolean),
    minimized: modules
      .filter((module) => module.classList.contains('dash-module-minimized'))
      .map((module) => module.dataset.moduleKey)
      .filter(Boolean),
    maximized: workspace.getAttribute('data-maximized-module') || '',
    geometry,
  };
  try {
    localStorage.setItem(DASH_WORKSPACE_STATE_KEY, JSON.stringify(state));
  } catch (_error) {
    // ignore persistence failures
  }
}

function applyModuleOrder(order) {
  const workspace = getWorkspaceRoot();
  if (!workspace || !Array.isArray(order)) {
    return;
  }
  const modules = getWorkspaceModules();
  const byKey = new Map(modules.map((module) => [module.dataset.moduleKey, module]));
  const seen = new Set();
  for (const key of order) {
    const module = byKey.get(key);
    if (!module || seen.has(key)) {
      continue;
    }
    workspace.appendChild(module);
    seen.add(key);
  }
  for (const module of modules) {
    if (!seen.has(module.dataset.moduleKey)) {
      workspace.appendChild(module);
    }
  }
}

function refreshDockButtonState() {
  const buttons = document.querySelectorAll('.dash-dock-button[data-dock-module]');
  for (const button of buttons) {
    const key = button.getAttribute('data-dock-module');
    const module = getModuleByKey(key);
    if (!module) {
      continue;
    }
    button.classList.toggle('is-restored', !module.classList.contains('dash-module-minimized'));
  }
}

function setModuleMinimized(module, minimized) {
  if (!module) return;
  if (minimized) {
    module.classList.add('dash-module-minimized');
  } else {
    module.classList.remove('dash-module-minimized');
  }
  refreshDockButtonState();
}

function isInteractiveSurfaceTarget(target) {
  if (!target) {
    return false;
  }
  return !!target.closest(
    'a, button, input, textarea, select, option, label, .dash-chat-panel-button, [data-chat-action], [data-surface], [href], [data-module-context-action]',
  );
}

function ensureModuleContextMenu() {
  const existing = document.getElementById('dashModuleContextMenu');
  if (existing) {
    return existing;
  }
  const menu = document.createElement('section');
  menu.id = 'dashModuleContextMenu';
  menu.className = 'dash-module-context-menu';
  menu.setAttribute('role', 'menu');
  menu.setAttribute('aria-hidden', 'true');
  document.body.appendChild(menu);
  return menu;
}

function hideModuleContextMenu() {
  const menu = document.getElementById('dashModuleContextMenu');
  if (!menu) return;
  menu.classList.remove('is-open');
  menu.setAttribute('aria-hidden', 'true');
}

function clampMenuPosition(event) {
  const viewportHeight = Math.max(0, window.innerHeight - 12);
  const viewportWidth = Math.max(0, window.innerWidth - 12);
  const menu = ensureModuleContextMenu();
  const rect = menu.getBoundingClientRect();
  const estimatedWidth = Math.max(170, rect.width || 170);
  const estimatedHeight = Math.max(150, rect.height || 150);
  return {
    left: clamp(event.clientX, 12, viewportWidth - estimatedWidth),
    top: clamp(event.clientY, 12, viewportHeight - estimatedHeight),
  };
}

async function applyModuleContextAction(module, action) {
  const key = getModuleKeyFromElement(module);
  const surface = getSurfaceForKey(key);
  if (!key) return;
  const maximized = currentMaximizedModule();
  if (action === 'open') {
    if (module.classList.contains('dash-module-minimized')) {
      setModuleMinimized(module, false);
    }
    focusModule(module);
    if (maximized && maximized !== key) {
      setMaximizedModule('');
    }
    if (surface) {
      await hydrateSurface(surface, surface.pathPrefix);
    }
    return;
  }
  if (action === 'minimize') {
    const shouldMinimize = !module.classList.contains('dash-module-minimized');
    setModuleMinimized(module, shouldMinimize);
    if (shouldMinimize && maximized === key) {
      setMaximizedModule('');
    }
    return;
  }
  if (action === 'maximize') {
    setMaximizedModule(maximized === key ? '' : key);
    focusModule(module);
    return;
  }
  if (action === 'refresh') {
    if (surface) {
      await hydrateSurface(surface, surface.pathPrefix);
    }
  }
}

function setMaximizedModule(key) {
  const workspace = getWorkspaceRoot();
  if (!workspace) return;
  const modules = getWorkspaceModules();
  workspace.classList.remove('dash-maximized');
  workspace.removeAttribute('data-maximized-module');
  for (const module of modules) {
    module.classList.remove('dash-module-maximized');
  }
  if (!key) {
    refreshDockButtonState();
    persistWorkspaceState();
    return;
  }
  const module = getModuleByKey(key);
  if (!module) return;
  setModuleMinimized(module, false);
  module.classList.add('dash-module-maximized');
  workspace.classList.add('dash-maximized');
  workspace.setAttribute('data-maximized-module', key);
  refreshDockButtonState();
  persistWorkspaceState();
}

function hydrateAll() {
  return Promise.all(
    DASH_SURFACES.map((surface) => hydrateSurface(surface, surface.pathPrefix)),
  );
}

function applyWorkspaceState(rawState) {
  const workspace = getWorkspaceRoot();
  if (!workspace) return;
  const state = rawState || {};
  const available = DASH_SURFACES.map((surface) => surface.key);
  const order = Array.isArray(state.order) ? state.order.filter((key) => available.includes(key)) : [];
  if (order.length === available.length) {
    applyModuleOrder(order);
  }
  const minimized = new Set(
    Array.isArray(state.minimized) ? state.minimized.filter((key) => available.includes(key)) : [],
  );
  for (const module of getWorkspaceModules()) {
    setModuleMinimized(module, minimized.has(module.dataset.moduleKey));
  }
  const savedGeometry =
    state.geometry && typeof state.geometry === 'object' ? state.geometry : {};
  for (const module of getWorkspaceModules()) {
    const key = module.dataset.moduleKey;
    const rawGeometry = key ? parseGeometry(savedGeometry[key] || {}) : null;
    if (!key || !rawGeometry) {
      continue;
    }
    applyModuleGeometry(module, rawGeometry);
  }
  applyDefaultLayout();
  const maximized = typeof state.maximized === 'string' ? state.maximized : '';
  if (maximized && available.includes(maximized)) {
    setMaximizedModule(maximized);
  } else {
    setMaximizedModule('');
  }
  refreshDockButtonState();
}

function initWorkspaceState() {
  applyWorkspaceState(loadWorkspaceState());
}

function normalizeSurfacePath(path) {
  return path.startsWith('/') ? path : `/${path}`;
}

function pathToSurfaceKey(path) {
  if (!path) {
    return null;
  }
  if (path.startsWith('/surface/blog')) {
    return 'build-log';
  }
  if (path.startsWith('/surface/git')) {
    return 'code';
  }
  if (path.startsWith('/surface/mail')) {
    return 'mail';
  }
  if (path.startsWith('/surface/apps')) {
    return 'apps';
  }
  return null;
}

function formatBytes(bytes) {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return '--';
  }
  const units = ['B', 'KB', 'MB', 'GB'];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return (value >= 10 || unit === 0 ? value.toFixed(0) : value.toFixed(1)) + ' ' + units[unit];
}

  function extractSurfaceContent(html) {
    const doc = new DOMParser().parseFromString(html, 'text/html');
    const slot = doc.querySelector('#surfaceSlot');
    if (slot) {
      return slot.outerHTML;
    }
    const wrapped = doc.querySelector('[data-dash-surface-root]');
    if (wrapped) {
      return wrapped.outerHTML;
    }
    return doc.querySelector('.dash-surface')?.outerHTML || html;
  }

async function hydrateSurface(surface, path) {
  if (!surface) {
    return;
  }
  const fallback = surface.fallback;
  const resolvedPath = normalizeSurfacePath(path || surface.pathPrefix);
  const slot = document.getElementById(surface.slotId);
  if (!slot) {
    return;
  }
  slot.classList.remove('dash-surface-empty');
  try {
    const response = await fetch(resolvedPath, { cache: 'no-store' });
    if (!response.ok) {
      throw new Error('surface unavailable');
    }
    const html = await response.text();
    slot.outerHTML = extractSurfaceContent(html);
    bindSurfaceSearchByKey(surface.key);
  } catch (_error) {
    slot.textContent = fallback;
    slot.classList.add('dash-surface-empty');
  }
}

function findSurfaceContainer(key) {
  const surface = DASH_SURFACES.find((s) => s.key === key);
  if (!surface) return null;
  const module = getModuleByKey(key);
  if (!module) return null;
  return module.querySelector('#surfaceSlot') || module.querySelector('.dash-surface') || module;
}

function bindSurfaceSearchByKey(key) {
  const container = findSurfaceContainer(key);
  if (!container) return;
  bindSurfaceSearch(container);
}

function bindSurfaceSearch(container) {
  const input = container.querySelector('[data-workspace-search-scope]');
  if (!input) {
    return;
  }
  const cards = [...container.querySelectorAll('[data-search-card]')];
  const empty = container.querySelector('[data-search-empty]');
  if (!cards.length) {
    return;
  }
  const apply = () => {
    const term = input.value.trim().toLowerCase();
    let visible = 0;
    for (const card of cards) {
      const haystack = (card.getAttribute('data-search-text') || '').toLowerCase();
      const show = term.length === 0 || haystack.includes(term);
      card.hidden = !show;
      if (show) {
        visible += 1;
      }
    }
    if (empty) {
      empty.hidden = visible > 0;
    }
  };
  input.addEventListener('input', apply);
  apply();
}

function resolveSurfaceFromClickTarget(target) {
  const raw =
    target.getAttribute('hx-get') ||
    target.getAttribute('href');
  if (!raw) {
    return null;
  }
  if (raw.startsWith('#')) {
    return null;
  }
  if (raw.startsWith('http')) {
    const parsed = new URL(raw, location.origin);
    if (parsed.origin !== location.origin) {
      return null;
    }
    if (!parsed.pathname.startsWith('/surface/')) {
      return null;
    }
    return {
      key: pathToSurfaceKey(parsed.pathname),
      path: parsed.pathname,
    };
  }
  if (raw.startsWith('/surface/')) {
    return { key: pathToSurfaceKey(raw), path: raw };
  }
  return null;
}

function bindSurfaceLinks() {
  document.addEventListener('click', (event) => {
    const button = event.target.closest('[hx-get], [href], [data-surface]');
    if (!button) {
      return;
    }
    const resolved = resolveSurfaceFromClickTarget(button);
    if (!resolved || !resolved.path || !resolved.key) {
      return;
    }
    const container = button.closest('[data-surface-module]');
    const explicit = button.getAttribute('data-surface');
    const moduleKey = explicit || getModuleKeyFromElement(container);
    const surface =
      getSurfaceForKey(moduleKey) ||
      getSurfaceForKey(resolved.key);
    if (!surface) {
      return;
    }
    event.preventDefault();
    const resolvedPath = normalizeSurfacePath(resolved.path);
    hydrateSurface(surface, resolvedPath);
  });
}

async function refreshDashStatus(options = {}) {
  const setText = (selector, text) => {
    for (const node of document.querySelectorAll(selector)) {
      node.textContent = text;
    }
  };
  const setRefreshState = (disabled) => {
    for (const node of document.querySelectorAll('[data-status-refresh]')) {
      node.disabled = disabled;
      node.classList.toggle('is-loading', disabled);
    }
  };
  try {
    if (options.manual) {
      setRefreshState(true);
    }
    const response = await fetch('/status.json', { cache: 'no-store' });
    if (!response.ok) return;
    const data = await response.json();
    if (!data.ok) return;
    setText('[data-status-sessions]', String(data.sessions));
    setText('[data-status-rps]', (Number(data.requests_per_second) || 0).toFixed(1));
    setText('[data-status-memory]', formatBytes(data.memory_bytes));
    setText('[data-status-cpu]', (Number(data.cpu_percent) || 0).toFixed(1) + '%');
    setText('[data-status-binary]', formatBytes(data.binary_bytes));
  } catch (_error) {
    // no-op on status refresh failure
  } finally {
    setRefreshState(false);
  }
}

function chatMessageTemplate(message) {
  const safeName = String(message.name || 'Guest');
  const safeText = String(message.message || '');
  return `<article class="dash-chat-entry"><header class="dash-chat-meta"><span class="dash-chat-name">${escapeHtml(safeName)}</span><time>${new Date((message.at || 0) * 1000).toLocaleTimeString([], {hour:'2-digit', minute:'2-digit'})}</time></header><p class="dash-chat-text">${escapeHtml(safeText)}</p></article>`;
}

function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (match) => {
    const map = {
      '&': '&amp;',
      '<': '&lt;',
      '>': '&gt;',
      '"': '&quot;',
      "'": '&#39;',
    };
    return map[match];
  });
}

async function refreshChatLog() {
  const status = document.getElementById('dashChatStatus');
  const log = document.getElementById('dashChatLog');
  if (!log) return;
  try {
    const response = await fetch('/api/chat', { cache: 'no-store' });
    if (!response.ok) {
      if (status) status.textContent = 'Unable to load chat messages.';
      log.innerHTML = '';
      return;
    }
    const data = await response.json();
    const messages = (data && Array.isArray(data.messages)) ? data.messages : [];
    if (messages.length === 0) {
      log.innerHTML = '<p class="dash-surface-empty">No messages yet.</p>';
      if (status) {
        status.textContent = '';
      }
      return;
    }
    log.innerHTML = messages.map(chatMessageTemplate).join('');
    log.scrollTop = log.scrollHeight;
  } catch (_error) {
    if (status) status.textContent = 'Unable to load chat messages.';
  }
}

function wireGlobalChat() {
  const form = document.getElementById('dashChatForm');
  const status = document.getElementById('dashChatStatus');
  const nameInput = document.getElementById('dashChatName');
  const messageInput = document.getElementById('dashChatMessage');
  const submit = document.getElementById('dashChatSend');
  const chatState = {
    lastPostTs: 0,
    messageHistory: new Map(),
  };

  function isLikelySpam(name, message) {
    const normalizedName = String(name || '').trim().toLowerCase();
    const normalizedMessage = String(message || '').trim();
    if (!normalizedName || !normalizedMessage) {
      return 'Name and message are required.';
    }
    if (normalizedName.length > 24) {
      return 'Name is too long.';
    }
    if (normalizedMessage.length > DASH_CHAT_MESSAGE_MAX) {
      return `Message must be under ${DASH_CHAT_MESSAGE_MAX} characters.`;
    }
    if (/(.)\1{12,}/.test(normalizedMessage)) {
      return 'Please avoid repetitive characters.';
    }
    const now = Date.now();
    if (now - chatState.lastPostTs < DASH_CHAT_MIN_DELAY_MS) {
      return 'Please wait before posting again.';
    }
    const history = (chatState.messageHistory.get(normalizedName) || []).filter(
      (entry) => now - entry.time < DASH_CHAT_DUP_WINDOW_MS,
    );
    if (history.length >= DASH_CHAT_REPEAT_WINDOW_LIMIT) {
      if (history.some((entry) => entry.message === normalizedMessage)) {
        return 'This looks spammy. Please adjust message.';
      }
      if (history.length > DASH_CHAT_REPEAT_WINDOW_LIMIT) {
        return 'Posting too frequently from this name.';
      }
    }
    if (history.some((entry) => entry.message === normalizedMessage)) {
      return 'You already posted this message just now.';
    }
    history.push({
      message: normalizedMessage,
      time: now,
    });
    chatState.messageHistory.set(normalizedName, history);
    chatState.lastPostTs = now;
    return '';
  }

  if (!form || !status || !nameInput || !messageInput || !submit) return;
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    const name = nameInput.value.trim();
    const message = messageInput.value.trim();
    const reason = isLikelySpam(name, message);
    if (reason) {
      status.textContent = reason;
      return;
    }
    submit.disabled = true;
    submit.textContent = 'Sending…';
    status.textContent = '';
    try {
      const response = await fetch('/api/chat', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Accept: 'application/json',
        },
        body: JSON.stringify({
          name,
          message,
        }),
      });
      const result = await response.json();
      if (!response.ok || !result || result.ok !== true) {
        status.textContent = result?.error || 'Post failed';
        return;
      }
      nameInput.value = name;
      messageInput.value = '';
      await refreshChatLog();
      status.textContent = 'Posted.';
      setTimeout(() => {
        status.textContent = '';
      }, 1500);
    } catch (_error) {
      status.textContent = 'Could not send chat message.';
    } finally {
      submit.disabled = false;
      submit.textContent = 'Post';
    }
  });
}

function getChatDockState() {
  try {
    const raw = localStorage.getItem(DASH_CHAT_STATE_KEY);
    if (raw === 'open' || raw === 'minimized' || raw === 'closed') {
      return raw;
    }
  } catch (_error) {
    // ignore
  }
  return 'minimized';
}

function setChatDockState(state) {
  const dock = document.getElementById('dashChatDock');
  const panel = document.getElementById('dashChatPanel');
  if (!dock || !panel) return;
  const normalized = state === 'minimized' || state === 'closed' ? state : 'open';
  dock.classList.remove('is-open', 'is-minimized', 'is-closed');
  panel.classList.remove('dash-chat-minimized', 'dash-chat-closed');
  if (normalized === 'open') {
    dock.classList.add('is-open');
  }
  if (normalized === 'minimized') {
    dock.classList.add('is-minimized');
    panel.classList.add('dash-chat-minimized');
  }
  if (normalized === 'closed') {
    dock.classList.add('is-closed');
    panel.classList.add('dash-chat-closed');
  }
  try {
    localStorage.setItem(DASH_CHAT_STATE_KEY, normalized);
  } catch (_error) {
    // ignore
  }
}

function wireChatDock() {
  const dock = document.getElementById('dashChatDock');
  if (!dock) return;
  dock.addEventListener('click', (event) => {
    const button = event.target.closest('[data-chat-action]');
    if (!button) return;
    event.preventDefault();
    const action = button.getAttribute('data-chat-action');
    if (action === 'open') {
      setChatDockState('open');
      return;
    }
    if (action === 'minimize') {
      setChatDockState('minimized');
      return;
    }
    if (action === 'close') {
      setChatDockState('closed');
      return;
    }
  });
  if (!document.getElementById('dashChatPanel')) {
    return;
  }
  setChatDockState(getChatDockState());
}

function wireModuleControls() {
  const workspace = getWorkspaceRoot();
  if (!workspace) return;
  const menu = ensureModuleContextMenu();
  menu.addEventListener('click', (event) => {
    const button = event.target.closest('[data-module-context-action]');
    if (!button) return;
    const action = button.getAttribute('data-module-context-action');
    const key = button.getAttribute('data-module-context-key');
    const module = getModuleByKey(key);
    if (!module) {
      hideModuleContextMenu();
      return;
    }
    event.preventDefault();
    applyModuleContextAction(module, action).finally(() => {
      hideModuleContextMenu();
      persistWorkspaceState();
      refreshDockButtonState();
    });
  });

  workspace.addEventListener('contextmenu', async (event) => {
    const module = event.target.closest('.dash-module');
    if (!module) return;
    if (isInteractiveSurfaceTarget(event.target)) {
      return;
    }
    event.preventDefault();
    focusModule(module);
    const key = getModuleKeyFromElement(module);
    const minimized = module.classList.contains('dash-module-minimized');
    const maximized = currentMaximizedModule() === key;
    const items = [
      {
        action: 'open',
        icon: '🡹',
        label: module.classList.contains('dash-module-minimized') ? 'Restore' : 'Open',
      },
      { action: 'refresh', icon: '↻', label: 'Refresh content' },
      { action: minimized ? 'open' : 'minimize', icon: minimized ? '📌' : '▾', label: minimized ? 'Unminimize' : 'Minimize' },
      { action: 'maximize', icon: maximized ? '⤢' : '▣', label: maximized ? 'Restore layout' : 'Maximize' },
    ];
    const entryHtml = items
      .map(
        (entry) =>
          `<button type="button" role="menuitem" data-module-context-key="${key}" data-module-context-action="${entry.action}" aria-label="${entry.label}"><span>${entry.icon}</span>${entry.label}</button>`,
      )
      .join('');
    menu.innerHTML = entryHtml;
    menu.setAttribute('aria-hidden', 'false');
    menu.classList.add('is-open');
    const menuPosition = clampMenuPosition(event);
    menu.style.left = `${menuPosition.left}px`;
    menu.style.top = `${menuPosition.top}px`;
  });

  addEventListener('click', (event) => {
    if (!menu.classList.contains('is-open')) {
      return;
    }
    const isMenu = event.target.closest('#dashModuleContextMenu');
    if (isMenu) {
      return;
    }
    hideModuleContextMenu();
  });

  addEventListener('keydown', (event) => {
    if (event.key === 'Escape') {
      hideModuleContextMenu();
    }
  });
  addEventListener('scroll', hideModuleContextMenu);
  addEventListener('resize', hideModuleContextMenu);
}

function wireWorkspaceDock() {
  const dock = document.getElementById('dashDock');
  if (!dock) return;
  const buttons = [...dock.querySelectorAll('.dash-dock-button')];

  function resetDockIcons() {
    for (const button of buttons) {
      button.style.setProperty('--dash-icon-scale', '1');
      button.style.setProperty('--dash-icon-dy', '0px');
    }
  }

  dock.addEventListener('click', async (event) => {
    const button = event.target.closest('[data-dock-module]');
    if (!button) return;
    event.preventDefault();
    const key = button.getAttribute('data-dock-module');
    const surface = getSurfaceForKey(key);
    const module = getModuleByKey(key);
    if (!surface || !module) {
      return;
    }
    if (module.classList.contains('dash-module-minimized')) {
      setModuleMinimized(module, false);
      focusModule(module);
      await hydrateSurface(surface, surface.pathPrefix);
      return;
    }
    const maximized = currentMaximizedModule();
    if (maximized === key) {
      setMaximizedModule('');
    } else {
      setMaximizedModule(key);
    }
    focusModule(module);
    module.scrollIntoView({ behavior: 'smooth', block: 'start' });
    await hydrateSurface(surface, surface.pathPrefix);
  });

  dock.addEventListener('pointermove', (event) => {
    if (buttons.length === 0) {
      return;
    }
    const dockRect = dock.getBoundingClientRect();
    const cursorX = event.clientX;
    for (const dockButton of buttons) {
      const buttonRect = dockButton.getBoundingClientRect();
      const centerX = buttonRect.left + buttonRect.width / 2;
      const distance = Math.abs(cursorX - centerX);
      const radius = Math.max(dockRect.width / 2, 180);
      const intensity = Math.max(0, 1 - distance / radius);
      const scale = 1 + (intensity * 0.46);
      const up = (intensity * 10) - 2;
      dockButton.style.setProperty('--dash-icon-scale', `${scale}`);
      dockButton.style.setProperty('--dash-icon-dy', `${up * -1}px`);
    }
  });

  dock.addEventListener('pointerleave', resetDockIcons);
  if (window.matchMedia && window.matchMedia('(hover: none)').matches) {
    resetDockIcons();
  }
}

function wireModuleDragReorder() {
  const workspace = getWorkspaceRoot();
  if (!workspace) return;
  let dragging = null;
  let pointerId = null;
  let pointerStartX = 0;
  let pointerStartY = 0;
  let startLeft = 0;
  let startTop = 0;

  const onPointerMove = (event) => {
    if (!dragging || event.pointerId !== pointerId) {
      return;
    }
    const moduleWidth = dragging.clientWidth;
    const moduleHeight = dragging.clientHeight;
    const left = startLeft + (event.clientX - pointerStartX);
    const top = startTop + (event.clientY - pointerStartY);
    const clamped = clampModuleGeometry(left, top, moduleWidth, moduleHeight, workspace);
    if (!clamped) return;
    dragging.style.left = `${clamped.left}px`;
    dragging.style.top = `${clamped.top}px`;
  };

  const stopDragging = (event) => {
    if (!dragging || event.pointerId !== pointerId) {
      return;
    }
    dragging.classList.remove('dash-module-dragging');
    dragging.classList.remove('is-dragging');
    try {
      dragging.releasePointerCapture(pointerId);
    } catch (_error) {
      // ignore
    }
    dragging = null;
    pointerId = null;
    removeEventListener('pointermove', onPointerMove);
    removeEventListener('pointerup', stopDragging);
    removeEventListener('pointercancel', stopDragging);
    persistWorkspaceState();
  };

  const onPointerDown = (event) => {
    const module = event.target.closest('.dash-module');
    if (!module || event.button !== 0 || isInteractiveSurfaceTarget(event.target) || currentMaximizedModule()) {
      if (module) {
        focusModule(module);
      }
      return;
    }
    event.preventDefault();
    dragging = module;
    pointerId = event.pointerId;
    pointerStartX = event.clientX;
    pointerStartY = event.clientY;
    startLeft = module.offsetLeft;
    startTop = module.offsetTop;
    dragging.classList.add('dash-module-dragging');
    dragging.classList.add('is-dragging');
    focusModule(dragging);
    try {
      dragging.setPointerCapture(pointerId);
    } catch (_error) {
      // ignore
    }
    addEventListener('pointermove', onPointerMove);
    addEventListener('pointerup', stopDragging);
    addEventListener('pointercancel', stopDragging);
  };

  const observer = new ResizeObserver(() => {
    if (!dragging) {
      persistWorkspaceState();
    }
  });

  for (const module of getWorkspaceModules()) {
    module.addEventListener('pointerdown', onPointerDown);
    observer.observe(module);
  }
  addEventListener('resize', () => {
    for (const module of getWorkspaceModules()) {
      const geometry = getModuleGeometry(module);
      if (geometry) {
        applyModuleGeometry(module, geometry);
      }
    }
    persistWorkspaceState();
  });
}

addEventListener('click', (event) => {
  const refresh = event.target.closest('[data-status-refresh]');
  if (!refresh) return;
  event.preventDefault();
  refreshDashStatus({ manual: true });
});

async function initDashboard() {
  initWorkspaceState();
  bindSurfaceLinks();
  if (typeof wireModuleControls === 'function') {
    wireModuleControls();
  }
  if (typeof wireWorkspaceDock === 'function') {
    wireWorkspaceDock();
  }
  if (typeof wireModuleDragReorder === 'function') {
    wireModuleDragReorder();
  }
  if (typeof wireChatDock === 'function') {
    wireChatDock();
  }
  await hydrateAll();
  await refreshChatLog();
  wireGlobalChat();
  refreshDashStatus();
  setInterval(refreshDashStatus, 5000);
  setInterval(refreshChatLog, 8000);
}

if (document.readyState === 'loading') {
  addEventListener('DOMContentLoaded', initDashboard);
} else {
  initDashboard();
}