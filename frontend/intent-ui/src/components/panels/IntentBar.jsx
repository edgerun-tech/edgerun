import { createSignal, createMemo, createEffect, Show, For, onMount, onCleanup } from "solid-js";
import { Motion } from "solid-motionone";
import {
  TbOutlineCommand,
  TbOutlineCode,
  TbOutlineMicrophone,
  TbOutlineTerminal,
  TbOutlineX,
  TbOutlineCloud,
  TbOutlineMail,
  TbOutlineUpload,
  TbOutlineFile,
  TbOutlineFolder,
  TbOutlineSearch,
  TbOutlineSun,
  TbOutlineCloud as CloudIcon,
  TbOutlineCloudRain,
  TbOutlineClock,
  TbOutlineFilter,
  TbOutlineLogs,
  TbOutlinePin,
  TbOutlineTrash,
  TbOutlineHistory,
  TbOutlineSparkles,
  TbOutlineKey,
  TbOutlineApps
} from "solid-icons/tb";
import { FiSettings, FiGithub, FiGlobe } from "solid-icons/fi";
import { mcpManager } from "../../lib/mcp/client";
import { llmRouter, defaultProviders } from "../../lib/llm/router";
import { intentProcessor } from "../../lib/intent/processor";
import { intentExecutor } from "../../lib/intent/executor";
import { context, addRecentCommand, addOpenWindow, getRecentCommands, removeOpenWindow } from "../../stores/context";
import { openWindow, closeWindow } from "../../stores/windows";
import { integrationStore } from "../../stores/integrations";
import {
  DEFAULT_WEATHER_COORDS,
  setRuntimeAccentIndex,
  setRuntimeWeatherCoords,
  uiRuntime
} from "../../stores/ui-runtime";
import { preferences } from "../../stores/preferences";
import { UI_EVENT_TOPICS } from "../../lib/ui-intents";
import { subscribeEvent } from "../../stores/eventbus";
import { navigateBrowser, ringCall, sendTerminalInput } from "../../stores/ui-actions";
import {
  getAllResults,
  getPinnedResults,
  addResult,
  removeResult,
  pinResult,
  clearResults
} from "../../lib/stores/results";
import { ResultRenderer } from "../results/ResultRenderer";
import { openCodexResponse, openWorkflowDemo, openWorkflowIntegrations, setAssistantProvider, workflowUi } from "../../stores/workflow-ui";
import { Kbd, KbdGroup } from "../../registry/ui/kbd";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
function cn(...classes) {
  return twMerge(clsx(classes));
}
const WEATHER_SNAPSHOT_KEY = "intent-ui-weather-snapshot-v1";
const INTENTBAR_PIN_KEY = "intent-ui-intentbar-pinned-v1";
const DEFAULT_WEATHER_STATE = {
  temp: null,
  condition: "",
  humidity: null,
  windSpeed: null,
  location: "",
  feelsLike: null,
  forecast: []
};
const loadWeatherSnapshot = () => {
  if (typeof window === "undefined") return DEFAULT_WEATHER_STATE;
  try {
    const raw = localStorage.getItem(WEATHER_SNAPSHOT_KEY);
    if (!raw) return DEFAULT_WEATHER_STATE;
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return DEFAULT_WEATHER_STATE;
    return {
      ...DEFAULT_WEATHER_STATE,
      ...parsed,
      forecast: Array.isArray(parsed.forecast) && parsed.forecast.length > 0 ? parsed.forecast : DEFAULT_WEATHER_STATE.forecast
    };
  } catch {
    return DEFAULT_WEATHER_STATE;
  }
};
const persistWeatherSnapshot = (next) => {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(WEATHER_SNAPSHOT_KEY, JSON.stringify(next));
  } catch {
    // ignore storage failures
  }
};
const loadIntentBarPinned = () => {
  if (typeof window === "undefined") return false;
  try {
    return localStorage.getItem(INTENTBAR_PIN_KEY) === "1";
  } catch {
    return false;
  }
};
const persistIntentBarPinned = (value) => {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(INTENTBAR_PIN_KEY, value ? "1" : "0");
  } catch {
    // ignore storage failures
  }
};
const [query, setQuery] = createSignal("");
const [plan, setPlan] = createSignal(null);
const [loading, setLoading] = createSignal(false);
const [executing, setExecuting] = createSignal(false);
const [error, setError] = createSignal(null);
const [mode, setMode] = createSignal("intent");
const [listening, setListening] = createSignal(false);
const [partyMode, setPartyMode] = createSignal(false);
const [uploadedFiles, setUploadedFiles] = createSignal([]);
const [filterResults, setFilterResults] = createSignal([]);
const [isFiltering, setIsFiltering] = createSignal(false);
const [showHistory, setShowHistory] = createSignal(false);
const [showCommandExplorer, setShowCommandExplorer] = createSignal(false);
const [results, setResults] = createSignal(getAllResults());
const [pinnedResults, setPinnedResults] = createSignal(getPinnedResults());
const [demoGenerating, setDemoGenerating] = createSignal(false);
const [historyIndex, setHistoryIndex] = createSignal(-1);
const [historyDraft, setHistoryDraft] = createSignal("");
const [currentTime, setCurrentTime] = createSignal(/* @__PURE__ */ new Date());
const [peekOpen, setPeekOpen] = createSignal(false);
const [hotkeyExpanded, setHotkeyExpanded] = createSignal(false);
const [hotkeyCollapsed, setHotkeyCollapsed] = createSignal(false);
const [showCalendar, setShowCalendar] = createSignal(false);
const [calendarMonth, setCalendarMonth] = createSignal((() => {
  const now = new Date();
  return new Date(now.getFullYear(), now.getMonth(), 1);
})());
const [selectedCalendarDate, setSelectedCalendarDate] = createSignal(/* @__PURE__ */ new Date());
const [weather, setWeather] = createSignal(DEFAULT_WEATHER_STATE);
const [responseLines, setResponseLines] = createSignal([]);
const [responseNowTick, setResponseNowTick] = createSignal(Date.now());
const [responsePaused, setResponsePaused] = createSignal(false);
const [nyanFlight, setNyanFlight] = createSignal(null);
const [intentBarPinned, setIntentBarPinned] = createSignal(false);
const WEATHER_REFRESH_MS = 10 * 60 * 1000;
const RESPONSE_LINE_TTL_MS = 60 * 1000;
const RESPONSE_MAX_LINES = 24;
const NYAN_BASE_INTERVAL_MS = 30 * 60 * 1000;
const NYAN_MIN_INTERVAL_MS = 20 * 60 * 1000;
const NYAN_JITTER_MS = 5 * 60 * 1000;
const NYAN_FLIGHT_DURATION_MS = 14000;
let recognition = null;
let fileInputRef;
let debounceTimer;
const filterPresets = [
  { id: "files", label: "Files", icon: TbOutlineFile, color: "text-blue-400" },
  { id: "email", label: "Email", icon: TbOutlineMail, color: "text-purple-400" },
  { id: "logs", label: "Logs", icon: TbOutlineLogs, color: "text-green-400" },
  { id: "cloud", label: "Cloud", icon: TbOutlineCloud, color: "text-orange-400" }
];
const demoCommandCatalog = [
  "demo investigate api latency spike",
  "demo summarize terminal failures and suggest fixes",
  "demo plan a release and show rollout windows",
  "demo review cloud costs and top offenders",
  "demo code edit workflow with streaming diff"
];
const mediaCommandCatalog = [
  "youtube music",
  "play music",
  "pause music",
  "next track",
  "previous track"
];
const googleCommandCatalog = [
  "connect google",
  "open email",
  "gmail",
  "google email",
  "google messages",
  "google events",
  "google contacts",
  "google drive"
];
const systemCommandCatalog = [
  "/help",
  "/guide",
  "set provider codex",
  "set provider qwen",
  "open credentials",
  "open onvif",
  "onvif",
  "credentials",
  "party mode on",
  "party mode off"
];
const helpCommandCatalog = [
  {
    section: "Core",
    items: [
      { command: "/help", description: "Open command explorer" },
      { command: "/guide", description: "Open interactive onboarding guide" },
      { command: "set provider codex", description: "Switch assistant provider" },
      { command: "set provider qwen", description: "Switch assistant provider" },
      { command: "set accent blue", description: "Change accent theme" },
      { command: "open credentials", description: "Open credentials vault panel" },
      { command: "open onvif", description: "Open ONVIF cameras panel" },
      { command: "party mode on", description: "Enable mic + RGB glow" },
      { command: "party mode off", description: "Disable party mode" }
    ]
  },
  {
    section: "Integrations",
    items: [
      { command: "connect github", description: "Open GitHub integration" },
      { command: "connect google", description: "Open Google integration" },
      { command: "open email", description: "Open Gmail client panel" },
      { command: "gmail", description: "Open Gmail client panel" },
      { command: "google drive", description: "Open Drive panel" },
      { command: "google messages", description: "Fetch Gmail messages" },
      { command: "google events", description: "Fetch Calendar events" },
      { command: "google contacts", description: "Fetch Contacts list" }
    ]
  },
  {
    section: "Media",
    items: [
      { command: "youtube music", description: "Open YouTube Music" },
      { command: "play music", description: "Play current media session" },
      { command: "pause music", description: "Pause current media session" },
      { command: "next track", description: "Skip to next track" },
      { command: "music status", description: "Read now-playing status" }
    ]
  },
  {
    section: "Other",
    items: [
      { command: "demo investigate api latency spike", description: "Generate demo result set" },
      { command: "@alex message hello there", description: "Run contact action demo" },
      { command: "$ ls -la", description: "Run command in terminal panel" },
      { command: "browser example.com", description: "Open URL in browser panel" }
    ]
  }
];
const contactNameCatalog = ["alex", "maya", "jordan", "sam", "taylor"];
const contactActionCatalog = ["message", "call", "video"];
const accentOptions = [
  { id: "blue", label: "Blue", swatch: "bg-blue-500", primary: "217 91% 60%", ring: "217 91% 60%" },
  { id: "emerald", label: "Emerald", swatch: "bg-emerald-500", primary: "160 84% 39%", ring: "160 84% 39%" },
  { id: "amber", label: "Amber", swatch: "bg-amber-500", primary: "38 92% 50%", ring: "38 92% 50%" },
  { id: "rose", label: "Rose", swatch: "bg-rose-500", primary: "346 77% 49%", ring: "346 77% 49%" },
  { id: "violet", label: "Violet", swatch: "bg-violet-500", primary: "262 83% 58%", ring: "262 83% 58%" }
];
function IntentBar() {
  let inputRef;
  let responseScrollRef;
  let timeInterval;
  let responseLineTimer;
  let peekCloseTimer;
  let weatherTimer;
  let nyanTimer;
  let nyanFlightTimer;
  let clockButtonRef;
  let calendarPopoverRef;
  let handleIntentBarToggle;
  let handleCalendarOutside;
  let unsubscribeIntentbarToggle;
  let mcpMessageHandler;
  let weatherUpdating = false;
  let weatherAborted = false;
  let lastGoodWeather = null;
  let latestAssistantSnapshot = "";
  let pendingAssistantChunk = "";
  let activeStreamingLineId = "";
  let pausedAssistantSnapshot = "";
  const clearNyanTimers = () => {
    if (nyanTimer) {
      clearTimeout(nyanTimer);
      nyanTimer = null;
    }
    if (nyanFlightTimer) {
      clearTimeout(nyanFlightTimer);
      nyanFlightTimer = null;
    }
  };
  const launchNyanFlight = () => {
    if (!partyMode()) return;
    const top = Math.round(window.innerHeight * (0.04 + Math.random() * 0.22));
    setNyanFlight({
      id: `nyan-${Date.now()}-${Math.random().toString(16).slice(2, 7)}`,
      top
    });
    if (nyanFlightTimer) clearTimeout(nyanFlightTimer);
    nyanFlightTimer = setTimeout(() => {
      setNyanFlight(null);
      nyanFlightTimer = null;
    }, NYAN_FLIGHT_DURATION_MS + 200);
  };
  const scheduleNextNyanFlight = (delayMs = NYAN_BASE_INTERVAL_MS) => {
    if (nyanTimer) clearTimeout(nyanTimer);
    nyanTimer = setTimeout(() => {
      launchNyanFlight();
      const jitter = Math.round((Math.random() * 2 - 1) * NYAN_JITTER_MS);
      const nextDelay = Math.max(NYAN_MIN_INTERVAL_MS, NYAN_BASE_INTERVAL_MS + jitter);
      scheduleNextNyanFlight(nextDelay);
    }, Math.max(15000, delayMs));
  };
  const accentActiveButtonStyle = {
    color: "hsl(var(--primary))",
    "font-weight": "600"
  };
  const accentHoverButtonClass = "hover:text-[hsl(var(--primary))] hover:bg-[hsl(var(--primary))]/12";
  const accentIndex = createMemo(() => uiRuntime().accentIndex);
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const calendarMonthLabel = createMemo(() =>
    calendarMonth().toLocaleDateString("en-US", { month: "long", year: "numeric" })
  );
  const calendarDays = createMemo(() => {
    const monthStart = calendarMonth();
    const year = monthStart.getFullYear();
    const month = monthStart.getMonth();
    const firstWeekday = new Date(year, month, 1).getDay();
    const daysInMonth = new Date(year, month + 1, 0).getDate();
    const cells = [];
    for (let index = 0; index < firstWeekday; index += 1) {
      cells.push({ kind: "pad", key: `pad-${index}` });
    }
    for (let day = 1; day <= daysInMonth; day += 1) {
      const date = new Date(year, month, day);
      const today = new Date();
      const isToday = date.toDateString() === today.toDateString();
      const isSelected = date.toDateString() === selectedCalendarDate().toDateString();
      cells.push({
        kind: "day",
        key: `day-${day}`,
        day,
        date,
        isToday,
        isSelected
      });
    }
    return cells;
  });
  const parseAccentCommand = (raw) => {
    const trimmed = raw.trim().toLowerCase();
    if (!trimmed.startsWith("set accent")) return null;
    const color = trimmed.replace(/^set\s+accent\s*/, "").trim();
    const alias = color === "pink" ? "rose" : color;
    return { color: alias };
  };
  const parseProviderCommand = (raw) => {
    const trimmed = raw.trim().toLowerCase();
    if (!trimmed.startsWith("set provider")) return null;
    const value = trimmed.replace(/^set\s+provider\s*/, "").trim();
    if (!value) return { provider: "" };
    if (value === "codex" || value === "qwen") return { provider: value };
    return { provider: "invalid" };
  };
  const parseMediaCommand = (raw) => {
    const trimmed = raw.trim().toLowerCase();
    if (!trimmed) return null;
    if (trimmed === "youtube music" || trimmed === "ytm" || /^(open|launch)\s+(youtube music|ytm)$/.test(trimmed)) {
      return { action: "open", label: "open YouTube Music" };
    }
    if (/^(play|resume)\s+(youtube music|ytm|music)$/.test(trimmed)) {
      return { action: "play", label: "play music" };
    }
    if (/^(pause)\s+(youtube music|ytm|music)$/.test(trimmed)) {
      return { action: "pause", label: "pause music" };
    }
    if (/^(play\s*pause|toggle)\s+(youtube music|ytm|music)$/.test(trimmed)) {
      return { action: "toggle", label: "toggle music" };
    }
    if (/^(next)\s+(track|song|music)$/.test(trimmed) || /^(next)\s+(on\s+)?(youtube music|ytm)$/.test(trimmed)) {
      return { action: "next", label: "next track" };
    }
    if (/^(prev|previous)\s+(track|song|music)$/.test(trimmed) || /^(prev|previous)\s+(on\s+)?(youtube music|ytm)$/.test(trimmed)) {
      return { action: "previous", label: "previous track" };
    }
    if (/^(music\s+status|now\s+playing)$/.test(trimmed)) {
      return { action: "status", label: "music status" };
    }
    return null;
  };
  const applyAccent = (index) => {
    const safeIndex = setRuntimeAccentIndex(index, accentOptions.length);
    const next = accentOptions[safeIndex] ?? accentOptions[0];
    if (typeof document === "undefined") return;
    document.documentElement.style.setProperty("--primary", next.primary);
    document.documentElement.style.setProperty("--ring", next.ring);
    document.documentElement.style.setProperty("--accent", next.primary);
    document.documentElement.style.setProperty("--accent-foreground", "0 0% 98%");
  };
  const cycleAccent = (direction = 1) => {
    const next = (accentIndex() + direction + accentOptions.length) % accentOptions.length;
    applyAccent(next);
    setQuery(`set accent ${accentOptions[next].id}`);
    return next;
  };
  const parseContactCommand = (raw) => {
    const trimmed = raw.trim();
    if (!trimmed.startsWith("@")) return null;
    const body = trimmed.slice(1).trim();
    if (!body) return { contact: "", action: "", payload: "" };
    const parts = body.split(/\s+/);
    const actionIndex = parts.findIndex((part) => contactActionCatalog.includes(part.toLowerCase()));
    if (actionIndex === -1) {
      return {
        contact: body,
        action: "",
        payload: ""
      };
    }
    return {
      contact: parts.slice(0, actionIndex).join(" ").trim(),
      action: parts[actionIndex].toLowerCase(),
      payload: parts.slice(actionIndex + 1).join(" ").trim()
    };
  };
  const hasActivity = createMemo(() => Boolean(
    query().trim() ||
    plan() ||
    error() ||
    loading() ||
    executing() ||
    listening() ||
    isFiltering() ||
    filterResults().length ||
    uploadedFiles().length ||
    showHistory() ||
    showCommandExplorer() ||
    mode() !== "intent"
  ));
  const isExpanded = createMemo(() => intentBarPinned() || hotkeyExpanded() || peekOpen() || hasActivity() && !hotkeyCollapsed());
  const commandIndex = createMemo(() => {
    const commands = new Set(systemCommandCatalog);
    for (const cmd of mediaCommandCatalog) commands.add(cmd);
    for (const cmd of googleCommandCatalog) commands.add(cmd);
    for (const cmd of demoCommandCatalog) commands.add(cmd);
    for (const option of accentOptions) commands.add(`set accent ${option.id}`);
    for (const group of helpCommandCatalog) {
      for (const item of group.items) commands.add(item.command);
    }
    for (const name of contactNameCatalog) {
      commands.add(`@${name} ${contactActionCatalog[0]} `);
    }
    return Array.from(commands);
  });
  const isSubsequence = (needle, haystack) => {
    let i = 0;
    for (let j = 0; j < haystack.length && i < needle.length; j += 1) {
      if (needle[i] === haystack[j]) i += 1;
    }
    return i === needle.length;
  };
  const scoreAutocompleteCandidate = (needle, candidate) => {
    const lower = candidate.toLowerCase();
    if (!needle || lower === needle) return -1;
    let score = 0;
    if (lower.startsWith(needle)) {
      score += 220 - Math.min(needle.length, 28);
    } else if (lower.includes(needle)) {
      score += 120 - Math.min(lower.indexOf(needle) * 4, 60);
    }
    const needleTokens = needle.split(/\s+/).filter(Boolean);
    if (needleTokens.length > 1 && needleTokens.every((token) => lower.includes(token))) {
      score += 55;
    }
    if (isSubsequence(needle, lower)) {
      score += 35;
    }
    score += Math.max(0, 22 - Math.abs(candidate.length - needle.length));
    return score;
  };
  const autocompleteSuggestions = createMemo(() => {
    const rawInput = query();
    const needle = rawInput.trim().toLowerCase();
    if (!needle) return [];
    const accentParsed = parseAccentCommand(rawInput);
    if (accentParsed) {
      return accentOptions.map((option) => `set accent ${option.id}`).filter((cmd) => cmd !== needle).map((value) => ({
        value,
        score: scoreAutocompleteCandidate(needle, value)
      })).filter((item) => item.score > 0).sort((a, b) => b.score - a.score).slice(0, 5);
    }
    const contactParsed = parseContactCommand(rawInput);
    if (contactParsed) {
      const contactSuggestions = [];
      if (!contactParsed.contact) {
        for (const name of contactNameCatalog) {
          contactSuggestions.push({ value: `@${name} `, score: 400 });
        }
      } else if (!contactParsed.action) {
        for (const action of contactActionCatalog) {
          contactSuggestions.push({ value: `@${contactParsed.contact} ${action} `, score: 360 });
        }
      } else if (contactParsed.action === "message" && !contactParsed.payload) {
        contactSuggestions.push({ value: `@${contactParsed.contact} message hey are you free for a quick call?`, score: 340 });
      }
      return contactSuggestions;
    }
    if (mode() !== "intent") return [];
    const ranked = commandIndex().map((value) => ({
      value,
      score: scoreAutocompleteCandidate(needle, value)
    })).filter((item) => item.score > 0).sort((a, b) => b.score - a.score || a.value.length - b.value.length);
    return ranked.slice(0, 5);
  });
  const intentAutocomplete = createMemo(() => autocompleteSuggestions()[0]?.value ?? "");
  const isDemoIntentQuery = (value) => value.trim().toLowerCase().startsWith("demo ");
  const isContactIntentQuery = (value) => value.trim().startsWith("@");
  const isAccentIntentQuery = (value) => value.trim().toLowerCase().startsWith("set accent");
  const isProviderIntentQuery = (value) => value.trim().toLowerCase().startsWith("set provider");
  const executeContactIntent = async (rawQuery) => {
    const parsed = parseContactCommand(rawQuery);
    if (!parsed || !parsed.contact || !parsed.action) {
      setError("Use @name message|call|video");
      return;
    }
    if (parsed.action === "message" && !parsed.payload) {
      setError("Add message content after @name message");
      return;
    }
    const nowIso = (/* @__PURE__ */ new Date()).toISOString();
    const contactName = parsed.contact.replace(/\s+/g, " ").trim();
    if (parsed.action === "call" || parsed.action === "video") {
      openWindow("call");
      ringCall(contactName, parsed.action);
    }
    setDemoGenerating(true);
    setLoading(true);
    setShowHistory(true);
    await sleep(720);
    const responses = [
      {
        success: true,
        data: {
          contact: contactName,
          action: parsed.action,
          content: parsed.payload || `${parsed.action} request prepared`,
          status: parsed.action === "message" ? "queued" : "ringing"
        },
        ui: {
          viewType: "preview",
          title: `Contact Action · ${contactName}`,
          description: parsed.action === "message" ? `Message drafted for ${contactName}` : `Opening ${parsed.action} call with ${contactName}`,
          metadata: { source: "Contacts Demo", timestamp: nowIso },
          actions: [
            { label: "Execute", intent: rawQuery.trim(), variant: "primary", authenticated: true, authTimeoutMs: 1e4 },
            { label: "Edit Query", intent: `@${contactName} ${parsed.action}${parsed.payload ? ` ${parsed.payload}` : " "}`, variant: "secondary" }
          ]
        }
      },
      {
        success: true,
        data: [
          { timestamp: nowIso, title: "Intent parsed", type: "success", description: `Detected @${contactName} ${parsed.action}` },
          { timestamp: new Date(Date.now() + 3e3).toISOString(), title: parsed.action === "message" ? "Message ready to send" : "Waiting for recipient", type: "warning", description: parsed.action === "message" ? "Awaiting confirmation" : `${contactName} is ringing...` }
        ],
        ui: {
          viewType: "timeline",
          title: "Contact Workflow",
          description: "Conversation action orchestration",
          metadata: { source: "Contacts Demo", itemCount: 2, timestamp: nowIso }
        }
      }
    ];
    for (const response of responses) {
      addResult({ query: rawQuery.trim(), response });
      setResults(getAllResults());
      setPinnedResults(getPinnedResults());
      await sleep(260);
    }
    addRecentCommand(rawQuery.trim());
    setError(null);
    setMode("contact");
    setQuery("");
    setDemoGenerating(false);
    setLoading(false);
  };
  const executeDemoIntent = async (rawQuery) => {
    const nowIso = (/* @__PURE__ */ new Date()).toISOString();
    const trimmed = rawQuery.trim();
    const prompt = trimmed.replace(/^demo\s+/i, "").trim() || "run an ops review";
    if (/(code|diff|commit|edit)/i.test(prompt)) {
      openWorkflowDemo(prompt);
    }
    setDemoGenerating(true);
    setLoading(true);
    setShowHistory(true);
    await sleep(1e3);
    const responses = [
      {
        success: true,
        data: {
          prompt,
          confidence: "high",
          summary: `Generated a multi-view response for: ${prompt}`
        },
        ui: {
          viewType: "preview",
          title: "AI Response",
          description: "Intent parsed and translated into parallel result views.",
          metadata: { source: "Intent Demo", timestamp: nowIso },
          actions: [
            { label: "Investigate Logs", intent: "/logs error", variant: "primary", authenticated: true, authTimeoutMs: 1e4 },
            { label: "Open Terminal", intent: "shell", variant: "secondary" }
          ]
        }
      },
      {
        success: true,
        data: [
          { area: "API Gateway", status: "warning", p95: "842ms", owner: "platform" },
          { area: "DB Read Pool", status: "healthy", p95: "120ms", owner: "data" },
          { area: "Worker Queue", status: "degraded", p95: "1240ms", owner: "jobs" }
        ],
        ui: {
          viewType: "table",
          title: "Operational Snapshot",
          description: "Most relevant systems ranked by user impact.",
          metadata: { source: "Intent Demo", itemCount: 3, timestamp: nowIso }
        }
      },
      {
        success: true,
        data: [
          { timestamp: "13:02:11", level: "warn", source: "api", message: "Error budget burn rate increased to 2.3x" },
          { timestamp: "13:02:28", level: "error", source: "worker", message: "Retry queue backlog crossed threshold" },
          { timestamp: "13:02:54", level: "info", source: "autoscaler", message: "Scale-up triggered for worker pool" }
        ],
        ui: {
          viewType: "log-viewer",
          title: "Relevant Evidence",
          description: "Filtered logs related to your prompt.",
          metadata: { source: "Intent Demo", itemCount: 3, timestamp: nowIso }
        }
      },
      {
        success: true,
        data: [
          { timestamp: "2026-02-26T13:04:00.000Z", title: "Detect anomaly", type: "warning", description: "Latency and queue depth crossed thresholds." },
          { timestamp: "2026-02-26T13:06:00.000Z", title: "Proposed action", type: "deployment", description: "Roll worker capacity +20% and rebalance shards." },
          { timestamp: "2026-02-26T13:10:00.000Z", title: "Expected outcome", type: "success", description: "Projected p95 < 400ms within 6 minutes." }
        ],
        ui: {
          viewType: "timeline",
          title: "Suggested Timeline",
          description: "Action sequence generated from intent analysis.",
          metadata: { source: "Intent Demo", itemCount: 3, timestamp: nowIso }
        }
      }
    ];
    for (const response of responses) {
      addResult({ query: trimmed, response });
      setResults(getAllResults());
      setPinnedResults(getPinnedResults());
      await sleep(260);
    }
    addRecentCommand(trimmed);
    setPlan(null);
    setError(null);
    setMode("intent");
    setQuery("");
    setDemoGenerating(false);
    setLoading(false);
  };
  onMount(async () => {
    setIntentBarPinned(loadIntentBarPinned());
    setWeather(loadWeatherSnapshot());
    responseLineTimer = setInterval(() => {
      if (responsePaused()) return;
      const now = Date.now();
      setResponseNowTick(now);
      setResponseLines((prev) => prev.filter((line) => line.streaming || now - line.createdAt < RESPONSE_LINE_TTL_MS));
    }, 1000);
    applyAccent(accentIndex());
    timeInterval = setInterval(() => {
      setCurrentTime(/* @__PURE__ */ new Date());
    }, 30000);
    weatherAborted = false;
    await refreshWeather();
    const scheduleWeatherRefresh = (delayMs = WEATHER_REFRESH_MS) => {
      if (weatherAborted) return;
      if (weatherTimer) clearTimeout(weatherTimer);
      const jitter = Math.floor(Math.random() * 20000);
      weatherTimer = setTimeout(async () => {
        await refreshWeather();
        scheduleWeatherRefresh(WEATHER_REFRESH_MS);
      }, Math.max(30000, delayMs + jitter));
    };
    scheduleWeatherRefresh(WEATHER_REFRESH_MS);
    integrationStore.checkAll();
    try {
      await mcpManager.connectServer({
        id: "browser-os",
        name: "BrowserOS",
        type: "builtin",
        workerScript: "/workers/mcp/browser-os.js",
        enabled: true
      });
      const githubToken = localStorage.getItem("github_token");
      if (githubToken) {
        await mcpManager.connectServer({
          id: "github",
          name: "GitHub",
          type: "builtin",
          workerScript: "/workers/mcp/github.js",
          enabled: true
        });
      }
      const cloudflareToken = localStorage.getItem("cloudflare_token");
      if (cloudflareToken) {
        await mcpManager.connectServer({
          id: "cloudflare",
          name: "Cloudflare",
          type: "builtin",
          workerScript: "/workers/mcp/cloudflare.js",
          enabled: true
        });
      }
      const qwenToken = localStorage.getItem("qwen_token");
      if (qwenToken) {
        try {
          const tokenData = JSON.parse(qwenToken);
          if (tokenData.access_token && Date.now() < tokenData.expiry_date) {
            await mcpManager.connectServer({
              id: "qwen",
              name: "Qwen Code",
              type: "builtin",
              workerScript: "/workers/mcp/qwen.js",
              enabled: true,
              auth: {
                type: "oauth",
                oauthProvider: "qwen",
                tokenKey: "qwen_token"
              }
            });
          }
        } catch (e) {
          console.warn("[IntentBar] Qwen token parse error:", e);
        }
      }
      await mcpManager.connectServer({
        id: "terminal",
        name: "Terminal",
        type: "builtin",
        workerScript: "/workers/mcp/terminal.js",
        enabled: true
      });
      defaultProviders.forEach((p) => {
        llmRouter.addProvider(p);
      });
      setupSpeechRecognition();
      setupMCPMessageHandling();
    } catch (e) {
      console.error("Failed to initialize IntentBar:", e);
    }
    handleIntentBarToggle = () => {
      const active = document.activeElement;
      const inputIsFocused = active === inputRef;
      if (intentBarPinned()) {
        setHotkeyCollapsed(false);
        setPeekOpen(true);
        queueMicrotask(() => inputRef?.focus());
        return;
      }
      if (hotkeyExpanded() || inputIsFocused) {
        if (showHistory()) {
          setShowHistory(false);
          return;
        }
        if (showCommandExplorer()) {
          setShowCommandExplorer(false);
          return;
        }
        setHotkeyExpanded(false);
        setHotkeyCollapsed(true);
        inputRef?.blur();
        return;
      }
      setHotkeyCollapsed(false);
      setHotkeyExpanded(true);
      queueMicrotask(() => inputRef?.focus());
    };
    unsubscribeIntentbarToggle = subscribeEvent(UI_EVENT_TOPICS.action.intentBarToggled, handleIntentBarToggle);
    handleCalendarOutside = (event) => {
      if (!showCalendar()) return;
      const target = event.target;
      if (calendarPopoverRef?.contains(target) || clockButtonRef?.contains(target)) return;
      setShowCalendar(false);
    };
    window.addEventListener("pointerdown", handleCalendarOutside);
    if (typeof window !== "undefined") {
      window.__intentDebug = window.__intentDebug || {};
      window.__intentDebug.triggerNyanCat = () => {
        setPartyMode(true);
        launchNyanFlight();
      };
    }
  });
  onCleanup(() => {
    if (timeInterval) clearInterval(timeInterval);
    if (responseLineTimer) clearInterval(responseLineTimer);
    if (peekCloseTimer) clearTimeout(peekCloseTimer);
    weatherAborted = true;
    if (weatherTimer) clearTimeout(weatherTimer);
    clearNyanTimers();
    setNyanFlight(null);
    if (recognition) {
      recognition.stop();
    }
    clearTimeout(debounceTimer);
    if (unsubscribeIntentbarToggle) unsubscribeIntentbarToggle();
    if (handleCalendarOutside) {
      window.removeEventListener("pointerdown", handleCalendarOutside);
    }
    if (mcpMessageHandler) {
      window.removeEventListener("message", mcpMessageHandler);
    }
    if (typeof window !== "undefined" && window.__intentDebug?.triggerNyanCat) {
      delete window.__intentDebug.triggerNyanCat;
    }
  });
  createEffect(() => {
    const q = query();
    const trimmed = q.trim();
    const accentParsed = parseAccentCommand(q);
    if (accentParsed?.color) {
      const found = accentOptions.findIndex((option) => option.id === accentParsed.color);
      if (found >= 0) applyAccent(found);
    }
    if (trimmed.startsWith("$")) {
      setMode("shell");
    } else if (trimmed.startsWith("/")) {
      setMode("files");
    } else if (trimmed.startsWith("#")) {
      setMode("logs");
    } else if (trimmed.startsWith("@")) {
      setMode("contact");
    } else if (mode() !== "intent") {
      setMode("intent");
    }
    clearTimeout(debounceTimer);
    if (!trimmed) {
      setPlan(null);
      setError(null);
      setFilterResults([]);
      setIsFiltering(false);
      return;
    }
    const quickPanelMatch = trimmed.match(/^\/(files|logs|email|cloud)\b/i);
    if (quickPanelMatch) {
      setFilterResults([]);
      setIsFiltering(false);
      return;
    }
    const filterMatch = q.match(/^\/(files|email|logs|cloud)\s+(.+)/i);
    if (filterMatch) {
      const [, filterType, searchTerm] = filterMatch;
      setMode(filterType);
      performFilter(filterType, searchTerm);
      return;
    }
    if (trimmed.startsWith("/") && trimmed.length > 1) {
      performFilter("files", trimmed.slice(1).trim());
      return;
    }
    if (trimmed.startsWith("#") && trimmed.length > 1) {
      performFilter("logs", trimmed.slice(1).trim());
      return;
    }
    if (trimmed.startsWith("@") && trimmed.length > 1) {
      setFilterResults([]);
      setIsFiltering(false);
      return;
    }
    if (q.toLowerCase().trim() === "shell") {
      setMode("shell");
      handleShellMode();
      return;
    }
  });
  const latestAssistantMessage = createMemo(() => {
    const messages = workflowUi().messages || [];
    for (let i = messages.length - 1; i >= 0; i -= 1) {
      const message = messages[i];
      if (message?.role === "assistant" && typeof message.text === "string") {
        return message.text;
      }
    }
    return "";
  });
  const normalizeLineText = (value) => String(value || "").replace(/\s+/g, " ").trim();
  const clearStreamingResponseLine = () => {
    if (!activeStreamingLineId) return;
    const id = activeStreamingLineId;
    activeStreamingLineId = "";
    setResponseLines((prev) => prev.filter((line) => line.id !== id));
  };
  const setStreamingResponseLine = (text) => {
    const normalized = normalizeLineText(text);
    if (!normalized) {
      clearStreamingResponseLine();
      return;
    }
    const now = Date.now();
    if (!activeStreamingLineId) {
      activeStreamingLineId = `streaming-${now}`;
      setResponseLines((prev) => [
        ...prev,
        { id: activeStreamingLineId, text: normalized, createdAt: now, streaming: true }
      ].slice(-RESPONSE_MAX_LINES));
      return;
    }
    setResponseLines((prev) => prev.map((line) => line.id === activeStreamingLineId
      ? { ...line, text: normalized, createdAt: now, streaming: true }
      : line
    ));
  };
  const pushFinalResponseLine = (text) => {
    const normalized = normalizeLineText(text);
    if (!normalized) return;
    const now = Date.now();
    setResponseLines((prev) => [
      ...prev.filter((line) => !line.streaming),
      { id: `line-${now}-${Math.random().toString(16).slice(2, 6)}`, text: normalized, createdAt: now, streaming: false }
    ].slice(-RESPONSE_MAX_LINES));
    activeStreamingLineId = "";
  };
  const flushAssistantLines = (text) => {
    const normalized = String(text || "").replace(/\r/g, "");
    if (!normalized.trim()) return;
    const segments = normalized.split(/\n+/).map((segment) => segment.trim()).filter(Boolean);
    for (const segment of segments) {
      pushFinalResponseLine(segment);
    }
  };
  createEffect(() => {
    const assistantText = latestAssistantMessage();
    if (responsePaused()) {
      pausedAssistantSnapshot = assistantText || pausedAssistantSnapshot;
      return;
    }
    if (pausedAssistantSnapshot && assistantText === latestAssistantSnapshot) {
      const resumed = pausedAssistantSnapshot;
      pausedAssistantSnapshot = "";
      if (resumed && resumed !== latestAssistantSnapshot) {
        latestAssistantSnapshot = "";
      }
    }
    if (!assistantText) {
      latestAssistantSnapshot = "";
      pendingAssistantChunk = "";
      clearStreamingResponseLine();
      return;
    }
    if (assistantText === latestAssistantSnapshot) {
      if (!workflowUi().streaming && pendingAssistantChunk.trim()) {
        pushFinalResponseLine(pendingAssistantChunk);
        pendingAssistantChunk = "";
      }
      if (!workflowUi().streaming) {
        clearStreamingResponseLine();
      }
      return;
    }
    const delta = assistantText.startsWith(latestAssistantSnapshot)
      ? assistantText.slice(latestAssistantSnapshot.length)
      : (() => {
        pendingAssistantChunk = "";
        clearStreamingResponseLine();
        return assistantText;
      })();
    latestAssistantSnapshot = assistantText;
    if (!delta) return;
    const combined = `${pendingAssistantChunk}${delta}`;
    const parts = combined.split(/\n+/);
    const complete = parts.slice(0, -1).join("\n");
    pendingAssistantChunk = parts[parts.length - 1] || "";
    if (complete.trim()) {
      flushAssistantLines(complete);
    }
    if (workflowUi().streaming) {
      setStreamingResponseLine(pendingAssistantChunk);
      return;
    }
    if (pendingAssistantChunk.trim()) {
      pushFinalResponseLine(pendingAssistantChunk);
      pendingAssistantChunk = "";
    }
    clearStreamingResponseLine();
  });
  createEffect(() => {
    const lines = responseLines();
    if (!responseScrollRef || responsePaused() || lines.length === 0) return;
    queueMicrotask(() => {
      if (!responseScrollRef) return;
      responseScrollRef.scrollTop = responseScrollRef.scrollHeight;
    });
  });
  createEffect(() => {
    if (partyMode()) {
      scheduleNextNyanFlight();
      return;
    }
    clearNyanTimers();
    setNyanFlight(null);
  });
  const handleKeyDown = async (e) => {
    if (e.key === "Escape") {
      if (showCalendar()) {
        e.preventDefault();
        setShowCalendar(false);
        return;
      }
      if (showCommandExplorer()) {
        e.preventDefault();
        setShowCommandExplorer(false);
        return;
      }
      if (showHistory()) {
        e.preventDefault();
        setShowHistory(false);
        return;
      }
      if (plan() || error() || filterResults().length > 0 || isFiltering()) {
        e.preventDefault();
        setPlan(null);
        setError(null);
        setFilterResults([]);
        setIsFiltering(false);
        return;
      }
      if (query().trim() || uploadedFiles().length > 0) {
        e.preventDefault();
        setQuery("");
        setUploadedFiles([]);
        setHistoryIndex(-1);
        setHistoryDraft("");
        return;
      }
      if (isExpanded()) {
        if (intentBarPinned()) return;
        e.preventDefault();
        setHotkeyExpanded(false);
        setHotkeyCollapsed(true);
        setPeekOpen(false);
        inputRef?.blur();
      }
      return;
    }
    if (e.key === "Tab" && !e.shiftKey) {
      if (isAccentIntentQuery(query())) {
        e.preventDefault();
        cycleAccent(1);
        return;
      }
      if (isContactIntentQuery(query())) {
        const parsed = parseContactCommand(query());
        if (parsed) {
          if (!parsed.contact) {
            e.preventDefault();
            setQuery(`@${contactNameCatalog[0]} `);
            return;
          }
          if (!parsed.payload) {
            e.preventDefault();
            if (!parsed.action) {
              setQuery(`@${parsed.contact} ${contactActionCatalog[0]} `);
              return;
            }
            const currentActionIndex = contactActionCatalog.indexOf(parsed.action);
            if (currentActionIndex >= 0) {
              const nextAction = contactActionCatalog[(currentActionIndex + 1) % contactActionCatalog.length];
              setQuery(`@${parsed.contact} ${nextAction} `);
              return;
            }
          }
        }
      }
      const suggestion = intentAutocomplete();
      if (suggestion) {
        e.preventDefault();
        setQuery(suggestion);
        return;
      }
      if (isDemoIntentQuery(query())) {
        e.preventDefault();
        await executeDemoIntent(query());
        return;
      }
      if (isContactIntentQuery(query())) {
        const parsed = parseContactCommand(query());
        if (parsed?.action && (parsed.action !== "message" || parsed.payload)) {
          e.preventDefault();
          await executeContactIntent(query());
          return;
        }
      }
      return;
    }
    if (e.key === "ArrowUp") {
      const history = getRecentCommands();
      if (history.length === 0) return;
      e.preventDefault();
      if (historyIndex() === -1) {
        setHistoryDraft(query());
        setHistoryIndex(0);
        setQuery(history[0] || "");
        return;
      }
      const nextIndex = Math.min(historyIndex() + 1, history.length - 1);
      setHistoryIndex(nextIndex);
      setQuery(history[nextIndex] || "");
      return;
    }
    if (e.key === "ArrowDown") {
      const history = getRecentCommands();
      if (history.length === 0 || historyIndex() === -1) return;
      e.preventDefault();
      if (historyIndex() <= 0) {
        setHistoryIndex(-1);
        setQuery(historyDraft());
        setHistoryDraft("");
        return;
      }
      const nextIndex = historyIndex() - 1;
      setHistoryIndex(nextIndex);
      setQuery(history[nextIndex] || "");
      return;
    }
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      const q = query();
      if (q.trim()) {
        await processQuery(q);
      }
    }
  };
  const performFilter = async (type, searchTerm) => {
    setIsFiltering(true);
    setError(null);
    try {
      const mockResults = [
        { id: 1, type, name: `${searchTerm} - Result 1`, path: `/${type}/${searchTerm}-1`, modified: /* @__PURE__ */ new Date() },
        { id: 2, type, name: `${searchTerm} - Result 2`, path: `/${type}/${searchTerm}-2`, modified: /* @__PURE__ */ new Date() },
        { id: 3, type, name: `${searchTerm} - Result 3`, path: `/${type}/${searchTerm}-3`, modified: /* @__PURE__ */ new Date() }
      ];
      setFilterResults(mockResults);
    } catch (e) {
      setError("Filter operation failed");
    } finally {
      setIsFiltering(false);
    }
  };
  const processQuery = async (q) => {
    const trimmed = q.trim();
    if (/^\/help$/i.test(trimmed) || /^help$/i.test(trimmed)) {
      setShowCommandExplorer(true);
      setShowHistory(false);
      setError(null);
      addRecentCommand("/help");
      setQuery("");
      setMode("intent");
      return;
    }
    if (/^\/guide$/i.test(trimmed) || /^guide$/i.test(trimmed) || /^open\s+guide$/i.test(trimmed)) {
      openWindow("guide");
      addRecentCommand("/guide");
      setQuery("");
      setMode("intent");
      setError(null);
      return;
    }
    if (/^\/files\b/i.test(trimmed)) {
      openWindow("files");
      addRecentCommand("open files");
      setQuery("");
      setMode("intent");
      setError(null);
      return;
    }
    if (/^\/logs\b/i.test(trimmed)) {
      openWindow("terminal");
      addRecentCommand("open terminal");
      setQuery("");
      setMode("intent");
      setError(null);
      return;
    }
    if (/^\/email\b/i.test(trimmed)) {
      openWindow("email");
      addRecentCommand("open email");
      setQuery("");
      setMode("intent");
      setError(null);
      return;
    }
    if (/^\/cloud\b/i.test(trimmed)) {
      openWindow("cloud");
      addRecentCommand("open cloud");
      setQuery("");
      setMode("intent");
      setError(null);
      return;
    }
    const mediaCommand = parseMediaCommand(trimmed);
    if (mediaCommand) {
      try {
        if (mediaCommand.action === "open") {
          openWindow("browser");
          navigateBrowser("https://music.youtube.com");
          setError(null);
          addRecentCommand(trimmed);
          setQuery("");
          setMode("intent");
          return;
        }
        if (mediaCommand.action === "status") {
          const response = await fetch("/api/media/status");
          const payload = await response.json().catch(() => ({}));
          if (!response.ok || !payload?.ok) {
            throw new Error(payload?.error || "Could not read media status.");
          }
          const label = payload.title ? `${payload.artist ? `${payload.artist} - ` : ""}${payload.title}` : "No active track";
          addResult({
            query: trimmed,
            response: {
              success: true,
              data: payload,
              ui: {
                viewType: "preview",
                title: "YouTube Music Status",
                description: `${payload.state || "unknown"} · ${label}`,
                metadata: {
                  source: `Player ${payload.player || "unknown"}`,
                  timestamp: (/* @__PURE__ */ new Date()).toISOString()
                }
              }
            }
          });
          setResults(getAllResults());
          setPinnedResults(getPinnedResults());
          setError(null);
          addRecentCommand(trimmed);
          setQuery("");
          setMode("intent");
          return;
        }
        const response = await fetch("/api/media/control", {
          method: "POST",
          headers: { "content-type": "application/json; charset=utf-8" },
          body: JSON.stringify({ action: mediaCommand.action })
        });
        const payload = await response.json().catch(() => ({}));
        if (!response.ok || !payload?.ok) {
          throw new Error(payload?.error || "Media control failed.");
        }
        const label = payload.title ? `${payload.artist ? `${payload.artist} - ` : ""}${payload.title}` : "No active track";
        addResult({
          query: trimmed,
          response: {
            success: true,
            data: payload,
            ui: {
              viewType: "preview",
              title: "YouTube Music Control",
              description: `${mediaCommand.label} · ${payload.state || "unknown"} · ${label}`,
              metadata: {
                source: `Player ${payload.player || "unknown"}`,
                timestamp: (/* @__PURE__ */ new Date()).toISOString()
              }
            }
          }
        });
        setResults(getAllResults());
        setPinnedResults(getPinnedResults());
        setError(null);
      } catch (error2) {
        setError(error2 instanceof Error ? error2.message : "Media command failed.");
      }
      addRecentCommand(trimmed);
      setQuery("");
      setMode("intent");
      return;
    }
    const isDirectUrl = /^https?:\/\//i.test(trimmed);
    const isBrowserCommand = /^browser\s+/i.test(trimmed);
    const isDomainLike = /^(?:[a-z0-9-]+\.)+[a-z]{2,}(?:[/:?#].*)?$/i.test(trimmed);
    if (isDirectUrl || isBrowserCommand || isDomainLike) {
      const rawUrl = isDirectUrl || isDomainLike ? trimmed : trimmed.replace(/^browser\s+/i, "").trim();
      if (rawUrl) {
        openWindow("browser");
        navigateBrowser(rawUrl);
      }
      setQuery("");
      setMode("intent");
      return;
    }
    if (isAccentIntentQuery(q)) {
      const parsed = parseAccentCommand(q);
      const explicitIdx = parsed?.color ? accentOptions.findIndex((option) => option.id === parsed.color) : -1;
      const finalIndex = explicitIdx >= 0 ? explicitIdx : accentIndex();
      applyAccent(finalIndex);
      addRecentCommand(`set accent ${accentOptions[finalIndex].id}`);
      setQuery("");
      setMode("intent");
      return;
    }
    if (isProviderIntentQuery(q)) {
      const parsed = parseProviderCommand(q);
      if (!parsed || !parsed.provider || parsed.provider === "invalid") {
        setError("Use: set provider codex|qwen");
        return;
      }
      setAssistantProvider(parsed.provider);
      addRecentCommand(`set provider ${parsed.provider}`);
      setQuery("");
      setMode("intent");
      setError(null);
      return;
    }
    if (/^connect\s+github$/i.test(trimmed)) {
      openWorkflowIntegrations("github");
      addRecentCommand("connect github");
      setQuery("");
      setMode("intent");
      setError(null);
      return;
    }
    if (/^connect\s+google$/i.test(trimmed)) {
      openWorkflowIntegrations("google");
      addRecentCommand("connect google");
      setQuery("");
      setMode("intent");
      setError(null);
      return;
    }
    if (/^(open\s+email|gmail|google\s+email)$/i.test(trimmed)) {
      const googleToken = localStorage.getItem("google_token");
      if (!googleToken) {
        setError("Google not connected. Use 'connect google' first.");
        openWorkflowIntegrations("google");
        return;
      }
      openWindow("email");
      addRecentCommand("open email");
      setQuery("");
      setMode("intent");
      setError(null);
      return;
    }
    if (/^(open\s+credentials|credentials|vault)$/i.test(trimmed)) {
      openWindow("credentials");
      addRecentCommand("open credentials");
      setQuery("");
      setMode("intent");
      setError(null);
      return;
    }
    if (/^(open\s+onvif|onvif|open\s+cameras|cameras)$/i.test(trimmed)) {
      openWindow("onvif");
      addRecentCommand("open onvif");
      setQuery("");
      setMode("intent");
      setError(null);
      return;
    }
    if (/^party\s+mode\s+on$/i.test(trimmed) || /^party\s+on$/i.test(trimmed)) {
      setPartyMode(true);
      addRecentCommand("party mode on");
      setQuery("");
      setMode("intent");
      setError(null);
      return;
    }
    if (/^party\s+mode\s+off$/i.test(trimmed) || /^party\s+off$/i.test(trimmed)) {
      setPartyMode(false);
      addRecentCommand("party mode off");
      setQuery("");
      setMode("intent");
      setError(null);
      return;
    }
    if (/^google\s+drive$/i.test(trimmed)) {
      openWindow("drive");
      addRecentCommand("google drive");
      setQuery("");
      setMode("intent");
      setError(null);
      return;
    }
    if (/^google\s+messages$/i.test(trimmed)) {
      const googleToken = localStorage.getItem("google_token");
      if (!googleToken) {
        setError("Google not connected. Use 'connect google' first.");
        return;
      }
      try {
        const response = await fetch(`/api/google/messages?limit=10&token=${encodeURIComponent(googleToken)}`);
        const payload = await response.json().catch(() => ({}));
        if (!response.ok || payload?.ok === false) {
          throw new Error(payload?.error || "Failed to load Google messages.");
        }
        addResult({
          query: trimmed,
          response: {
            success: true,
            data: payload,
            ui: {
              viewType: "email-reader",
              title: "Google Messages",
              description: `Loaded ${(payload?.messages || []).length} messages`,
              metadata: {
                source: "Gmail API",
                timestamp: (/* @__PURE__ */ new Date()).toISOString()
              }
            }
          }
        });
        setResults(getAllResults());
        setPinnedResults(getPinnedResults());
        setError(null);
        addRecentCommand(trimmed);
        setQuery("");
        setMode("intent");
      } catch (error2) {
        setError(error2 instanceof Error ? error2.message : "Failed to load messages.");
      }
      return;
    }
    if (/^google\s+events$/i.test(trimmed)) {
      const googleToken = localStorage.getItem("google_token");
      if (!googleToken) {
        setError("Google not connected. Use 'connect google' first.");
        return;
      }
      try {
        const response = await fetch(`/api/google/events?limit=10&token=${encodeURIComponent(googleToken)}`);
        const payload = await response.json().catch(() => ({}));
        if (!response.ok || payload?.ok === false) {
          throw new Error(payload?.error || "Failed to load Google events.");
        }
        addResult({
          query: trimmed,
          response: {
            success: true,
            data: payload.items || [],
            ui: {
              viewType: "timeline",
              title: "Google Calendar Events",
              description: `Loaded ${(payload?.items || []).length} events`,
              metadata: {
                source: "Calendar API",
                itemCount: (payload?.items || []).length,
                timestamp: (/* @__PURE__ */ new Date()).toISOString()
              }
            }
          }
        });
        setResults(getAllResults());
        setPinnedResults(getPinnedResults());
        setError(null);
        addRecentCommand(trimmed);
        setQuery("");
        setMode("intent");
      } catch (error2) {
        setError(error2 instanceof Error ? error2.message : "Failed to load events.");
      }
      return;
    }
    if (/^google\s+contacts$/i.test(trimmed)) {
      const googleToken = localStorage.getItem("google_token");
      if (!googleToken) {
        setError("Google not connected. Use 'connect google' first.");
        return;
      }
      try {
        const response = await fetch(`/api/google/contacts?limit=50&token=${encodeURIComponent(googleToken)}`);
        const payload = await response.json().catch(() => ({}));
        if (!response.ok || payload?.ok === false) {
          throw new Error(payload?.error || "Failed to load Google contacts.");
        }
        addResult({
          query: trimmed,
          response: {
            success: true,
            data: payload.items || [],
            ui: {
              viewType: "table",
              title: "Google Contacts",
              description: `Loaded ${(payload?.items || []).length} contacts`,
              metadata: {
                source: "People API",
                itemCount: (payload?.items || []).length,
                timestamp: (/* @__PURE__ */ new Date()).toISOString()
              }
            }
          }
        });
        setResults(getAllResults());
        setPinnedResults(getPinnedResults());
        setError(null);
        addRecentCommand(trimmed);
        setQuery("");
        setMode("intent");
      } catch (error2) {
        setError(error2 instanceof Error ? error2.message : "Failed to load contacts.");
      }
      return;
    }
    if (isDemoIntentQuery(q)) {
      await executeDemoIntent(q);
      return;
    }
    if (isContactIntentQuery(q)) {
      await executeContactIntent(q);
      return;
    }
    if (q.trim().startsWith("$")) {
      const shellInput = q.trim().slice(1).trim();
      openWindow("terminal");
      if (shellInput) {
        sendTerminalInput(shellInput, true, true);
      }
      setQuery("");
      return;
    }
    const provider = workflowUi().provider || "codex";
    openCodexResponse(q, { provider });
    addRecentCommand(trimmed);
    setQuery("");
    setMode("intent");
    setError(null);
    setPlan(null);
    return;
    setLoading(true);
    setError(null);
    try {
      const appCtx = {
        currentRepo: context.currentRepo,
        currentBranch: context.currentBranch,
        currentHost: context.currentHost,
        currentProject: context.currentProject,
        recentFiles: context.recentFiles,
        recentCommands: context.recentCommands,
        activeIntegrations: context.activeIntegrations,
        environment: context.environment,
        openWindows: context.openWindows
      };
      const result = await intentProcessor.process(q, appCtx);
      setPlan(result);
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Processing failed";
      if (msg.includes("LLM provider") || msg.includes("No LLM")) {
        const qwenToken = localStorage.getItem("qwen_token");
        const hasQwen = qwenToken && (() => {
          try {
            const data = JSON.parse(qwenToken);
            return data.access_token && Date.now() < data.expiry_date;
          } catch {
            return false;
          }
        })();
        if (hasQwen) {
          setError("Qwen connected. Processing with Qwen...");
          try {
            const qwenProvider = {
              id: "qwen-oauth",
              name: "Qwen Code",
              type: "qwen",
              baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
              apiKey: JSON.parse(qwenToken).access_token,
              defaultModel: "qwen-plus",
              availableModels: ["qwen-plus", "qwen-turbo", "qwen-max"],
              enabled: true,
              priority: 1
            };
            llmRouter.addProvider(qwenProvider);
            const appCtx = {
              currentRepo: context.currentRepo,
              currentBranch: context.currentBranch,
              currentHost: context.currentHost,
              currentProject: context.currentProject,
              recentFiles: context.recentFiles,
              recentCommands: context.recentCommands,
              activeIntegrations: context.activeIntegrations,
              environment: context.environment,
              openWindows: context.openWindows
            };
            const result = await intentProcessor.process(q, appCtx);
            setPlan(result);
            return;
          } catch (retryError) {
            setError("Qwen request failed. Please try again.");
          }
        } else {
          setError("AI commands require an LLM. Use Qwen OAuth or configure in integrations.");
        }
        setPlan({
          id: "llm-prompt",
          intent: { raw: q, verb: "", target: "", modifiers: [], context, confidence: 0 },
          steps: [],
          risk: "low",
          preview: [],
          requiresAuth: true,
          predictedResult: "Configure LLM in integrations"
        });
      } else {
        setError(msg);
        setPlan(null);
      }
    } finally {
      setLoading(false);
    }
  };
  const handleExecute = async () => {
    const p = plan();
    if (!p) return;
    setExecuting(true);
    setError(null);
    try {
      const result = await intentExecutor.execute(p);
      if (result.success) {
        addRecentCommand(query());
        if (result.responses && result.responses.length > 0) {
          for (const response of result.responses) {
            addResult({
              query: query(),
              response
            });
          }
        } else {
          const toolResponse = {
            success: true,
            data: result,
            ui: {
              viewType: "preview",
              title: p.intent.raw,
              description: p.predictedResult,
              metadata: {
                timestamp: (/* @__PURE__ */ new Date()).toISOString(),
                source: "Intent Execution"
              }
            }
          };
          addResult({
            query: query(),
            response: toolResponse
          });
        }
        setResults(getAllResults());
        setPinnedResults(getPinnedResults());
        setQuery("");
        setPlan(null);
      } else {
        setError(result.message);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Execution failed");
    } finally {
      setExecuting(false);
    }
  };
  const handleShellMode = () => {
    openWindow("terminal");
    if (recognition && !listening()) {
      try {
        recognition.start();
        setListening(true);
      } catch {
        // ignore duplicate start attempts
      }
    }
    setQuery("");
    setMode("shell");
  };
  const setupSpeechRecognition = () => {
    const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition;
    if (!SpeechRecognition) {
      console.warn("Speech recognition not supported");
      return;
    }
    const rec = new SpeechRecognition();
    recognition = rec;
    rec.continuous = true;
    rec.interimResults = true;
    rec.onresult = (event) => {
      const results2 = event.results;
      const latest = results2[results2.length - 1];
      const transcript = latest[0].transcript;
      sendTerminalInput(transcript, false, latest.isFinal);
      if (mode() === "intent" && latest.isFinal) {
        setQuery((prev) => prev + " " + transcript);
      }
    };
    rec.onerror = (event) => {
      console.error("Speech recognition error:", event.error);
      setListening(false);
    };
    rec.onend = () => {
      setListening(false);
      if (partyMode()) {
        setTimeout(() => {
          if (!recognition || !partyMode()) return;
          try {
            recognition.start();
            setListening(true);
          } catch {
            // already starting
          }
        }, 140);
      }
    };
  };
  const setupMCPMessageHandling = () => {
    mcpMessageHandler = (event) => {
      if (event.source !== window) return;
      if (event.origin !== window.location.origin) return;
      const data = event.data;
      if (!data?.type?.startsWith("tool:")) return;
      switch (data.type) {
        case "tool:open_window":
          openWindow(data.params.windowId);
          addOpenWindow(data.params.windowId);
          break;
        case "tool:close_window":
          closeWindow(data.params.windowId);
          removeOpenWindow(data.params.windowId);
          break;
        case "tool:send_to_terminal":
          sendTerminalInput(data.params.text, data.params.execute, true);
          break;
      }
    };
    window.addEventListener("message", mcpMessageHandler);
  };
  const toggleListening = () => {
    if (!recognition) return;
    if (partyMode()) {
      setPartyMode(false);
      if (listening() && mode() !== "shell") {
        try {
          recognition.stop();
        } catch {
          // ignore stop errors
        }
      }
      return;
    }
    if (listening()) {
      try {
        recognition.stop();
      } catch {
        // ignore stop errors
      }
    } else {
      try {
        recognition.start();
        setListening(true);
      } catch {
        // ignore duplicate start attempts
      }
    }
  };
  createEffect(() => {
    if (!recognition) return;
    if (partyMode()) {
      if (!listening()) {
        try {
          recognition.start();
          setListening(true);
        } catch {
          // ignore duplicate start attempts
        }
      }
      return;
    }
    if (listening() && mode() !== "shell") {
      try {
        recognition.stop();
      } catch {
        // ignore stop errors
      }
    }
  });
  const handleFileUpload = (e) => {
    const target = e.target;
    const files = target.files;
    if (files && files.length > 0) {
      setUploadedFiles((prev) => [...prev, ...Array.from(files)]);
    }
    target.value = "";
  };
  const removeUploadedFile = (index) => {
    setUploadedFiles((prev) => prev.filter((_, i) => i !== index));
  };
  const formatTime = (date) => {
    const p = preferences();
    try {
      return new Intl.DateTimeFormat("en-US", {
        hour: "2-digit",
        minute: "2-digit",
        hour12: !p.use24HourClock,
        timeZone: p.timezone === "local" ? undefined : p.timezone
      }).format(date);
    } catch {
      return date.toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" });
    }
  };
  const formatDate = (date) => {
    const p = preferences();
    try {
      return new Intl.DateTimeFormat("en-US", {
        weekday: "short",
        month: "short",
        day: "numeric",
        timeZone: p.timezone === "local" ? undefined : p.timezone
      }).format(date);
    } catch {
      return date.toLocaleDateString("en-GB", { weekday: "short", month: "short", day: "numeric" });
    }
  };
  const resolveWeatherCoords = async () => {
    const cached = uiRuntime().weatherCoords;
    if (Number.isFinite(cached?.lat) && Number.isFinite(cached?.lon)) return cached;
    if (typeof navigator === "undefined" || !navigator.geolocation) return DEFAULT_WEATHER_COORDS;
    try {
      const position = await new Promise((resolve, reject) => {
        navigator.geolocation.getCurrentPosition(resolve, reject, {
          enableHighAccuracy: false,
          timeout: 7000,
          maximumAge: 15 * 60 * 1000
        });
      });
      const coords = {
        lat: Number(position.coords.latitude),
        lon: Number(position.coords.longitude),
        location: "Current location"
      };
      return setRuntimeWeatherCoords(coords);
    } catch {
      return DEFAULT_WEATHER_COORDS;
    }
  };
  const fetchWithTimeout = async (url, timeoutMs = 9000) => {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    try {
      return await fetch(url, { signal: controller.signal });
    } finally {
      clearTimeout(timer);
    }
  };
  const refreshWeather = async () => {
    if (weatherUpdating || weatherAborted) return;
    weatherUpdating = true;
    try {
      const coords = await resolveWeatherCoords();
      let payload = null;
      let success = false;
      for (let attempt = 0; attempt < 3 && !success; attempt += 1) {
        try {
          const response = await fetchWithTimeout(
            `/api/weather?lat=${encodeURIComponent(coords.lat)}&lon=${encodeURIComponent(coords.lon)}`,
            8500 + attempt * 1000
          );
          const parsed = await response.json().catch(() => ({}));
          if (response.ok && parsed?.ok && parsed?.weather) {
            payload = parsed;
            success = true;
            break;
          }
        } catch {
          // retry on transient failures
        }
        if (attempt < 2) {
          await sleep(400 * (attempt + 1));
        }
      }
      if (!success || !payload?.weather) {
        if (lastGoodWeather) {
          setWeather((prev) => ({ ...prev, ...lastGoodWeather }));
        }
        return;
      }
      const next = payload.weather;
      const mergedWeather = {
        temp: Number.isFinite(next.temp) ? next.temp : weather().temp,
        condition: typeof next.condition === "string" ? next.condition : weather().condition,
        humidity: Number.isFinite(next.humidity) ? next.humidity : weather().humidity,
        windSpeed: Number.isFinite(next.windSpeed) ? next.windSpeed : weather().windSpeed,
        feelsLike: Number.isFinite(next.feelsLike) ? next.feelsLike : weather().feelsLike,
        location: typeof next.location === "string" && next.location ? next.location : coords.location || weather().location,
        forecast: Array.isArray(next.forecast) && next.forecast.length > 0 ? next.forecast : weather().forecast
      };
      lastGoodWeather = mergedWeather;
      setWeather((prev) => ({
        ...prev,
        ...mergedWeather
      }));
      persistWeatherSnapshot(mergedWeather);
    } catch {
      if (lastGoodWeather) {
        setWeather((prev) => ({ ...prev, ...lastGoodWeather }));
      }
    } finally {
      weatherUpdating = false;
    }
  };
  const getWeatherIcon = (condition, size = 20) => {
    switch (condition) {
      case "sunny":
        return <TbOutlineSun size={size} class="text-yellow-400" />;
      case "cloudy":
        return <CloudIcon size={size} class="text-gray-400" />;
      case "rainy":
      case "stormy":
        return <TbOutlineCloudRain size={size} class="text-blue-400" />;
      default:
        return <TbOutlineSun size={size} class="text-yellow-400" />;
    }
  };
  const hasSpecificWeatherLocation = createMemo(() => {
    const raw = String(weather().location || "").trim();
    if (!raw) return false;
    return raw.toLowerCase() !== "current location";
  });
  const PartyBars = () => (
    <div class="intent-party-bars" aria-hidden="true">
      <span class="intent-party-bar" />
      <span class="intent-party-bar" />
      <span class="intent-party-bar" />
      <span class="intent-party-bar" />
      <span class="intent-party-bar" />
    </div>
  );
  const triggerFileUpload = () => {
    fileInputRef?.click();
  };
  const handleResultAction = async (intent, action) => {
    if (action?.label === "Execute") {
      setQuery(intent);
      await processQuery(intent);
      return;
    }
    setQuery(intent);
    inputRef?.focus();
  };
  return <Motion.div
    data-layer-zone="intentbar"
    initial={{ y: -40, opacity: 0 }}
    animate={{ y: isExpanded() ? 0 : "calc(100% - 44px)", opacity: 1 }}
    transition={{ duration: 0.4, easing: [0.33, 1, 0.68, 1] }}
    class="fixed bottom-0 left-0 right-0 z-[10003] flex flex-col items-center px-4 pb-5 pt-2"
    onMouseEnter={() => {
      if (peekCloseTimer) {
        clearTimeout(peekCloseTimer);
        peekCloseTimer = null;
      }
      setPeekOpen(true);
    }}
    onMouseLeave={() => {
      if (intentBarPinned()) return;
      if (peekCloseTimer) clearTimeout(peekCloseTimer);
      peekCloseTimer = setTimeout(() => {
        setPeekOpen(false);
        peekCloseTimer = null;
      }, 10000);
    }}
  >
      <Show when={partyMode() && nyanFlight()}>
        {(flight) => (
          <Motion.div
            key={flight().id}
            class="pointer-events-none fixed left-[-210px] z-[10035]"
            style={{ top: `${flight().top}px` }}
            initial={{ x: 0, opacity: 0.96 }}
            animate={{ x: (typeof window !== "undefined" ? window.innerWidth : 1600) + 420, opacity: 1 }}
            transition={{ duration: NYAN_FLIGHT_DURATION_MS / 1000, easing: "linear" }}
          >
            <img
              src="https://www.nyan.cat/cats/original.gif"
              alt="Nyan Cat"
              class="h-20 w-auto drop-shadow-[0_0_16px_rgba(255,255,255,0.35)]"
            />
          </Motion.div>
        )}
      </Show>
      <Show when={partyMode()}>
        <div class="intent-party-glow pointer-events-none absolute -inset-4 -z-10 rounded-3xl" />
      </Show>
      {
    /* Main Bar Container */
  }
      <div class="w-full max-w-3xl overflow-visible rounded-2xl border border-neutral-700/50 bg-[#17171d]/88 shadow-[0_22px_48px_rgba(0,0,0,0.46)] backdrop-blur-2xl transition-all duration-500">
        {
    /* Top Bar - Clock, Weather, Status */
  }
        <div
    class="flex cursor-pointer items-center justify-between border-b border-neutral-800/70 bg-gradient-to-r from-neutral-900/35 via-transparent to-neutral-900/35 px-4 py-2"
    onClick={() => setPeekOpen((v) => !v)}
  >
          {
    /* Left - Time */
  }
          <Motion.div
    class="relative flex items-center gap-3"
    initial={{ x: -20, opacity: 0 }}
    animate={{ x: 0, opacity: 1 }}
    transition={{ delay: 0.1 }}
  >
            <button
    ref={clockButtonRef}
    type="button"
    class="flex items-center gap-2 rounded-md px-1 py-0.5 text-white hover:bg-neutral-800/70"
    onClick={(event) => {
      event.stopPropagation();
      setShowCalendar((prev) => !prev);
    }}
    aria-label="Open calendar"
    aria-expanded={showCalendar()}
  >
              <TbOutlineClock size={16} class="text-neutral-400" />
              <span class="text-sm font-medium tabular-nums">{formatTime(currentTime())}</span>
              <span class="text-xs text-neutral-500">{formatDate(currentTime())}</span>
            </button>
            <Show when={showCalendar()}>
              <div
    ref={calendarPopoverRef}
    class="absolute bottom-full left-0 z-30 mb-2 w-72 rounded-xl border border-neutral-700 bg-[#121318] p-3 shadow-[0_18px_38px_rgba(0,0,0,0.45)]"
    onClick={(event) => event.stopPropagation()}
  >
                <div class="mb-2 flex items-center justify-between">
                  <button
      type="button"
      class="rounded px-2 py-1 text-xs text-neutral-300 hover:bg-neutral-800"
      onClick={() => {
        const current = calendarMonth();
        setCalendarMonth(new Date(current.getFullYear(), current.getMonth() - 1, 1));
      }}
    >
                    Prev
                  </button>
                  <p class="text-xs font-medium text-neutral-200">{calendarMonthLabel()}</p>
                  <button
      type="button"
      class="rounded px-2 py-1 text-xs text-neutral-300 hover:bg-neutral-800"
      onClick={() => {
        const current = calendarMonth();
        setCalendarMonth(new Date(current.getFullYear(), current.getMonth() + 1, 1));
      }}
    >
                    Next
                  </button>
                </div>
                <div class="mb-1 grid grid-cols-7 gap-1 text-center text-[10px] uppercase tracking-wide text-neutral-500">
                  <For each={["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"]}>
                    {(label) => <span>{label}</span>}
                  </For>
                </div>
                <div class="grid grid-cols-7 gap-1">
                  <For each={calendarDays()}>
                    {(cell) => (
                      <Show
                        when={cell.kind === "day"}
                        fallback={<span class="h-8 rounded" />}
                      >
                        <button
      type="button"
      class={cn(
        "h-8 rounded text-xs transition-colors",
        cell.isSelected ? "text-white" : cell.isToday ? "text-neutral-100" : "text-neutral-300 hover:bg-neutral-800"
      )}
      style={cell.isSelected ? accentActiveButtonStyle : void 0}
      onClick={() => setSelectedCalendarDate(cell.date)}
    >
                          {cell.day}
                        </button>
                      </Show>
                    )}
                  </For>
                </div>
              </div>
            </Show>
          </Motion.div>

          {
    /* Center - Mode / Accent Suggestions */
  }
          <Show when={isAccentIntentQuery(query()) || mode() !== "intent"}>
            <Show
    when={isAccentIntentQuery(query())}
    fallback={<Motion.div
      initial={{ scale: 0.8, opacity: 0 }}
      animate={{ scale: 1, opacity: 1 }}
      exit={{ scale: 0.8, opacity: 0 }}
      class="flex items-center gap-1.5 rounded-full border border-neutral-700 bg-neutral-800/50 px-2.5 py-1"
    >
                <Show
      when={mode() === "shell"}
      fallback={<TbOutlineFilter size={14} class="text-blue-400" />}
    >
                  <TbOutlineTerminal size={14} class="text-green-400" />
                </Show>
                <span class="text-xs capitalize text-neutral-300">{mode()}</span>
              </Motion.div>}
  >
              <Motion.div
    initial={{ scale: 0.86, opacity: 0 }}
    animate={{ scale: 1, opacity: 1 }}
    class="flex items-center gap-2 rounded-full border border-neutral-700 bg-neutral-900/70 px-2.5 py-1"
  >
                <span class="text-[11px] text-neutral-300">Accent</span>
                <div class="flex items-center gap-1">
                  <For each={accentOptions}>
                    {(option, index) => <button
    type="button"
    onClick={() => {
      applyAccent(index());
      setQuery(`set accent ${option.id}`);
    }}
    class={cn(
      "h-4 w-4 rounded-full border transition-transform",
      option.swatch,
      index() === accentIndex() ? "scale-110 border-white/80" : "border-neutral-800"
    )}
    aria-label={`Select ${option.label} accent`}
    title={option.label}
  />}
                  </For>
                </div>
                <KbdGroup>
                  <Kbd>Tab</Kbd>
                </KbdGroup>
              </Motion.div>
            </Show>
          </Show>

          {
    /* Right - Weather */
  }
          <Motion.div
    class="flex items-center gap-3"
    initial={{ x: 20, opacity: 0 }}
    animate={{ x: 0, opacity: 1 }}
    transition={{ delay: 0.1 }}
  >
            <div class="flex items-center gap-2 rounded-full border border-neutral-800/80 bg-neutral-900/45 px-2 py-0.5 text-neutral-300">
              <Show
                when={Number.isFinite(weather().temp)}
                fallback={<span class="text-sm font-medium text-neutral-500">--</span>}
              >
                <Show when={partyMode()} fallback={getWeatherIcon(weather().condition, 18)}>
                  <PartyBars />
                </Show>
                <span class="text-sm font-medium">{weather().temp}°</span>
              </Show>
              <Show when={hasSpecificWeatherLocation()}>
                <span class="text-xs text-neutral-500 hidden sm:inline">{weather().location}</span>
              </Show>
            </div>
          </Motion.div>
        </div>

        {
    /* Input Area */
  }
        <div class="p-4">
          <div class="flex items-center gap-3">
            <Show
    when={mode() === "shell"}
    fallback={<Motion.div
      animate={{ rotate: loading() ? 360 : 0 }}
      transition={{ duration: 2, repeat: loading() ? Infinity : 0, easing: "linear" }}
    >
                  <TbOutlineCommand size={20} class="text-blue-400" />
                </Motion.div>}
  >
              <TbOutlineTerminal size={20} class="text-green-400" />
            </Show>

            <form onSubmit={async (e) => {
    e.preventDefault();
    const q = query();
    if (q.trim()) {
      await processQuery(q);
    }
  }} class="relative z-0 flex-1 flex items-center gap-2">
              <input
    ref={inputRef}
    type="text"
    value={query()}
    onInput={(e) => {
      setQuery(e.currentTarget.value);
      setHistoryIndex(-1);
      setHotkeyCollapsed(false);
    }}
    onKeyDown={handleKeyDown}
    placeholder={mode() === "shell" ? "$ run shell command..." : mode() === "files" ? "/ search files..." : mode() === "contact" ? "@alex message hello there" : mode() === "email" ? "@ find contacts or email..." : mode() === "logs" ? "# search logs..." : mode() === "cloud" ? "Search cloud resources..." : "What do you want to do? (Press Enter to send)"}
    onFocus={() => {
      setPeekOpen(true);
      setHotkeyExpanded(false);
      setHotkeyCollapsed(false);
    }}
    class="flex-1 border-none bg-transparent text-base text-white outline-none placeholder:text-neutral-500"
  />
            </form>

            {
    /* File Upload */
  }
            <input
    ref={fileInputRef}
    type="file"
    multiple
    onChange={handleFileUpload}
    class="hidden"
  />
            <Motion.button
    type="button"
    onClick={triggerFileUpload}
    hover={{ scale: 1.1 }}
    press={{ scale: 0.95 }}
    class={`p-2 text-neutral-400 ${accentHoverButtonClass} rounded-lg transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900`}
    title="Upload files"
    aria-label="Upload files"
  >
              <TbOutlineUpload size={18} />
            </Motion.button>

            {
    /* Voice Input */
  }
            <Motion.button
    type="button"
    onClick={toggleListening}
    hover={{ scale: 1.1 }}
    press={{ scale: 0.95 }}
    class={cn(
      "p-2 rounded-lg transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900",
      listening() ? "text-red-400 bg-red-900/20 animate-pulse" : `text-neutral-400 ${accentHoverButtonClass}`
    )}
    title="Voice input"
    aria-label={listening() ? "Stop voice input" : "Start voice input"}
    aria-pressed={listening()}
  >
              <TbOutlineMicrophone size={18} />
            </Motion.button>

            <Motion.button
    type="button"
    onClick={() => setPartyMode((prev) => !prev)}
    hover={{ scale: 1.1 }}
    press={{ scale: 0.95 }}
    class={cn(
      "p-2 rounded-lg transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900",
      partyMode() ? "" : `text-neutral-400 ${accentHoverButtonClass}`
    )}
    style={partyMode() ? accentActiveButtonStyle : void 0}
    title={partyMode() ? "Disable Party Mode" : "Enable Party Mode"}
    aria-label={partyMode() ? "Disable party mode" : "Enable party mode"}
    aria-pressed={partyMode()}
  >
              <TbOutlineSparkles size={18} />
            </Motion.button>

            <Motion.button
    type="button"
    onClick={() => {
      const next = !intentBarPinned();
      setIntentBarPinned(next);
      persistIntentBarPinned(next);
      if (next) {
        setPeekOpen(true);
        setHotkeyCollapsed(false);
      }
    }}
    hover={{ scale: 1.1 }}
    press={{ scale: 0.95 }}
    class={cn(
      "p-2 rounded-lg transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900",
      intentBarPinned() ? "" : `text-neutral-400 ${accentHoverButtonClass}`
    )}
    style={intentBarPinned() ? accentActiveButtonStyle : void 0}
    title={intentBarPinned() ? "Unpin IntentBar" : "Pin IntentBar"}
    aria-label={intentBarPinned() ? "Unpin IntentBar" : "Pin IntentBar"}
    aria-pressed={intentBarPinned()}
  >
              <TbOutlinePin size={18} />
            </Motion.button>

            <Show when={query() || uploadedFiles().length > 0}>
              <Motion.button
    type="button"
    onClick={() => {
      setQuery("");
      setUploadedFiles([]);
      setPlan(null);
      setFilterResults([]);
    }}
    initial={{ scale: 0 }}
    animate={{ scale: 1 }}
    exit={{ scale: 0 }}
    hover={{ scale: 1.1 }}
    press={{ scale: 0.95 }}
    class={`p-2 text-neutral-500 ${accentHoverButtonClass} rounded-lg transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900`}
    aria-label="Clear query"
  >
                <TbOutlineX size={18} />
              </Motion.button>
            </Show>
          </div>
          <Show when={autocompleteSuggestions().length > 0}>
            <div class="mt-2 pl-8">
              <div class="mb-1 inline-flex items-center gap-2 text-xs text-neutral-500">
                <KbdGroup>
                  <Kbd>Tab</Kbd>
                </KbdGroup>
                <span>autocomplete</span>
              </div>
              <div class="flex flex-wrap items-center gap-1.5">
                <For each={autocompleteSuggestions()}>
                  {(item, index) => <button
    type="button"
    class={cn(
      "rounded-md border px-2 py-1 text-xs transition-colors",
      index() === 0 ? "border-neutral-600 bg-neutral-800/90 text-neutral-100" : "border-neutral-800 bg-neutral-900/70 text-neutral-400 hover:text-neutral-200 hover:border-neutral-700"
    )}
    onClick={() => setQuery(item.value)}
  >
                      {item.value}
                    </button>}
                </For>
              </div>
            </div>
          </Show>

          {
    /* Uploaded Files */
  }
          <Show when={uploadedFiles().length > 0}>
            <Motion.div
    initial={{ height: 0, opacity: 0 }}
    animate={{ height: "auto", opacity: 1 }}
    exit={{ height: 0, opacity: 0 }}
    class="flex flex-wrap gap-2 mt-3 pt-3 border-t border-neutral-800"
  >
              <For each={uploadedFiles()}>
                {(file, index) => <Motion.div
    initial={{ scale: 0.8, opacity: 0 }}
    animate={{ scale: 1, opacity: 1 }}
    exit={{ scale: 0.8, opacity: 0 }}
    class="flex items-center gap-2 px-3 py-1.5 bg-neutral-800 rounded-lg text-sm text-neutral-300"
  >
                    <TbOutlineFile size={14} class="text-blue-400" />
                    <span class="truncate max-w-[150px]">{file.name}</span>
                    <button
    type="button"
    onClick={() => removeUploadedFile(index())}
    class="text-neutral-500 hover:text-red-400"
  >
                      <TbOutlineX size={14} />
                    </button>
                  </Motion.div>}
              </For>
            </Motion.div>
          </Show>
        </div>

        {
    /* Execution Plan */
  }
        <Show when={plan()}>
          <Motion.div
    initial={{ height: 0, opacity: 0 }}
    animate={{ height: "auto", opacity: 1 }}
    exit={{ height: 0, opacity: 0 }}
    transition={{ duration: 0.3 }}
    class="px-4 pb-3 border-b border-neutral-800"
  >
            <div class="flex items-center gap-2 mb-3">
              <span class="text-xs text-neutral-400">Intent:</span>
              <span class="text-sm text-white">{plan()?.intent.raw}</span>
              <span class={cn(
    "text-xs px-2 py-0.5 rounded-full",
    plan().risk === "low" && "bg-green-900/30 text-green-300",
    plan().risk === "medium" && "bg-yellow-900/30 text-yellow-300",
    plan().risk === "high" && "bg-red-900/30 text-red-300"
  )}>
                {plan().risk}
              </span>
            </div>

            <Show when={plan()?.preview?.length}>
              <div class="space-y-1.5 mb-3">
                <For each={plan()?.preview}>
                  {(item) => <div class="flex justify-between text-xs">
                      <span class="text-neutral-500">{item.label}</span>
                      <span class={item.type === "danger" ? "text-red-400" : "text-neutral-300"}>
                        {item.value}
                      </span>
                    </div>}
                </For>
              </div>
            </Show>

            <div class="flex items-center justify-between">
              <span class="text-xs text-neutral-500">{plan()?.predictedResult}</span>
              <Motion.button
    type="button"
    onClick={() => plan()?.requiresAuth ? openWindow("integrations") : handleExecute()}
    disabled={executing()}
    hover={{ scale: executing() ? 1 : 1.02 }}
    press={{ scale: executing() ? 1 : 0.98 }}
    class={cn(
      "px-4 py-1.5 rounded-lg text-sm font-medium transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900",
      executing() ? "bg-neutral-700 text-neutral-400 cursor-not-allowed" : plan()?.requiresAuth ? "bg-orange-600 hover:bg-orange-500 text-white" : "bg-blue-600 hover:bg-blue-500 text-white"
    )}
    aria-label={executing() ? "Executing plan" : plan()?.requiresAuth ? "Setup required" : "Execute plan"}
  >
                {executing() ? "Executing..." : plan()?.requiresAuth ? "Setup Required" : "Run"}
              </Motion.button>
            </div>
          </Motion.div>
        </Show>

        {
    /* Filter Results */
  }
        <Show when={filterResults().length > 0}>
          <Motion.div
    initial={{ height: 0, opacity: 0 }}
    animate={{ height: "auto", opacity: 1 }}
    exit={{ height: 0, opacity: 0 }}
    class="px-4 pb-3 border-b border-neutral-800"
  >
            <div class="flex items-center gap-2 mb-2">
              <TbOutlineSearch size={14} class="text-blue-400" />
              <span class="text-xs text-neutral-400">Found {filterResults().length} results</span>
            </div>
            <div class="space-y-1 max-h-40 overflow-y-auto">
              <For each={filterResults()}>
                {(result) => <div class="flex items-center gap-2 text-sm p-2 hover:bg-neutral-800 rounded-lg cursor-pointer transition-colors">
                    <Show
    when={result.type === "files"}
    fallback={<TbOutlineFolder size={16} class="text-neutral-400" />}
  >
                      <TbOutlineFile size={16} class="text-blue-400" />
                    </Show>
                    <span class="text-neutral-300 flex-1 truncate">{result.name}</span>
                    <span class="text-xs text-neutral-500">{result.path}</span>
                  </div>}
              </For>
            </div>
          </Motion.div>
        </Show>

        {
    /* Error State */
  }
        <Show when={error()}>
          <Motion.div
    initial={{ height: 0, opacity: 0 }}
    animate={{ height: "auto", opacity: 1 }}
    exit={{ height: 0, opacity: 0 }}
    class="px-4 py-2 bg-red-900/20 border-b border-red-900/30 flex items-center justify-between"
  >
            <p class="text-xs text-red-300">{error()}</p>
            <button
    type="button"
    onClick={() => openWindow("settings")}
    class="text-xs text-red-200 hover:text-white underline cursor-pointer focus:outline-none focus:ring-2 focus:ring-red-500 rounded"
    aria-label="Open settings to fix authentication"
  >
              Setup
            </button>
          </Motion.div>
        </Show>

        {
    /* Quick Actions */
  }
        <div class="no-scrollbar flex items-center gap-1.5 overflow-x-auto border-t border-neutral-800/70 bg-neutral-900/25 px-3 py-2">
          <span class="text-[11px] text-neutral-500 whitespace-nowrap pr-1">Quick</span>

          <For each={filterPresets}>
            {(preset) => <Motion.button
    type="button"
    onClick={() => {
      setMode(preset.id);
      setQuery(`/${preset.id} `);
      inputRef?.focus();
    }}
    hover={{ scale: 1.05 }}
    press={{ scale: 0.95 }}
    class={cn(
      "flex items-center gap-1 border px-2.5 py-1 text-[11px] rounded-lg transition-colors whitespace-nowrap cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900",
      mode() === preset.id ? "" : `border-transparent text-neutral-400 ${accentHoverButtonClass}`
    )}
    style={mode() === preset.id ? accentActiveButtonStyle : void 0}
    aria-label={`Switch to ${preset.id} mode`}
  >
                <preset.icon size={14} class={preset.color} />
                {preset.label}
              </Motion.button>}
          </For>

          <div class="flex-1 min-w-2" />

          <Motion.button
    type="button"
    onClick={() => openWindow("editor")}
    hover={{ scale: 1.05 }}
    press={{ scale: 0.95 }}
    class={`flex items-center justify-center px-2.5 py-1.5 text-xs text-neutral-400 ${accentHoverButtonClass} rounded-lg transition-colors whitespace-nowrap cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900`}
    aria-label="Open Editor"
    title="Editor"
  >
            <TbOutlineCode size={14} />
          </Motion.button>

          <Motion.button
    type="button"
    onClick={() => openWindow("github")}
    hover={{ scale: 1.05 }}
    press={{ scale: 0.95 }}
    class={`flex items-center justify-center px-2.5 py-1.5 text-xs text-neutral-400 ${accentHoverButtonClass} rounded-lg transition-colors whitespace-nowrap cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900`}
    aria-label="Open GitHub"
    title="GitHub"
  >
            <FiGithub size={14} />
          </Motion.button>

          <Motion.button
    type="button"
    onClick={() => openWindow("cloudflare")}
    hover={{ scale: 1.05 }}
    press={{ scale: 0.95 }}
    class={`flex items-center justify-center px-2.5 py-1.5 text-xs text-neutral-400 ${accentHoverButtonClass} rounded-lg transition-colors whitespace-nowrap cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900`}
    aria-label="Open Cloudflare"
    title="Cloudflare"
  >
            <FiGlobe size={14} />
          </Motion.button>

          <Motion.button
    type="button"
    onClick={() => openWindow("credentials")}
    hover={{ scale: 1.05 }}
    press={{ scale: 0.95 }}
    class={`flex items-center justify-center px-2.5 py-1.5 text-xs text-neutral-400 ${accentHoverButtonClass} rounded-lg transition-colors whitespace-nowrap cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900`}
    aria-label="Open Credentials"
    title="Credentials"
  >
            <TbOutlineKey size={14} />
          </Motion.button>

          <Motion.button
    type="button"
    onClick={() => openWindow("widgets")}
    hover={{ scale: 1.05 }}
    press={{ scale: 0.95 }}
    class={`flex items-center justify-center px-2.5 py-1.5 text-xs text-neutral-400 ${accentHoverButtonClass} rounded-lg transition-colors whitespace-nowrap cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900`}
    aria-label="Open widgets"
    title="Widgets"
  >
            <TbOutlineApps size={14} />
          </Motion.button>

          <Motion.button
    type="button"
    onClick={() => openWindow("settings")}
    hover={{ scale: 1.05 }}
    press={{ scale: 0.95 }}
    class={`flex items-center justify-center px-2.5 py-1.5 text-xs text-neutral-400 ${accentHoverButtonClass} rounded-lg transition-colors whitespace-nowrap cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900`}
    aria-label="Open settings"
    title="Settings"
  >
            <FiSettings size={14} />
          </Motion.button>

          {
    /* History Toggle */
  }
          <Motion.button
    type="button"
    onClick={() => {
      setShowCommandExplorer(!showCommandExplorer());
      setShowHistory(false);
    }}
    hover={{ scale: 1.05 }}
    press={{ scale: 0.95 }}
    class={cn(
      "flex items-center gap-1 border px-2.5 py-1 text-[11px] rounded-lg transition-colors whitespace-nowrap cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900",
      showCommandExplorer() ? "" : `border-transparent text-neutral-400 ${accentHoverButtonClass}`
    )}
    style={showCommandExplorer() ? accentActiveButtonStyle : void 0}
    aria-label={showCommandExplorer() ? "Close command explorer" : "Open command explorer"}
    aria-pressed={showCommandExplorer()}
  >
            <TbOutlineCommand size={14} />
            <span class="hidden sm:inline">Help</span>
          </Motion.button>

          <Motion.button
    type="button"
    onClick={() => {
      setShowHistory(!showHistory());
      setShowCommandExplorer(false);
    }}
    hover={{ scale: 1.05 }}
    press={{ scale: 0.95 }}
    class={cn(
      "flex items-center gap-1 border px-2.5 py-1 text-[11px] rounded-lg transition-colors whitespace-nowrap cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900",
      showHistory() ? "" : `border-transparent text-neutral-400 ${accentHoverButtonClass}`
    )}
    style={showHistory() ? accentActiveButtonStyle : void 0}
    aria-label={showHistory() ? "Close history" : "Open history"}
    aria-pressed={showHistory()}
  >
            <TbOutlineHistory size={14} />
            <span class="hidden sm:inline">History</span>
            <Show when={results().length > 0}>
              <span class="ml-1 px-1.5 py-0.5 bg-white/20 rounded-full text-[10px]">
                {results().length}
              </span>
            </Show>
          </Motion.button>
        </div>

        {
    /* Command Explorer */
  }
        <Show when={showCommandExplorer()}>
          <Motion.div
    initial={{ height: 0, opacity: 0 }}
    animate={{ height: "auto", opacity: 1 }}
    exit={{ height: 0, opacity: 0 }}
    class="border-t border-neutral-800"
  >
            <div class="px-4 py-3 flex items-center justify-between">
              <span class="text-xs font-medium text-neutral-400">Command Explorer</span>
              <span class="text-xs text-neutral-500">Type `/help` anytime</span>
            </div>
            <div class="px-4 pb-4 space-y-3 max-h-[340px] overflow-y-auto">
              <For each={helpCommandCatalog}>
                {(group) => (
                  <div class="rounded-lg border border-neutral-800 bg-neutral-900/40 p-2">
                    <p class="mb-2 text-[10px] uppercase tracking-wide text-neutral-500">{group.section}</p>
                    <div class="space-y-1.5">
                      <For each={group.items}>
                        {(item) => (
                          <div class="flex items-center gap-2 rounded-md border border-neutral-800 bg-neutral-900/60 px-2 py-1.5">
                            <button
                              type="button"
                              class="min-w-0 flex-1 truncate text-left text-xs text-neutral-200 hover:text-white"
                              onClick={() => {
                                setQuery(item.command);
                                setShowCommandExplorer(false);
                                inputRef?.focus();
                              }}
                              title={item.command}
                            >
                              {item.command}
                            </button>
                            <span class="hidden text-[10px] text-neutral-500 sm:inline">{item.description}</span>
                            <button
                              type="button"
                              class="rounded bg-neutral-800 px-1.5 py-0.5 text-[10px] text-neutral-300 hover:bg-neutral-700"
                              onClick={async () => {
                                setShowCommandExplorer(false);
                                await processQuery(item.command);
                              }}
                            >
                              Run
                            </button>
                          </div>
                        )}
                      </For>
                    </div>
                  </div>
                )}
              </For>
            </div>
          </Motion.div>
        </Show>

        {
    /* Result History Panel */
  }
        <Show when={showHistory()}>
          <Motion.div
    initial={{ height: 0, opacity: 0 }}
    animate={{ height: "auto", opacity: 1 }}
    exit={{ height: 0, opacity: 0 }}
    class="border-t border-neutral-800"
  >
            <div class="px-4 py-3 flex items-center justify-between">
              <div class="flex items-center gap-3">
                <span class="text-xs font-medium text-neutral-400">Result History</span>
                <Show when={pinnedResults().length > 0}>
                  <span class="text-xs text-neutral-500">•</span>
                  <span class="text-xs text-neutral-500">{pinnedResults().length} pinned</span>
                </Show>
              </div>
              <div class="flex items-center gap-2">
                <button
    type="button"
    onClick={() => {
      clearResults();
      setResults(getAllResults());
      setPinnedResults(getPinnedResults());
    }}
    class="text-xs text-neutral-500 hover:text-red-400 transition-colors flex items-center gap-1"
  >
                  <TbOutlineTrash size={12} />
                  Clear
                </button>
              </div>
            </div>

            <div class="px-4 pb-4 space-y-3 max-h-[400px] overflow-y-auto">
              <Show when={demoGenerating()}>
                <Motion.div
    initial={{ opacity: 0, y: 6 }}
    animate={{ opacity: 1, y: 0 }}
    class="rounded-lg border border-neutral-700 bg-neutral-900/60 px-3 py-2 text-xs text-neutral-400"
  >
                  Generating response windows...
                </Motion.div>
              </Show>
              {
    /* Pinned Results First */
  }
              <For each={pinnedResults()}>
                {(result, index) => <Motion.div
    initial={{ opacity: 0, y: 10, scale: 0.985 }}
    animate={{ opacity: 1, y: 0, scale: 1 }}
    transition={{ duration: 0.26, delay: index() * 0.14, easing: [0.22, 1, 0.36, 1] }}
    class="relative group"
  >
                    <ResultRenderer
    response={result.response}
    onAction={(intent, action) => {
      handleResultAction(intent, action);
    }}
  />
                    <div class="absolute top-2 right-2 flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                      <button
    type="button"
    onClick={() => {
      pinResult(result.id, false);
      setPinnedResults(getPinnedResults());
    }}
    class="p-1 text-neutral-400 hover:text-white bg-neutral-800 rounded"
    title="Unpin"
  >
                        <TbOutlinePin size={14} />
                      </button>
                      <button
    type="button"
    onClick={() => {
      removeResult(result.id);
      setResults(getAllResults());
      setPinnedResults(getPinnedResults());
    }}
    class="p-1 text-neutral-400 hover:text-red-400 bg-neutral-800 rounded"
    title="Remove"
  >
                        <TbOutlineTrash size={14} />
                      </button>
                    </div>
                  </Motion.div>}
              </For>

              {
    /* Regular Results */
  }
              <For each={results().filter((r) => !r.isPinned)}>
                {(result, index) => <Motion.div
    initial={{ opacity: 0, y: 10, scale: 0.985 }}
    animate={{ opacity: 1, y: 0, scale: 1 }}
    transition={{ duration: 0.26, delay: pinnedResults().length * 0.14 + index() * 0.14, easing: [0.22, 1, 0.36, 1] }}
    class="relative group"
  >
                    <ResultRenderer
    response={result.response}
    onAction={(intent, action) => {
      handleResultAction(intent, action);
    }}
  />
                    <div class="absolute top-2 right-2 flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                      <button
    type="button"
    onClick={() => {
      pinResult(result.id, true);
      setPinnedResults(getPinnedResults());
    }}
    class="p-1 text-neutral-400 hover:text-blue-400 bg-neutral-800 rounded"
    title="Pin"
  >
                        <TbOutlinePin size={14} />
                      </button>
                      <button
    type="button"
    onClick={() => {
      removeResult(result.id);
      setResults(getAllResults());
      setPinnedResults(getPinnedResults());
    }}
    class="p-1 text-neutral-400 hover:text-red-400 bg-neutral-800 rounded"
    title="Remove"
  >
                        <TbOutlineTrash size={14} />
                      </button>
                    </div>
                  </Motion.div>}
              </For>

              {
    /* Empty State */
  }
              <Show when={results().length === 0}>
                <div class="text-center py-8 text-neutral-500">
                  <TbOutlineHistory size={32} class="mx-auto mb-2 opacity-50" />
                  <p class="text-sm">No results yet</p>
                  <p class="text-xs mt-1">Your query results will appear here</p>
                </div>
              </Show>
            </div>
          </Motion.div>
        </Show>
      </div>

    </Motion.div>;
}
export {
  IntentBar as default
};
