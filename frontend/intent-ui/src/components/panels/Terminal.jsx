import { For, Show, createEffect, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { UI_EVENT_TOPICS } from "../../lib/ui-intents";
import { subscribeEvent } from "../../stores/eventbus";
import { sendTerminalInput } from "../../stores/ui-actions";

const TARGET_KEY = "intent-ui-terminal-target-v1";
const DEFAULT_TARGET = "http://127.0.0.1:8081";
const MAX_QUEUE = 8;
const COMMAND_SOURCE = "intent-ui-terminal";
const ACK_TIMEOUT_MS = 1200;

function randomSessionId() {
  if (typeof window !== "undefined" && window.crypto?.randomUUID) {
    return window.crypto.randomUUID();
  }
  return `intent-${Date.now()}-${Math.floor(Math.random() * 1e6)}`;
}

function withHttpScheme(value) {
  const raw = String(value || "").trim();
  if (!raw) return "";
  return /^[a-zA-Z][a-zA-Z\d+\-.]*:\/\//.test(raw) ? raw : `http://${raw}`;
}

function toTermWebUrl(value, sid) {
  const withScheme = withHttpScheme(value);
  if (!withScheme) return "";
  let url;
  try {
    url = new URL(withScheme);
  } catch {
    return "";
  }
  if (url.protocol !== "http:" && url.protocol !== "https:") return "";
  const path = url.pathname.replace(/\/+$/, "");
  if (!path || path === "/") {
    url.pathname = "/term";
  } else if (!/\/term$/i.test(path)) {
    url.pathname = `${path}/term`;
  }
  if (sid) url.searchParams.set("sid", sid);
  return url.toString();
}

function readSavedTarget() {
  if (typeof window === "undefined") return DEFAULT_TARGET;
  const saved = window.localStorage.getItem(TARGET_KEY);
  const trimmed = String(saved || "").trim();
  return trimmed || DEFAULT_TARGET;
}

function TerminalComponent() {
  let unsubscribeTerminalInput;
  let removeMessageListener;
  let iframeRef;
  let frameReadyFallbackTimer;
  const pendingAckTimers = new Map();
  const sessionId = randomSessionId();
  const initialTarget = readSavedTarget();
  const [targetInput, setTargetInput] = createSignal(initialTarget);
  const [activeTarget, setActiveTarget] = createSignal(initialTarget);
  const [frameLoaded, setFrameLoaded] = createSignal(false);
  const [iframeReady, setIframeReady] = createSignal(false);
  const [queuedCommands, setQueuedCommands] = createSignal([]);

  const termWebUrl = createMemo(() => toTermWebUrl(activeTarget(), sessionId));
  const canInjectCommands = createMemo(() => Boolean(termWebUrl()) && iframeReady());

  const clearAckTimer = (id) => {
    const timer = pendingAckTimers.get(id);
    if (!timer) return;
    window.clearTimeout(timer);
    pendingAckTimers.delete(id);
  };

  const clearFrameReadyFallback = () => {
    if (frameReadyFallbackTimer == null) return;
    window.clearTimeout(frameReadyFallbackTimer);
    frameReadyFallbackTimer = undefined;
  };

  const updateCommandStatus = (id, status, error = "") => {
    const sentAt = status === "sent" ? new Date().toISOString() : undefined;
    setQueuedCommands((prev) => prev.map((item) => {
      if (item.id !== id) return item;
      return {
        ...item,
        status,
        error,
        sentAt: sentAt || item.sentAt
      };
    }));
  };

  const postCommandToIframe = (item) => {
    if (!canInjectCommands()) return false;
    const frameWindow = iframeRef?.contentWindow;
    if (!frameWindow) return false;
    let targetOrigin = "*";
    try {
      targetOrigin = new URL(termWebUrl()).origin;
    } catch {
      targetOrigin = "*";
    }
    try {
      frameWindow.postMessage({
        source: COMMAND_SOURCE,
        type: "stdin",
        sid: sessionId,
        commandId: item.id,
        text: item.text,
        execute: item.execute
      }, targetOrigin);
      return true;
    } catch {
      return false;
    }
  };

  const flushPendingCommands = () => {
    if (!canInjectCommands()) return;
    const pendingIds = queuedCommands()
      .filter((item) => item.status === "pending" || item.status === "error")
      .map((item) => item.id);
    for (const commandId of pendingIds) {
      const command = queuedCommands().find((item) => item.id === commandId);
      if (!command) continue;
      if (!postCommandToIframe(command)) continue;
      updateCommandStatus(command.id, "posted");
      clearAckTimer(command.id);
      const timeout = window.setTimeout(() => {
        updateCommandStatus(command.id, "sent");
        clearAckTimer(command.id);
      }, ACK_TIMEOUT_MS);
      pendingAckTimers.set(command.id, timeout);
    }
  };

  const handleForwardedInput = (event) => {
    const detail = event?.payload || {};
    const text = typeof detail.text === "string" ? detail.text : "";
    if (!text.trim()) return;

    const command = {
      id: randomSessionId(),
      text: text.trim(),
      execute: Boolean(detail.execute),
      status: "pending",
      createdAt: new Date().toISOString()
    };

    setQueuedCommands((prev) => {
      const next = [command, ...prev];
      return next.slice(0, MAX_QUEUE);
    });

    queueMicrotask(() => {
      flushPendingCommands();
    });
  };

  const connectTarget = () => {
    const next = targetInput().trim();
    if (!next) return;
    clearFrameReadyFallback();
    setActiveTarget(next);
    setFrameLoaded(false);
    setIframeReady(false);
    if (typeof window !== "undefined") {
      window.localStorage.setItem(TARGET_KEY, next);
    }
  };

  const retryCommand = (commandId) => {
    const command = queuedCommands().find((item) => item.id === commandId && item.status !== "sent");
    if (!command) return;
    updateCommandStatus(command.id, "pending", "");
    flushPendingCommands();
  };

  createEffect(() => {
    termWebUrl();
    if (!iframeReady()) return;
    flushPendingCommands();
  });

  onMount(() => {
    unsubscribeTerminalInput = subscribeEvent(UI_EVENT_TOPICS.action.terminalInputSent, handleForwardedInput);
    const onWindowMessage = (event) => {
      const data = event?.data;
      if (!data || typeof data !== "object") return;
      if (data.source !== "edgerun-term-web") return;
      if (typeof data.sid === "string" && data.sid !== sessionId) return;
      if (data.type === "ready") {
        clearFrameReadyFallback();
        setIframeReady(true);
        flushPendingCommands();
        return;
      }
      if (data.type !== "stdin-ack") return;
      if (typeof data.commandId !== "string") return;
      clearAckTimer(data.commandId);
      if (data.accepted === false) {
        updateCommandStatus(data.commandId, "error", String(data.error || "Rejected by terminal"));
        return;
      }
      updateCommandStatus(data.commandId, "sent", "");
    };
    window.addEventListener("message", onWindowMessage);
    removeMessageListener = () => window.removeEventListener("message", onWindowMessage);
  });

  onCleanup(() => {
    for (const id of pendingAckTimers.keys()) {
      clearAckTimer(id);
    }
    clearFrameReadyFallback();
    if (removeMessageListener) removeMessageListener();
    if (unsubscribeTerminalInput) unsubscribeTerminalInput();
  });

  return <div class="h-full w-full bg-[#0b1118] text-neutral-200 p-3 flex flex-col gap-3">
      <div class="grid grid-cols-1 gap-2 lg:grid-cols-[1fr_auto]">
        <input
          class="h-9 rounded-md border border-neutral-700 bg-neutral-900/80 px-3 font-mono text-xs text-neutral-100 outline-none focus:border-emerald-500/60"
          value={targetInput()}
          aria-label="Intent UI terminal target"
          data-testid="intent-ui-terminal-target-input"
          placeholder="http://127.0.0.1:8081"
          onInput={(event) => setTargetInput(event.currentTarget.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") connectTarget();
          }}
        />
        <button
          type="button"
          class="h-9 rounded-md border border-emerald-500/40 bg-emerald-500/10 px-3 text-xs font-semibold text-emerald-300 hover:bg-emerald-500/20"
          data-testid="intent-ui-terminal-connect"
          onClick={connectTarget}
        >
          Connect
        </button>
      </div>

      <Show when={queuedCommands().length > 0}>
        <div class="rounded-md border border-neutral-700/80 bg-neutral-900/50 p-2">
          <p class="text-[11px] uppercase tracking-wide text-neutral-400">Forwarded Commands</p>
          <p class="mt-1 text-[11px] text-neutral-500">Commands are auto-injected when the terminal is ready and remain visible for audit/retry.</p>
          <div class="mt-2 max-h-24 overflow-auto space-y-1 font-mono text-[11px] text-neutral-300" data-testid="intent-ui-forwarded-commands">
            <For each={queuedCommands()}>{(item) => <div class="flex items-center gap-2" data-testid="intent-ui-forwarded-command">
                <span class={`inline-flex items-center rounded border px-1.5 py-0.5 text-[10px] uppercase tracking-wide ${item.status === "sent" ? "border-emerald-500/40 text-emerald-300" : item.status === "error" ? "border-red-500/40 text-red-300" : "border-amber-500/40 text-amber-300"}`}>{item.status}</span>
                <span class="min-w-0 flex-1 truncate" title={item.text}>{item.execute ? "$ " : ""}{item.text}</span>
                <Show when={item.status !== "sent"}>
                  <button
                    type="button"
                    class="rounded border border-amber-500/40 bg-amber-500/10 px-1.5 py-0.5 text-[10px] text-amber-200 hover:bg-amber-500/20"
                    onClick={() => retryCommand(item.id)}
                  >
                    Retry
                  </button>
                </Show>
              </div>}</For>
          </div>
        </div>
      </Show>

      <Show when={termWebUrl()} fallback={<div class="flex flex-1 items-center justify-center rounded-md border border-amber-500/40 bg-amber-500/5 p-4 text-center">
            <div>
              <p class="text-sm font-semibold text-amber-300">Invalid terminal target.</p>
              <p class="mt-1 font-mono text-xs text-neutral-400">Use `http(s)://host[:port]` for a term-server endpoint.</p>
            </div>
          </div>}>
        <div class="min-h-0 flex-1 overflow-hidden rounded-md border border-neutral-700 bg-black">
          <iframe
            src={termWebUrl()}
            class="h-full w-full"
            title="Intent UI terminal"
            loading="lazy"
            referrerPolicy="no-referrer"
            sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-downloads"
            data-testid="intent-ui-terminal-iframe"
            ref={iframeRef}
            onLoad={() => {
              clearFrameReadyFallback();
              setFrameLoaded(true);
              frameReadyFallbackTimer = window.setTimeout(() => {
                setIframeReady(true);
                flushPendingCommands();
                clearFrameReadyFallback();
              }, 600);
              flushPendingCommands();
            }}
          />
        </div>
      </Show>

      <div class="flex items-center justify-between text-[11px] text-neutral-500">
        <p class="truncate">Active: {termWebUrl() || "not connected"}</p>
        <span class={`rounded border px-2 py-0.5 text-[10px] uppercase tracking-wide ${iframeReady() ? "border-emerald-500/40 text-emerald-300" : frameLoaded() ? "border-amber-500/40 text-amber-300" : "border-neutral-700 text-neutral-400"}`} data-testid="intent-ui-terminal-ready-state">
          {iframeReady() ? "ready" : frameLoaded() ? "loading-shell" : "loading-frame"}
        </span>
        <p class="ml-2 shrink-0">sid: {sessionId.slice(0, 8)}</p>
      </div>
    </div>;
}

function writeToTerminal(text, execute = true) {
  if (!text) return;
  sendTerminalInput(text, execute, true);
}

export {
  TerminalComponent as default,
  writeToTerminal,
};
