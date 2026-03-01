import { For, Show, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { UI_EVENT_TOPICS } from "../../lib/ui-intents";
import { subscribeEvent } from "../../stores/eventbus";
import { sendTerminalInput } from "../../stores/ui-actions";

const TARGET_KEY = "intent-ui-terminal-target-v1";
const DEFAULT_TARGET = "http://127.0.0.1:8081";
const MAX_QUEUE = 8;

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
  const sessionId = randomSessionId();
  const initialTarget = readSavedTarget();
  const [targetInput, setTargetInput] = createSignal(initialTarget);
  const [activeTarget, setActiveTarget] = createSignal(initialTarget);
  const [queuedCommands, setQueuedCommands] = createSignal([]);

  const termWebUrl = createMemo(() => toTermWebUrl(activeTarget(), sessionId));

  const handleForwardedInput = (event) => {
    const detail = event?.payload || {};
    const text = typeof detail.text === "string" ? detail.text : "";
    if (!text.trim()) return;

    setQueuedCommands((prev) => {
      const next = [{
        id: randomSessionId(),
        text: text.trim(),
        execute: Boolean(detail.execute)
      }, ...prev];
      return next.slice(0, MAX_QUEUE);
    });
  };

  const connectTarget = () => {
    const next = targetInput().trim();
    if (!next) return;
    setActiveTarget(next);
    if (typeof window !== "undefined") {
      window.localStorage.setItem(TARGET_KEY, next);
    }
  };

  onMount(() => {
    unsubscribeTerminalInput = subscribeEvent(UI_EVENT_TOPICS.action.terminalInputSent, handleForwardedInput);
  });

  onCleanup(() => {
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
          <p class="text-[11px] uppercase tracking-wide text-neutral-400">Queued Commands</p>
          <p class="mt-1 text-[11px] text-neutral-500">Commands sent from IntentBar are shown here for manual paste into term-web.</p>
          <div class="mt-2 max-h-20 overflow-auto space-y-1 font-mono text-[11px] text-neutral-300">
            <For each={queuedCommands()}>{(item) => <div class="truncate" title={item.text}>{item.execute ? "$ " : ""}{item.text}</div>}</For>
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
          />
        </div>
      </Show>

      <div class="flex items-center justify-between text-[11px] text-neutral-500">
        <p class="truncate">Active: {termWebUrl() || "not connected"}</p>
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
