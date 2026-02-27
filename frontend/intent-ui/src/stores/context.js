const RECENT_COMMANDS_KEY = "intent-ui-recent-commands-v1";
const MAX_RECENT_COMMANDS = 50;
const loadRecentCommands = () => {
  if (typeof window === "undefined") return [];
  try {
    const raw = localStorage.getItem(RECENT_COMMANDS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((item) => typeof item === "string" && item.trim()).slice(0, MAX_RECENT_COMMANDS);
  } catch {
    return [];
  }
};
const persistRecentCommands = (commands) => {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(RECENT_COMMANDS_KEY, JSON.stringify(commands.slice(0, MAX_RECENT_COMMANDS)));
  } catch {
    // ignore storage failures
  }
};
const context = {
  currentRepo: "demo/repo",
  currentBranch: "main",
  currentHost: "localhost",
  currentProject: "solid-components-showcase",
  recentFiles: ["src/App.jsx", "components/ui/Button.jsx"],
  recentCommands: loadRecentCommands(),
  activeIntegrations: ["github", "cloudflare"],
  environment: "development",
  openWindows: []
};
function getRecentCommands() {
  if (!Array.isArray(context.recentCommands)) return [];
  return context.recentCommands.filter((item) => typeof item === "string" && item.trim());
}
function addRecentCommand(command) {
  const normalized = typeof command === "string" ? command.trim() : "";
  if (!normalized) return;
  const deduped = getRecentCommands().filter((item) => item !== normalized);
  const next = [normalized, ...deduped].slice(0, MAX_RECENT_COMMANDS);
  context.recentCommands = next;
  persistRecentCommands(next);
}
function addOpenWindow(id) {
  if (!context.openWindows.includes(id)) {
    context.openWindows = [...context.openWindows, id];
  }
}
function removeOpenWindow(id) {
  context.openWindows = context.openWindows.filter((w) => w !== id);
}
export {
  addOpenWindow,
  addRecentCommand,
  context,
  getRecentCommands,
  removeOpenWindow
};
