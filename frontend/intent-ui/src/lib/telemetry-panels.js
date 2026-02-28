function normalizeStatus(status) {
  return String(status || "").toLowerCase();
}

export function executorStatusPriority(status) {
  const normalized = normalizeStatus(status);
  if (normalized === "failure" || normalized === "error") return 0;
  if (normalized === "started" || normalized === "running" || normalized === "pending") return 1;
  if (normalized === "success" || normalized === "ok") return 2;
  return 3;
}

export function severityBadgeClass(severity) {
  if (severity === "critical") return "border-red-500/60 bg-red-500/10 text-red-200";
  if (severity === "warn") return "border-amber-500/60 bg-amber-500/10 text-amber-200";
  if (severity === "ok") return "border-emerald-500/60 bg-emerald-500/10 text-emerald-200";
  return "border-neutral-600/70 bg-neutral-700/30 text-neutral-200";
}

export function formatEventPayload(payload) {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) return "";
  const keys = Object.keys(payload);
  if (keys.length === 0) return "";
  const preview = {};
  for (const key of keys.slice(0, 3)) preview[key] = payload[key];
  try {
    return JSON.stringify(preview);
  } catch {
    return "";
  }
}

export function formatEventAge(createdAt) {
  const raw = createdAt ?? Date.now();
  let ts = Number.NaN;
  if (typeof raw === "number") {
    ts = raw;
  } else if (typeof raw === "string" && /^\d+$/.test(raw.trim())) {
    const numeric = Number(raw.trim());
    ts = numeric < 1_000_000_000_000 ? numeric * 1000 : numeric;
  } else {
    ts = new Date(raw).getTime();
  }
  if (!Number.isFinite(ts)) return "just now";
  const delta = Math.max(0, Math.floor((Date.now() - ts) / 1000));
  if (delta < 5) return "just now";
  if (delta < 60) return `${delta}s ago`;
  const minutes = Math.floor(delta / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

export function createDefaultFloatingLayouts() {
  const viewportWidth = typeof window === "undefined" ? 1360 : window.innerWidth;
  const viewportHeight = typeof window === "undefined" ? 900 : window.innerHeight;
  const panelWidth = 320;
  const panelHeight = 184;
  const gap = 12;
  return {
    eventBus: {
      x: Math.max(12, viewportWidth - panelWidth - 12),
      y: Math.max(12, viewportHeight - panelHeight - 96),
      width: panelWidth,
      height: panelHeight
    },
    dockerLogs: {
      x: Math.max(12, viewportWidth - (panelWidth * 2) - gap - 12),
      y: Math.max(12, viewportHeight - panelHeight - 96),
      width: panelWidth,
      height: panelHeight
    },
    systemState: {
      x: Math.max(12, viewportWidth - (panelWidth * 3) - gap * 2 - 12),
      y: Math.max(12, viewportHeight - panelHeight - 96),
      width: panelWidth,
      height: panelHeight
    }
  };
}

function getExecutorSeverity(status) {
  const priority = executorStatusPriority(status);
  if (priority === 0) return "critical";
  if (priority === 1) return "warn";
  if (priority === 2) return "ok";
  return "info";
}

function collectLatestExecutorState(events) {
  const executorState = new Map();
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const event = events[index];
    const topic = String(event?.topic || "");
    const payload = event?.payload && typeof event.payload === "object" ? event.payload : {};
    if (
      topic.startsWith("edgerun.executors.") &&
      topic.endsWith(".status") &&
      payload?.crate &&
      payload?.mode &&
      payload?.status
    ) {
      const key = `${payload.crate}:${payload.mode}`;
      if (!executorState.has(key)) {
        executorState.set(key, {
          crate: String(payload.crate),
          mode: String(payload.mode),
          status: String(payload.status),
          detail: String(payload.detail || "").trim(),
          updatedAt: event?.createdAt || ""
        });
      }
    }
  }
  return [...executorState.values()].sort((a, b) =>
    executorStatusPriority(a.status) - executorStatusPriority(b.status)
    || a.crate.localeCompare(b.crate)
    || a.mode.localeCompare(b.mode)
  );
}

function summarizeExecutors(entries) {
  return entries.reduce((acc, entry) => {
    const normalized = normalizeStatus(entry.status);
    if (normalized === "failure" || normalized === "error") acc.failure += 1;
    else if (normalized === "started" || normalized === "running" || normalized === "pending") acc.started += 1;
    else if (normalized === "success" || normalized === "ok") acc.success += 1;
    else acc.other += 1;
    return acc;
  }, { failure: 0, started: 0, success: 0, other: 0 });
}

function buildGlobalState(events) {
  let latestCodeRevision = "";
  let latestDiffProposed = "";
  let latestDiffAccepted = "";
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const event = events[index];
    const topic = String(event?.topic || "");
    const payload = event?.payload && typeof event.payload === "object" ? event.payload : {};
    if (!latestCodeRevision && (topic === "edgerun.code.updated" || payload?.event_type === "code_updated")) {
      latestCodeRevision = String(payload?.revision || "").trim();
    }
    if (!latestDiffProposed && (topic.includes(".diff.proposed") || payload?.kind === "agent.diff.proposed")) {
      latestDiffProposed = String(payload?.run_id || payload?.runId || "").trim() || "received";
    }
    if (!latestDiffAccepted && (topic.includes(".diff.accepted") || payload?.event_type === "agent_diff_accepted")) {
      latestDiffAccepted = String(payload?.run_id || payload?.runId || "").trim() || "received";
    }
    if (latestCodeRevision && latestDiffProposed && latestDiffAccepted) break;
  }
  return { latestCodeRevision, latestDiffProposed, latestDiffAccepted };
}

export function buildSystemStateItems(params) {
  const {
    events,
    eventBusFilterLabel,
    localBridgeConnected,
    localBridgeStatus,
    localBridgeError,
    knownDevices
  } = params;
  const executorEntriesRaw = collectLatestExecutorState(events);
  const executorSummary = summarizeExecutors(executorEntriesRaw);
  const { latestCodeRevision, latestDiffProposed, latestDiffAccepted } = buildGlobalState(events);
  const executorEntries = executorEntriesRaw.slice(0, 8).map((entry) => ({
    id: `executor-${entry.crate}-${entry.mode}`,
    label: `${entry.crate} ${entry.mode}`,
    value: entry.status,
    detail: entry.detail,
    updatedAt: entry.updatedAt,
    severity: getExecutorSeverity(entry.status),
    topicFilter: `edgerun.executors.${entry.crate}.${entry.mode}.status`,
    filterType: "exact",
    filterValue: `edgerun.executors.${entry.crate}.${entry.mode}.status`,
    clearFilter: false
  }));
  const connectedDevices = knownDevices.filter((device) => device.online).length;
  const totalDevices = knownDevices.length;
  const items = [
    {
      id: "eventbus-filter",
      label: "event bus filter",
      value: eventBusFilterLabel || "all",
      detail: eventBusFilterLabel ? "click to clear filter" : "click executor row to filter",
      severity: eventBusFilterLabel ? "warn" : "info",
      clearFilter: Boolean(eventBusFilterLabel),
      topicFilter: "",
      filterType: "all",
      filterValue: ""
    },
    {
      id: "eventbus-filter-executors",
      label: "event bus preset",
      value: "all executor statuses",
      detail: "click to show edgerun.executors.*",
      severity: "info",
      clearFilter: false,
      topicFilter: "",
      filterType: "prefix",
      filterValue: "edgerun.executors."
    },
    {
      id: "bridge",
      label: "bridge",
      value: localBridgeConnected ? "connected" : localBridgeStatus || "error",
      detail: localBridgeError || "",
      severity: localBridgeConnected ? "ok" : "critical",
      clearFilter: false,
      topicFilter: "",
      filterType: "all",
      filterValue: ""
    },
    {
      id: "devices",
      label: "devices",
      value: `${connectedDevices}/${totalDevices} online`,
      detail: "",
      severity: connectedDevices > 0 ? "ok" : "warn",
      clearFilter: false,
      topicFilter: "",
      filterType: "all",
      filterValue: ""
    },
    {
      id: "executors-summary",
      label: "executors",
      value: `${executorSummary.success} ok · ${executorSummary.started} running · ${executorSummary.failure} fail`,
      detail: executorSummary.other > 0 ? `${executorSummary.other} unknown` : "",
      severity: executorSummary.failure > 0 ? "critical" : (executorSummary.started > 0 ? "warn" : "ok"),
      clearFilter: false,
      topicFilter: "",
      filterType: "prefix",
      filterValue: "edgerun.executors."
    },
    {
      id: "code",
      label: "code",
      value: latestCodeRevision || "unknown",
      detail: latestCodeRevision ? "latest revision" : "",
      severity: latestCodeRevision ? "ok" : "warn",
      clearFilter: false,
      topicFilter: "",
      filterType: "all",
      filterValue: ""
    },
    {
      id: "diff-proposed",
      label: "diff proposed",
      value: latestDiffProposed || "none",
      detail: "",
      severity: latestDiffProposed ? "warn" : "info",
      clearFilter: false,
      topicFilter: "",
      filterType: "all",
      filterValue: ""
    },
    {
      id: "diff-accepted",
      label: "diff accepted",
      value: latestDiffAccepted || "none",
      detail: "",
      severity: latestDiffAccepted ? "ok" : "info",
      clearFilter: false,
      topicFilter: "",
      filterType: "all",
      filterValue: ""
    }
  ];
  return [...items, ...executorEntries].slice(0, 20);
}
