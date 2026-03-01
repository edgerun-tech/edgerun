import { createSignal } from "solid-js";
import { knownDevices } from "./devices";
import { integrationStore } from "./integrations";
import { canonicalBridgeId } from "../lib/integrations/official-bridges";

const DEFAULT_CODE = `export function sumLatency(samples) {
  if (!samples.length) return 0
  return samples.reduce((total, current) => total + current, 0) / samples.length
}`;
const APP_VERSION = "v0.10";
const BUILD_NUMBER_KEY = "intent-ui-build-number";
const SESSION_HISTORY_KEY = "intent-ui-opencode-sessions";
const SESSION_MESSAGES_KEY = "intent-ui-opencode-session-messages";
const LEGACY_SESSION_HISTORY_KEY = "intent-ui-codex-sessions";
const LEGACY_SESSION_MESSAGES_KEY = "intent-ui-codex-session-messages";
const MAX_SESSIONS = Number.POSITIVE_INFINITY;
const MAX_SESSION_MESSAGES = 200;

function normalizeSessionEntry(item, index) {
  if (!item || typeof item !== "object") return null;
  const sessionId = typeof item.sessionId === "string" ? item.sessionId.trim() : "";
  const threadId = typeof item.threadId === "string" ? item.threadId.trim() : "";
  const resolvedSessionId = sessionId || threadId;
  if (!resolvedSessionId) return null;
  const updatedAt = typeof item.updatedAt === "string" && item.updatedAt.trim()
    ? item.updatedAt
    : new Date(Date.now() - index).toISOString();
  return {
    sessionId: resolvedSessionId,
    threadId: threadId || resolvedSessionId,
    provider: typeof item.provider === "string" && item.provider.trim() ? item.provider : "opencode",
    preview: typeof item.preview === "string" && item.preview.trim() ? item.preview : "Current session",
    updatedAt
  };
}

function loadSessionHistoryFromKey(storageKey) {
  if (typeof localStorage === "undefined") return [];
  try {
    const raw = localStorage.getItem(storageKey);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .map((entry, index) => normalizeSessionEntry(entry, index))
      .filter(Boolean);
  } catch {
    return [];
  }
}

function sortSessionsByUpdatedAt(history) {
  return [...history].sort((a, b) => new Date(b.updatedAt || 0).getTime() - new Date(a.updatedAt || 0).getTime());
}

function mergeSessionHistories(...histories) {
  const merged = [];
  const seen = new Set();
  for (const history of histories) {
    for (const entry of history || []) {
      if (!entry?.sessionId) continue;
      if (seen.has(entry.sessionId)) continue;
      seen.add(entry.sessionId);
      merged.push(entry);
    }
  }
  return sortSessionsByUpdatedAt(merged);
}

function loadSessionHistory() {
  const next = mergeSessionHistories(
    loadSessionHistoryFromKey(SESSION_HISTORY_KEY),
    loadSessionHistoryFromKey(LEGACY_SESSION_HISTORY_KEY)
  );
  return Number.isFinite(MAX_SESSIONS) ? next.slice(0, MAX_SESSIONS) : next;
}

function persistSessionHistory(history) {
  if (typeof localStorage === "undefined") return;
  try {
    const normalized = sortSessionsByUpdatedAt(
      (history || [])
        .map((entry, index) => normalizeSessionEntry(entry, index))
        .filter(Boolean)
    );
    localStorage.setItem(
      SESSION_HISTORY_KEY,
      JSON.stringify(Number.isFinite(MAX_SESSIONS) ? normalized.slice(0, MAX_SESSIONS) : normalized)
    );
  } catch {
    // ignore storage failures
  }
}

function normalizeMessage(item, index) {
  if (!item || typeof item !== "object") return null;
  const role = item.role === "assistant" ? "assistant" : "user";
  const text = typeof item.text === "string" ? item.text : "";
  return {
    id: typeof item.id === "string" ? item.id : `${role}-${Date.now()}-${index}`,
    role,
    text,
    createdAt: typeof item.createdAt === "string" ? item.createdAt : new Date().toISOString()
  };
}

function loadSessionMessageMapForKey(storageKey) {
  if (typeof localStorage === "undefined") return {};
  try {
    const raw = localStorage.getItem(storageKey);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
    const next = {};
    for (const [sessionId, messages] of Object.entries(parsed)) {
      if (!sessionId || !Array.isArray(messages)) continue;
      next[sessionId] = messages
        .map((message, index) => normalizeMessage(message, index))
        .filter(Boolean)
        .slice(-MAX_SESSION_MESSAGES);
    }
    return next;
  } catch {
    return {};
  }
}

function mergeSessionMessageMaps(...maps) {
  const next = {};
  for (const messageMap of maps) {
    for (const [sessionId, messages] of Object.entries(messageMap || {})) {
      if (!sessionId || !Array.isArray(messages)) continue;
      const existing = Array.isArray(next[sessionId]) ? next[sessionId] : [];
      next[sessionId] = [...existing, ...messages]
        .map((message, index) => normalizeMessage(message, index))
        .filter(Boolean)
        .sort((a, b) => new Date(a.createdAt || 0).getTime() - new Date(b.createdAt || 0).getTime())
        .slice(-MAX_SESSION_MESSAGES);
    }
  }
  return next;
}

function loadSessionMessageMap() {
  return mergeSessionMessageMaps(
    loadSessionMessageMapForKey(SESSION_MESSAGES_KEY),
    loadSessionMessageMapForKey(LEGACY_SESSION_MESSAGES_KEY)
  );
}

function buildSessionHistoryFromMessageMap(messageMap) {
  const entries = [];
  for (const [sessionId, messages] of Object.entries(messageMap || {})) {
    if (!sessionId || !Array.isArray(messages) || messages.length === 0) continue;
    const latestMessage = messages[messages.length - 1] || null;
    const preview = getLatestAssistantText(messages)
      || (typeof latestMessage?.text === "string" ? latestMessage.text.trim() : "")
      || "Restored session";
    entries.push(normalizeSessionEntry({
      sessionId,
      threadId: sessionId,
      provider: "opencode",
      preview,
      updatedAt: latestMessage?.createdAt || new Date().toISOString()
    }, entries.length));
  }
  return entries.filter(Boolean);
}

function mergeHistoryWithMessageMap(history, messageMap) {
  const derived = buildSessionHistoryFromMessageMap(messageMap);
  return mergeSessionHistories(history, derived);
}

function persistSessionMessageMap(messageMap) {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(SESSION_MESSAGES_KEY, JSON.stringify(messageMap));
  } catch {
    // ignore storage failures
  }
}

function getNextBuildNumber() {
  if (typeof localStorage === "undefined") return 1;
  try {
    const raw = localStorage.getItem(BUILD_NUMBER_KEY);
    const current = Number.parseInt(raw || "0", 10);
    const next = Number.isFinite(current) && current > 0 ? current + 1 : 1;
    localStorage.setItem(BUILD_NUMBER_KEY, String(next));
    return next;
  } catch {
    return 1;
  }
}

function getLatestAssistantText(messages) {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (message?.role === "assistant" && typeof message.text === "string" && message.text.trim()) {
      return message.text.trim();
    }
  }
  return "";
}

function buildSessionHistoryEntry(sessionId, threadId, provider, preview) {
  return {
    sessionId,
    threadId: threadId || "",
    provider: provider || "opencode",
    preview: preview || "Current session",
    updatedAt: new Date().toISOString()
  };
}

function upsertSessionHistory(history, entry) {
  const normalized = normalizeSessionEntry(entry, 0);
  if (!normalized) return history;
  const deduped = (history || []).filter((item) => item.sessionId !== normalized.sessionId);
  const next = [normalized, ...deduped];
  const sorted = sortSessionsByUpdatedAt(next);
  return Number.isFinite(MAX_SESSIONS) ? sorted.slice(0, MAX_SESSIONS) : sorted;
}

function appendChatMessage(messages, role, text) {
  if (!text && role !== "assistant") return messages;
  return [
    ...messages,
    {
      id: `${role}-${Date.now()}-${messages.length}`,
      role,
      text: typeof text === "string" ? text : "",
      createdAt: new Date().toISOString()
    }
  ];
}

function appendAssistantChunk(messages, chunk) {
  if (!chunk) return messages;
  const next = [...messages];
  const last = next[next.length - 1];
  if (!last || last.role !== "assistant") {
    return appendChatMessage(next, "assistant", chunk);
  }
  next[next.length - 1] = {
    ...last,
    text: `${last.text || ""}${chunk}`
  };
  return next;
}

function finalizeAssistantMessage(messages, text) {
  const next = [...messages];
  const last = next[next.length - 1];
  if (!last || last.role !== "assistant") {
    return text ? appendChatMessage(next, "assistant", text) : next;
  }
  if (!text) {
    return last.text ? next : next.slice(0, -1);
  }
  next[next.length - 1] = {
    ...last,
    text
  };
  return next;
}

let sessionMessageMap = {};

function getSessionMessages(sessionId) {
  if (!sessionId) return [];
  return Array.isArray(sessionMessageMap[sessionId]) ? sessionMessageMap[sessionId].slice(-MAX_SESSION_MESSAGES) : [];
}

function persistMessagesForSession(sessionId, messages, history) {
  if (!sessionId) return;
  const normalized = messages
    .map((message, index) => normalizeMessage(message, index))
    .filter(Boolean)
    .slice(-MAX_SESSION_MESSAGES);
  sessionMessageMap = {
    ...sessionMessageMap,
    [sessionId]: normalized
  };
  const allowedSessionIds = new Set((history || []).map((item) => item?.sessionId).filter(Boolean));
  for (const id of Object.keys(sessionMessageMap)) {
    if (allowedSessionIds.size > 0 && !allowedSessionIds.has(id)) {
      delete sessionMessageMap[id];
    }
  }
  persistSessionMessageMap(sessionMessageMap);
}

const [workflowUi, setWorkflowUi] = createSignal({
  isOpen: false,
  showCodeWorkflow: false,
  visible: false,
  leftOpen: false,
  rightOpen: false,
  topOpen: false,
  topPanel: "assistant",
  leftPanel: "files",
  rightPanel: "conversations",
  streaming: false,
  streamText: "",
  responseText: "",
  statusEvents: [],
  provider: "opencode",
  appVersion: APP_VERSION,
  buildNumber: 1,
  sessionId: "",
  threadId: "",
  selectedIntegrationId: "",
  sessionHistory: [],
  messages: [],
  assistantPhase: "idle",
  assistantStepIndex: 0,
  assistantTotalSteps: 0,
  code: DEFAULT_CODE,
  commitPending: false,
  committed: false,
  prompt: ""
});

let streamTimer = null;
let commitTimer = null;
let closeTimer = null;
let assistantTimer = null;

function clearWorkflowTimers() {
  if (streamTimer) {
    clearInterval(streamTimer);
    streamTimer = null;
  }
  if (commitTimer) {
    clearTimeout(commitTimer);
    commitTimer = null;
  }
  if (closeTimer) {
    clearTimeout(closeTimer);
    closeTimer = null;
  }
  if (assistantTimer) {
    clearInterval(assistantTimer);
    assistantTimer = null;
  }
}

function openWorkflowDemo(promptText) {
  const streamChunks = [
    "Analyzing repository context...\\n",
    "Identified unstable latency averaging logic in `sumLatency`.\\n",
    "Generating safer patch with edge-case guards and precision rounding.\\n",
    "Prepared unified diff and commit proposal.\\n",
    `Ready for review: ${promptText || "code edit workflow"}`
  ];

  clearWorkflowTimers();
  setWorkflowUi((prev) => ({
    ...prev,
    isOpen: true,
    showCodeWorkflow: true,
    visible: true,
    rightOpen: true,
    leftOpen: prev.leftOpen,
    topOpen: prev.topOpen,
    streaming: true,
    streamText: "",
    responseText: "",
    messages: appendChatMessage([], "assistant", ""),
    statusEvents: [],
    assistantPhase: "idle",
    assistantStepIndex: 0,
    assistantTotalSteps: 0,
    commitPending: false,
    committed: false,
    prompt: promptText || "code edit workflow"
  }));

  let index = 0;
  streamTimer = setInterval(() => {
    const chunk = `${streamChunks[index]}\\n`;
    setWorkflowUi((prev) => ({
      ...prev,
      streamText: `${prev.streamText}${chunk}`,
      responseText: `${prev.responseText}${chunk}`,
      messages: appendAssistantChunk(prev.messages, chunk)
    }));
    index += 1;

    if (index >= streamChunks.length) {
      clearInterval(streamTimer);
      streamTimer = null;
      setWorkflowUi((prev) => ({ ...prev, streaming: false }));
    }
  }, 340);
}

function closeWorkflowDemo() {
  clearWorkflowTimers();
  setWorkflowUi((prev) => ({
    ...prev,
    showCodeWorkflow: false,
    visible: false,
    streaming: false,
    commitPending: false
  }));
  closeTimer = setTimeout(() => {
    closeTimer = null;
    setWorkflowUi((prev) => ({
      ...prev,
      isOpen: false,
      streamText: "",
      responseText: "",
      statusEvents: []
    }));
  }, 280);
}

function toggleWorkflowDrawer(side) {
  if (closeTimer) {
    clearTimeout(closeTimer);
    closeTimer = null;
  }
  setWorkflowUi((prev) => {
    const panel = typeof side === "object" ? side.panel : null;
    const sideId = typeof side === "object" ? side.side : side;
    const next = { ...prev };
    const validLeftPanel = (id) => ["launcher", "files", "cloud", "integrations", "credentials", "settings"].includes(id);
    const validRightPanel = (id) => ["conversations", "devices"].includes(id);

    if (sideId === "left") {
      const resolvedPanel = panel ?? prev.leftPanel;
      if (!validLeftPanel(resolvedPanel)) {
        next.leftOpen = false;
        return next;
      }
      if (panel) next.leftPanel = panel;
      const samePanelSelected = !panel || panel === prev.leftPanel;
      next.leftOpen = samePanelSelected ? !prev.leftOpen : true;
    }

    if (sideId === "right") {
      const legacyPanel = panel === "events" || panel === "insights" || panel === "history";
      const normalizedPanel = legacyPanel ? "conversations" : panel;
      const resolvedPanel = normalizedPanel ?? prev.rightPanel;
      if (!validRightPanel(resolvedPanel)) {
        next.rightOpen = false;
        return next;
      }
      if (normalizedPanel) next.rightPanel = normalizedPanel;
      const samePanelSelected = !normalizedPanel || normalizedPanel === prev.rightPanel;
      next.rightOpen = samePanelSelected ? !prev.rightOpen : true;
      if (resolvedPanel === "conversations" && next.rightOpen && prev.sessionHistory.length === 0 && prev.sessionId) {
        const bootEntry = buildSessionHistoryEntry(prev.sessionId, prev.threadId, prev.provider, prev.prompt);
        next.sessionHistory = [bootEntry];
        persistSessionHistory(next.sessionHistory);
      }
    }

    if (sideId === "top") {
      next.topPanel = "assistant";
      next.topOpen = !prev.topOpen;
    }

    return next;
  });
}

function setAssistantProvider() {
  const normalized = "opencode";
  setWorkflowUi((prev) => ({ ...prev, provider: normalized }));
}

function hydrateWorkflowUiFromStorage() {
  sessionMessageMap = loadSessionMessageMap();
  const history = mergeHistoryWithMessageMap(loadSessionHistory(), sessionMessageMap);
  if (history.length > 0) {
    persistSessionHistory(history);
  }
  const first = history[0] || null;
  const activeSessionId = workflowUi().sessionId || first?.sessionId || "";
  const restoredMessages = activeSessionId ? getSessionMessages(activeSessionId) : [];
  const restoredResponse = getLatestAssistantText(restoredMessages);
  const nextBuild = getNextBuildNumber();
  setWorkflowUi((prev) => ({
    ...prev,
    buildNumber: nextBuild,
    sessionHistory: prev.sessionHistory.length > 0 ? prev.sessionHistory : history,
    sessionId: prev.sessionId || first?.sessionId || "",
    threadId: prev.threadId || first?.threadId || prev.sessionId || first?.sessionId || "",
    provider: prev.provider || first?.provider || "opencode",
    prompt: prev.prompt || first?.preview || "",
    messages: prev.messages.length > 0 ? prev.messages : restoredMessages,
    streamText: prev.streamText || restoredResponse,
    responseText: prev.responseText || restoredResponse
  }));
}

function openWorkflowIntegrations(providerId = "github") {
  const normalizedProviderId = canonicalBridgeId(providerId) || providerId;
  if (closeTimer) {
    clearTimeout(closeTimer);
    closeTimer = null;
  }
  setWorkflowUi((prev) => ({
    ...prev,
    isOpen: true,
    showCodeWorkflow: false,
    visible: true,
    leftOpen: true,
    leftPanel: "integrations",
    topOpen: true,
    topPanel: "assistant",
    selectedIntegrationId: normalizedProviderId || prev.selectedIntegrationId || "github",
    statusEvents: [
      ...prev.statusEvents,
      { type: "phase", label: "integrations", detail: `Open ${normalizedProviderId || "integration"} configuration.` }
    ]
  }));
}

function openWorkflowFlipper(deviceName = "Flipper") {
  const normalizedDeviceName = String(deviceName || "Flipper").trim() || "Flipper";
  openWorkflowDemo(`flipper workflow: inspect ${normalizedDeviceName} over web bluetooth and propose next actions`);
  setWorkflowUi((prev) => ({
    ...prev,
    isOpen: true,
    visible: true,
    leftOpen: true,
    leftPanel: "integrations",
    selectedIntegrationId: "flipper",
    statusEvents: [
      ...prev.statusEvents,
      { type: "phase", label: "flipper", detail: `Workflow bootstrapped for ${normalizedDeviceName}.` }
    ]
  }));
}

function useWorkflowSession(session) {
  if (!session?.sessionId) return;
  const primaryMessages = getSessionMessages(session.sessionId);
  const sessionMessages = primaryMessages.length > 0
    ? primaryMessages
    : getSessionMessages(session.threadId || session.sessionId);
  const latestResponse = getLatestAssistantText(sessionMessages);
  setWorkflowUi((prev) => ({
    ...prev,
    provider: session.provider || prev.provider,
    sessionId: session.sessionId,
    threadId: session.threadId || "",
    prompt: session.preview || prev.prompt,
    messages: sessionMessages,
    streamText: latestResponse,
    responseText: latestResponse,
    topOpen: true,
    topPanel: "assistant"
  }));
}

function switchWorkflowSession(selector) {
  const value = String(selector || "").trim();
  if (!value) return false;
  const sessions = workflowUi().sessionHistory || [];
  const byPrefix = sessions.find((session) => String(session?.sessionId || "").startsWith(value));
  const byThreadPrefix = sessions.find((session) => String(session?.threadId || "").startsWith(value));
  const parsed = Number.parseInt(value, 10);
  const byIndex = Number.isFinite(parsed) && parsed > 0 ? sessions[Math.max(0, parsed - 1)] : null;
  const target = byPrefix || byThreadPrefix || byIndex || null;
  if (!target?.sessionId) return false;
  useWorkflowSession(target);
  return true;
}

function startNewAssistantSession() {
  setWorkflowUi((prev) => ({
    ...prev,
    sessionId: "",
    threadId: "",
    prompt: "",
    messages: [],
    responseText: "",
    streamText: "",
    statusEvents: [
      { type: "phase", label: "new", detail: "Started a new Codex session." }
    ],
    assistantPhase: "idle",
    assistantStepIndex: 0
  }));
}

function assistantIntegrationId() {
  return "opencode_cli";
}

function anyDeviceConnected() {
  return knownDevices().some((device) => Boolean(device?.online));
}

async function openAssistantResponse(queryText, options = {}) {
  const prompt = queryText?.trim() || "No prompt";
  const provider = "opencode";
  const requiredIntegrationId = assistantIntegrationId(provider);
  integrationStore.checkAll();
  const requiredIntegration = integrationStore.get(requiredIntegrationId);
  const initialEvents = [
    { type: "phase", label: "queued", detail: "Preparing request..." },
    { type: "phase", label: "thinking", detail: "Routing to assistant..." }
  ];

  if (closeTimer) {
    clearTimeout(closeTimer);
    closeTimer = null;
  }
  if (assistantTimer) {
    clearInterval(assistantTimer);
    assistantTimer = null;
  }

  const currentState = workflowUi();

  if (!anyDeviceConnected()) {
    setWorkflowUi((prev) => ({
      ...prev,
      topOpen: true,
      topPanel: "assistant",
      streaming: false,
      assistantPhase: "error",
      assistantStepIndex: 0,
      assistantTotalSteps: 0,
      prompt,
      statusEvents: [
        ...prev.statusEvents,
        { type: "error", label: "blocked", detail: "Assistant blocked: no connected device is available." }
      ],
      messages: appendChatMessage(prev.messages, "user", prompt)
    }));
    return;
  }

  if (!requiredIntegration?.available) {
    const integrationName = requiredIntegration?.name || requiredIntegrationId;
    const reason = requiredIntegration?.availabilityReason ? ` (${requiredIntegration.availabilityReason})` : "";
    setWorkflowUi((prev) => ({
      ...prev,
      topOpen: true,
      topPanel: "assistant",
      streaming: false,
      assistantPhase: "error",
      assistantStepIndex: 0,
      assistantTotalSteps: 0,
      prompt,
      selectedIntegrationId: requiredIntegrationId,
      statusEvents: [
        ...prev.statusEvents,
        { type: "error", label: "blocked", detail: `Assistant blocked: connect ${integrationName} integration first${reason}.` }
      ],
      messages: appendChatMessage(prev.messages, "user", prompt)
    }));
    return;
  }

  const draftMessages = appendChatMessage(
    appendChatMessage(currentState.messages, "user", prompt),
    "assistant",
    ""
  );

  setWorkflowUi((prev) => ({
    ...prev,
    topOpen: true,
    topPanel: "assistant",
    streaming: true,
    streamText: "",
    responseText: "",
    messages: draftMessages,
    statusEvents: initialEvents,
    assistantPhase: "thinking",
    assistantStepIndex: 0,
    assistantTotalSteps: 0,
    prompt
  }));
  try {
    const requestBody = {
      provider,
      message: prompt,
      threadId: currentState.threadId || ""
    };
    if (currentState.sessionId) {
      requestBody.sessionId = currentState.sessionId;
    }
    setWorkflowUi((prev) => ({
      ...prev,
      statusEvents: [...prev.statusEvents, { type: "phase", label: "executing", detail: `Calling ${provider}...` }],
      assistantPhase: "executing",
      assistantStepIndex: 0
    }));

    const response = await fetch("/api/assistant", {
      method: "POST",
      headers: {
        "content-type": "application/json"
      },
      body: JSON.stringify(requestBody)
    });

    if (!response.ok) {
      const payload = await response.json().catch(() => ({}));
      const errMessage = payload?.error || `Assistant request failed (${response.status})`;
      setWorkflowUi((prev) => ({
        ...prev,
        streaming: false,
        assistantPhase: "error",
        assistantStepIndex: 0,
        responseText: "",
        statusEvents: [
          ...prev.statusEvents,
          { type: "error", label: "error", detail: errMessage }
        ]
      }));
      return;
    }

    const contentType = response.headers.get("content-type") || "";
    if (!contentType.includes("application/x-ndjson") || !response.body) {
      const payload = await response.json().catch(() => ({}));
      if (!payload?.ok) {
        const errMessage = payload?.error || "Assistant request failed.";
        setWorkflowUi((prev) => ({
          ...prev,
          streaming: false,
          assistantPhase: "error",
          assistantStepIndex: 0,
          responseText: "",
          statusEvents: [
            ...prev.statusEvents,
            { type: "error", label: "error", detail: errMessage }
          ]
        }));
        return;
      }
      const responseText = String(payload.message || "").trim();
      const actionLines = Array.isArray(payload.actions) && payload.actions.length > 0
        ? `\n\nSuggested actions:\n${payload.actions.map((action, idx) => `${idx + 1}. ${action}`).join("\n")}`
        : "";
      const finalText = `${responseText || "No assistant response."}${actionLines}`;
      setWorkflowUi((prev) => {
        const payloadSessionId = typeof payload.sessionId === "string" ? payload.sessionId.trim() : "";
        const payloadThreadId = typeof payload.threadId === "string" ? payload.threadId.trim() : "";
        const nextSessionId = payloadSessionId || payloadThreadId || prev.sessionId;
        const nextThreadId = payloadThreadId || payloadSessionId || prev.threadId;
        const nextHistory = nextSessionId
          ? upsertSessionHistory(prev.sessionHistory, buildSessionHistoryEntry(nextSessionId, nextThreadId, provider, prompt))
          : prev.sessionHistory;
        const nextMessages = finalizeAssistantMessage(prev.messages, finalText);

        if (nextSessionId) {
          persistSessionHistory(nextHistory);
          persistMessagesForSession(nextSessionId, nextMessages, nextHistory);
        }

        return {
          ...prev,
          streaming: false,
          assistantPhase: "done",
          assistantStepIndex: 0,
          responseText: finalText,
          streamText: finalText,
          messages: nextMessages,
          statusEvents: [
            ...prev.statusEvents,
            ...(Array.isArray(payload.statusEvents) ? payload.statusEvents : []),
            { type: "phase", label: "done", detail: "Response ready." }
          ],
          sessionId: nextSessionId,
          threadId: nextThreadId,
          provider,
          sessionHistory: nextHistory
        };
      });
      return;
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";
    let finalEvent = null;
    let pendingDelta = "";
    let flushTimer = null;

    const flushDelta = () => {
      if (!pendingDelta) return;
      const chunk = pendingDelta;
      pendingDelta = "";
      setWorkflowUi((prev) => ({
        ...prev,
        streamText: `${prev.streamText}${chunk}`,
        responseText: `${prev.responseText}${chunk}`,
        messages: appendAssistantChunk(prev.messages, chunk)
      }));
    };

    const scheduleFlush = () => {
      if (flushTimer) return;
      flushTimer = setTimeout(() => {
        flushTimer = null;
        flushDelta();
      }, 34);
    };

    const applyStreamEvent = (event) => {
      if (!event || typeof event !== "object") return;
      if (event.type === "meta") {
        setWorkflowUi((prev) => ({
          // backend may return only one of sessionId/threadId; normalize both for resumability
          ...prev,
          provider: event.provider || prev.provider,
          sessionId: (typeof event.sessionId === "string" ? event.sessionId.trim() : "")
            || (typeof event.threadId === "string" ? event.threadId.trim() : "")
            || prev.sessionId,
          threadId: (typeof event.threadId === "string" ? event.threadId.trim() : "")
            || (typeof event.sessionId === "string" ? event.sessionId.trim() : "")
            || prev.threadId
        }));
        return;
      }
      if (event.type === "status" && event.event) {
        setWorkflowUi((prev) => ({
          ...prev,
          assistantPhase: event.event.label || prev.assistantPhase,
          statusEvents: [...prev.statusEvents, event.event]
        }));
        return;
      }
      if (event.type === "delta" && typeof event.text === "string") {
        pendingDelta += event.text;
        scheduleFlush();
        return;
      }
      if (event.type === "action" && typeof event.text === "string") {
        setWorkflowUi((prev) => ({
          ...prev,
          statusEvents: [
            ...prev.statusEvents,
            { type: "phase", label: "action", detail: event.text }
          ]
        }));
        return;
      }
      if (event.type === "done") {
        finalEvent = event;
      }
    };

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      let newlineIndex = buffer.indexOf("\n");
      while (newlineIndex >= 0) {
        const rawLine = buffer.slice(0, newlineIndex).trim();
        buffer = buffer.slice(newlineIndex + 1);
        if (rawLine) {
          try {
            applyStreamEvent(JSON.parse(rawLine));
          } catch {
            // ignore invalid event chunks
          }
        }
        newlineIndex = buffer.indexOf("\n");
      }
    }
    buffer += decoder.decode();
    const trailing = buffer.trim();
    if (trailing) {
      try {
        applyStreamEvent(JSON.parse(trailing));
      } catch {
        // ignore invalid trailing chunk
      }
    }
    if (flushTimer) {
      clearTimeout(flushTimer);
      flushTimer = null;
    }
    flushDelta();

    const actionLines = Array.isArray(finalEvent?.actions) && finalEvent.actions.length > 0
      ? `\n\nSuggested actions:\n${finalEvent.actions.map((action, idx) => `${idx + 1}. ${action}`).join("\n")}`
      : "";

    setWorkflowUi((prev) => {
      const streamedMessage = typeof finalEvent?.message === "string" && finalEvent.message.trim()
        ? finalEvent.message.trim()
        : getLatestAssistantText(prev.messages);
      const finalMessage = streamedMessage || (finalEvent?.error ? "" : "No assistant response.");
      const finalText = `${finalMessage}${actionLines}`;
      const finalSessionId = typeof finalEvent?.sessionId === "string" ? finalEvent.sessionId.trim() : "";
      const finalThreadId = typeof finalEvent?.threadId === "string" ? finalEvent.threadId.trim() : "";
      const nextSessionId = finalSessionId || finalThreadId || prev.sessionId;
      const nextThreadId = finalThreadId || finalSessionId || prev.threadId;
      const finalProvider = finalEvent?.provider || provider;
      const doneDetail = finalEvent?.error ? finalEvent.error : "Response ready.";
      const doneLabel = finalEvent?.error ? "error" : "done";
      const nextHistory = nextSessionId
        ? upsertSessionHistory(prev.sessionHistory, buildSessionHistoryEntry(nextSessionId, nextThreadId, finalProvider, prompt))
        : prev.sessionHistory;
      const nextMessages = finalizeAssistantMessage(prev.messages, finalText);

      if (nextSessionId) {
        persistSessionHistory(nextHistory);
        persistMessagesForSession(nextSessionId, nextMessages, nextHistory);
      }

      return {
        ...prev,
        streaming: false,
        assistantPhase: finalEvent?.error ? "error" : "done",
        assistantStepIndex: 0,
        streamText: finalText,
        responseText: finalText,
        messages: nextMessages,
        statusEvents: [
          ...prev.statusEvents,
          { type: finalEvent?.error ? "error" : "phase", label: doneLabel, detail: doneDetail }
        ],
        sessionId: nextSessionId,
        threadId: nextThreadId,
        provider: finalProvider,
        sessionHistory: nextHistory
      };
    });
  } catch (error) {
    const detail = error instanceof Error ? error.message : "Unknown error";
    setWorkflowUi((prev) => ({
      ...prev,
      streaming: false,
      assistantPhase: "error",
      assistantStepIndex: 0,
      responseText: "",
      statusEvents: [...prev.statusEvents, { type: "error", label: "error", detail }]
    }));
  }
}

function setWorkflowCode(code) {
  setWorkflowUi((prev) => ({ ...prev, code }));
}

function triggerWorkflowCommit() {
  const state = workflowUi();
  if (state.committed) return;

  if (!state.commitPending) {
    setWorkflowUi((prev) => ({ ...prev, commitPending: true }));
    if (commitTimer) {
      clearTimeout(commitTimer);
    }
    commitTimer = setTimeout(() => {
      commitTimer = null;
      setWorkflowUi((prev) => ({ ...prev, commitPending: false }));
    }, 10000);
    return;
  }

  if (commitTimer) {
    clearTimeout(commitTimer);
    commitTimer = null;
  }

  setWorkflowUi((prev) => ({
    ...prev,
    commitPending: false,
    committed: true
  }));
}

export {
  closeWorkflowDemo,
  hydrateWorkflowUiFromStorage,
  openWorkflowDemo,
  openAssistantResponse,
  openWorkflowIntegrations,
  openWorkflowFlipper,
  startNewAssistantSession,
  switchWorkflowSession,
  setAssistantProvider,
  setWorkflowCode,
  toggleWorkflowDrawer,
  triggerWorkflowCommit,
  useWorkflowSession,
  workflowUi
};
