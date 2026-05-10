"use client";
import { cn } from "@/lib/utils";
import { sendCodexMessage } from "@/lib/codex-stream";
import { AssistantMessageContent } from "@/components/os/assistant-message-content";
import { useStore } from "@nanostores/react";
import { Bot, Check, Copy, Loader2, PanelTopClose, Square, User } from "lucide-react";
import {
  AnimatePresence,
  motion,
  useMotionValue,
  useSpring,
  useTransform,
} from "motion/react";
import type { MotionValue } from "motion/react";

import { useEffect, useMemo, useRef, useState, useCallback, type ReactNode } from "react";
import { bootstrapBrowserCdpRelay } from "@/platform/dev/browser-cdp-relay";
import {
  appendAssistantMessage,
  assistantElapsedMsStore,
  assistantLastDurationStore,
  assistantLoadingStore,
  assistantMessagesStore,
  assistantStartedAtStore,
  assistantStatusStore,
  abortAssistantRequest,
  clearAssistantMessages,
  finishAssistantRequest,
  formatAssistantMessageTime,
  getAssistantUserMessageContents,
  getLastAssistantUserMessage,
  hasRetryableAssistantError,
  isAssistantAbortError,
  setAssistantStatus,
  setAssistantAbortController,
  startAssistantRequest,
  updateAssistantMessages,
  type AssistantMessage,
  type AssistantStatus,
} from "@/stores/assistant-store";

type FloatingDockItem = {
  title: string;
  icon: ReactNode;
  href?: string;
  onClick?: () => void;
  kind?: "app" | "person" | "trigger";
  subtitle?: string;
};

type FloatingDockContext = {
  mode?: "apps" | "chat-heads" | "triggers";
  items?: FloatingDockItem[];
};

type DockPage = "people" | "launcher" | "command";
type CommandPrefix = "/" | "?" | "~";

type CommandSuggestion = {
  value: string;
  label: string;
  prefix: CommandPrefix;
};

type DockCommandInputEvent = CustomEvent<{
  prefix?: CommandPrefix;
  value?: string;
}>;

type AssistantStatusEvent = CustomEvent<{ status?: AssistantStatus }>;

const COMMAND_PREFIXES: Record<CommandPrefix, { label: string; title: string; className: string }> = {
  "/": { label: "General", title: "General command", className: "text-muted-foreground hover:text-foreground" },
  "?": { label: "Help", title: "Help topics", className: "text-amber-300 hover:text-amber-200" },
  "~": { label: "AI", title: "AI input", className: "text-primary hover:text-primary/85" },
};

const SUGGESTIONS: CommandSuggestion[] = [
  { prefix: "/", value: "/open settings", label: "Open Settings" },
  { prefix: "/", value: "/lock", label: "Lock profile" },
  { prefix: "/", value: "/node identity", label: "Node identity" },
  { prefix: "/", value: "/checklist", label: "Checklist" },
  { prefix: "?", value: "?commands", label: "Prompt commands" },
  { prefix: "?", value: "?node", label: "Node commands" },
  { prefix: "?", value: "?checklist", label: "Checklist commands" },
  { prefix: "?", value: "?open", label: "Open apps" },
];

function commandPrefixFor(value: string): CommandPrefix {
  const first = value.trimStart().slice(0, 1) as CommandPrefix;
  return first === "?" || first === "~" || first === "/" ? first : "/";
}

function commandBody(value: string) {
  const trimmed = value.trimStart();
  return ["/", "?", "~"].includes(trimmed[0] || "") ? trimmed.slice(1) : trimmed;
}

function nextPrefix(prefix: CommandPrefix): CommandPrefix {
  if (prefix === "/") return "?";
  if (prefix === "?") return "~";
  return "/";
}

function isCommandPrefix(value: string): value is CommandPrefix {
  return value === "/" || value === "?" || value === "~";
}

function assistantStatusClass(status: AssistantStatus) {
  if (status === "ready") return "text-emerald-300 hover:text-emerald-200";
  if (status === "offline") return "text-red-300 hover:text-red-200";
  return "text-amber-300 hover:text-amber-200";
}

function assistantPlaceholder(status: AssistantStatus) {
  if (status === "ready") return "ask assistant...";
  if (status === "offline") return "assistant offline...";
  return "assistant checking...";
}

function stripActions(text: string) {
  return text.replace(/\[action:[^\]]+\]/g, "").trim();
}

function formatRequestDuration(ms: number) {
  const seconds = Math.max(0, Math.floor(ms / 1000));
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  if (minutes > 0) return `${minutes}:${remainingSeconds.toString().padStart(2, "0")}`;
  return `${remainingSeconds}s`;
}

function formatRelayResponse(result: unknown, destination: "frontend" | "backend") {
  const label = destination === "backend" ? "backend" : "frontend";
  if (typeof result === "string" && result.trim()) return result.trim();
  if (destination === "frontend") return "Sent to frontend relay.";
  if (result && typeof result === "object") {
    const data = result as { text?: unknown; response?: unknown; ok?: unknown };
    for (const key of ["text", "response"] as const) {
      if (typeof data[key] === "string" && data[key].trim()) return data[key].trim();
    }
    if (data.ok === true) return `Sent to ${label}.`;
    return JSON.stringify(result, null, 2);
  }
  return `Sent to ${label}.`;
}

export const FloatingDock = ({
  items,
  context,
  desktopClassName,
  mobileClassName,
  onCommandSubmit,
}: {
  items: FloatingDockItem[];
  context?: FloatingDockContext;
  desktopClassName?: string;
  mobileClassName?: string;
  onCommandSubmit?: (command: string) => void | string | Promise<void | string>;
}) => {
  const mobileItems = context?.items?.length ? context.items : items;

  return (
    <>
      <FloatingDockDesktop
        launcherItems={items}
        context={context}
        className={desktopClassName}
        onCommandSubmit={onCommandSubmit}
      />
      <FloatingDockMobile items={mobileItems} className={mobileClassName} />
    </>
  );
};

const FloatingDockMobile = ({
  items,
  className,
}: {
  items: FloatingDockItem[];
  className?: string;
}) => {
  const [open, setOpen] = useState(false);
  return (
    <div className={cn("relative block md:hidden", className)}>
      <AnimatePresence>
        {open && (
          <motion.div
            layoutId="nav"
            className="absolute bottom-full left-1/2 mb-3 flex -translate-x-1/2 flex-col gap-2"
          >
            {items.map((item, idx) => (
              <motion.div
                key={item.title}
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: 10 }}
                transition={{ delay: (items.length - 1 - idx) * 0.05 }}
              >
                <button
                  type="button"
                  onClick={() => {
                    item.onClick?.();
                    setOpen(false);
                  }}
                  className="flex h-11 w-11 items-center justify-center rounded-full border border-white/10 bg-transparent text-white shadow-lg backdrop-blur-xl hover:bg-primary/15"
                  aria-label={item.title}
                  title={item.title}
                >
                  <div className="h-4 w-4">{item.icon}</div>
                </button>
              </motion.div>
            ))}
          </motion.div>
        )}
      </AnimatePresence>
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="flex h-11 w-11 items-center justify-center rounded-full border border-white/10 bg-transparent text-white shadow-lg backdrop-blur-xl hover:bg-primary/15"
        aria-label="Open app dock"
      >
        <PanelTopClose className="h-5 w-5" />
      </button>
    </div>
  );
};

const FloatingDockDesktop = ({
  launcherItems,
  context,
  className,
  onCommandSubmit,
}: {
  launcherItems: FloatingDockItem[];
  context?: FloatingDockContext;
  className?: string;
  onCommandSubmit?: (command: string) => void | string | Promise<void | string>;
}) => {
  const mouseX = useMotionValue(Infinity);
  const [page, setPage] = useState<DockPage>("command");
  const [command, setCommand] = useState("");
  const [currentPrefix, setCurrentPrefix] = useState<CommandPrefix>("~");
  const [messageTimeNow, setMessageTimeNow] = useState(() => Date.now());
  const [copiedMessageId, setCopiedMessageId] = useState<string | null>(null);
  const [historyIndex, setHistoryIndex] = useState<number | null>(null);
  const assistantStatus = useStore(assistantStatusStore);
  const assistantLoading = useStore(assistantLoadingStore);
  const assistantMessages = useStore(assistantMessagesStore);
  const assistantStartedAt = useStore(assistantStartedAtStore);
  const assistantElapsedMs = useStore(assistantElapsedMsStore);
  const assistantLastDurationMs = useStore(assistantLastDurationStore);
  const inputRef = useRef<HTMLInputElement>(null);
  const assistantOutputRef = useRef<HTMLDivElement>(null);

  const peopleItems = useMemo(() => context?.items ?? [], [context?.items]);
  const launcherItemsStable = useMemo(() => launcherItems, [launcherItems]);
  const hasPeoplePage = Boolean(peopleItems.length);
  const commandPrefix = commandPrefixFor(command || currentPrefix);
  const commandMode = COMMAND_PREFIXES[commandPrefix];
  const prefixClassName = commandPrefix === "~" ? assistantStatusClass(assistantStatus) : commandMode.className;
  const placeholder = commandPrefix === "~" ? assistantPlaceholder(assistantStatus) : `${commandMode.label.toLowerCase()}...`;
  const commandQuery = commandBody(command).toLowerCase();
  const assistantDurationMs = assistantLoading ? assistantElapsedMs : assistantLastDurationMs;
  const hasCommandText = Boolean(commandBody(command).trim());
  const lastAssistantUserMessage = useMemo(() => getLastAssistantUserMessage(assistantMessages), [assistantMessages]);
  const assistantUserMessages = useMemo(() => getAssistantUserMessageContents(assistantMessages), [assistantMessages]);
  const canRetryAssistant = Boolean(lastAssistantUserMessage) && hasRetryableAssistantError(assistantMessages);
  const promptLaunchItems = useMemo(() => [...launcherItemsStable, ...peopleItems], [launcherItemsStable, peopleItems]);
  const suggestions = useMemo(() => {
    const launchSuggestions: CommandSuggestion[] = commandPrefix === "/"
      ? promptLaunchItems.map((item) => ({
        prefix: "/" as const,
        value: `/${item.title}`,
        label: item.title,
      }))
      : [];

    return [...SUGGESTIONS, ...launchSuggestions]
      .filter((item) => item.prefix === commandPrefix)
      .filter((item) => !commandQuery || item.label.toLowerCase().includes(commandQuery) || item.value.toLowerCase().includes(commandQuery))
      .slice(0, 4);
  }, [commandPrefix, commandQuery, promptLaunchItems]);
  const reversedAssistantMessages = useMemo(() => [...assistantMessages].reverse(), [assistantMessages]);

  useEffect(() => {
    assistantOutputRef.current?.scrollTo({ top: 0, behavior: "smooth" });
  }, [assistantMessages, assistantLoading]);

  useEffect(() => {
    if (!assistantStartedAt) return;
    assistantElapsedMsStore.set(Date.now() - assistantStartedAt);
    const interval = window.setInterval(() => {
      assistantElapsedMsStore.set(Date.now() - assistantStartedAt);
    }, 500);
    return () => window.clearInterval(interval);
  }, [assistantStartedAt]);

  useEffect(() => {
    const interval = window.setInterval(() => setMessageTimeNow(Date.now()), 60_000);
    return () => window.clearInterval(interval);
  }, []);

  useEffect(() => {
    if (!hasPeoplePage && page === "people") setPage("launcher");
  }, [hasPeoplePage, page]);

  useEffect(() => {
    if (page !== "command") return;
    const frame = requestAnimationFrame(() => inputRef.current?.focus());
    return () => cancelAnimationFrame(frame);
  }, [page]);

  useEffect(() => {
    let cancelled = false;

    async function checkAssistant() {
      setAssistantStatus("checking");
      try {
        const res = await fetch("/api/codex", { method: "GET", cache: "no-store" });
        const data = await res.json().catch(() => null) as { ok?: boolean } | null;
        if (!cancelled) setAssistantStatus(res.ok && data?.ok === true ? "ready" : "offline");
      } catch {
        if (!cancelled) setAssistantStatus("offline");
      }
    }

    void checkAssistant();
    const interval = window.setInterval(() => void checkAssistant(), 30_000);
    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  }, []);

  useEffect(() => {
    const onAssistantStatus = (event: Event) => {
      const status = (event as AssistantStatusEvent).detail?.status;
      if (status === "checking" || status === "ready" || status === "offline") {
        setAssistantStatus(status);
      }
    };

    window.addEventListener("edgerun:assistant-status", onAssistantStatus);
    return () => window.removeEventListener("edgerun:assistant-status", onAssistantStatus);
  }, []);

  useEffect(() => {
    const openCommandInput = (event: Event) => {
      const detail = (event as DockCommandInputEvent).detail;
      const prefix = detail?.prefix && detail.prefix in COMMAND_PREFIXES ? detail.prefix : "~";
      const body = detail?.value ? commandBody(detail.value) : "";
      setCurrentPrefix(prefix);
      setCommand(`${prefix}${body}`);
      setPage("command");
      requestAnimationFrame(() => inputRef.current?.focus());
    };

    window.addEventListener("edgerun:dock-command-input", openCommandInput);
    return () => window.removeEventListener("edgerun:dock-command-input", openCommandInput);
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        event.stopPropagation();
        setCurrentPrefix("~");
        setCommand("~");
        setPage("command");
        requestAnimationFrame(() => inputRef.current?.focus());
        return;
      }

      if (!event.ctrlKey || event.metaKey || event.altKey) return;
      if (!["ArrowLeft", "ArrowRight"].includes(event.key)) return;
      event.preventDefault();
      event.stopPropagation();

      if (event.key === "ArrowLeft") {
        setPage((current) => {
          if (current === "people") return "launcher";
          if (current === "launcher") return "command";
          return current;
        });
        return;
      }

      setPage((current) => {
        if (current === "command") return "launcher";
        if (current === "launcher" && hasPeoplePage) return "people";
        return current;
      });
    };

    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () => window.removeEventListener("keydown", onKeyDown, { capture: true });
  }, [hasPeoplePage]);

  const setPrefix = useCallback((prefix: CommandPrefix) => {
    setCurrentPrefix(prefix);
    setCommand(`${prefix}${commandBody(command)}`);
    inputRef.current?.focus();
  }, [command]);

  const cyclePrefix = useCallback(() => {
    setPrefix(nextPrefix(commandPrefix));
  }, [commandPrefix, setPrefix]);

  const sendAssistantMessage = useCallback(async (message: string, options: { appendUser?: boolean; force?: boolean } = {}) => {
    const relay = bootstrapBrowserCdpRelay();
    const relayDestination = relay?.status().destination;
    const selectedRelayDestination = relayDestination === "frontend" || relayDestination === "backend" ? relayDestination : null;
    const startedAt = Date.now();
    const controller = new AbortController();
    startAssistantRequest(startedAt);
    setAssistantAbortController(controller);
    if (options.appendUser !== false) {
      updateAssistantMessages((current) => [
        ...current.slice(-3),
        { id: `user-${Date.now()}`, role: "user", content: message },
      ]);
    }
    const assistantId = `assistant-${Date.now()}`;
    updateAssistantMessages((current) => [
      ...current.slice(-3),
      { id: assistantId, role: "assistant", content: selectedRelayDestination ? `Sending to ${selectedRelayDestination}...` : "Starting Codex..." },
    ]);

    try {
      if (selectedRelayDestination) {
        const result = await relay!.relay(message, selectedRelayDestination, {
          source: "floating-dock",
          route: "dock-prompt",
          prefix: "~",
        });
        updateAssistantMessages((current) => current.map((item) => (
          item.id === assistantId
            ? { ...item, content: formatRelayResponse(result, selectedRelayDestination) }
            : item
        )));
        setAssistantStatus("ready");
        return;
      }

      if (assistantStatus === "offline" && !options.force) {
        updateAssistantMessages((current) => current.map((item) => (
          item.id === assistantId
            ? { ...item, content: "Codex bridge is offline." }
            : item
        )));
        return;
      }

      await sendCodexMessage(message, {
        onStatus: (text) => {
          updateAssistantMessages((current) => current.map((item) => (
            item.id === assistantId && (item.content === "Starting Codex..." || item.content.startsWith("Network error."))
              ? { ...item, content: text }
              : item
          )));
        },
        onMessage: (text) => {
          updateAssistantMessages((current) => current.map((item) => (
            item.id === assistantId
              ? { ...item, content: stripActions(text || "Codex completed without a final message.") }
              : item
          )));
        },
      }, { signal: controller.signal });
      setAssistantStatus("ready");
    } catch (error) {
      if (isAssistantAbortError(error)) {
        updateAssistantMessages((current) => current.map((item) => (
          item.id === assistantId
            ? { ...item, content: "Stopped." }
            : item
        )));
        setAssistantStatus("ready");
        return;
      }
      updateAssistantMessages((current) => current.map((item) => (
        item.id === assistantId
          ? { ...item, content: `Error: ${error instanceof Error ? error.message : String(error)}` }
          : item
      )));
      setAssistantStatus("offline");
    } finally {
      finishAssistantRequest(startedAt, assistantStatusStore.get());
    }
  }, [assistantStatus]);

  const appendDockMessage = useCallback((message: Omit<AssistantMessage, "id">) => {
    appendAssistantMessage(message, 6);
  }, []);

  const submitCommand = useCallback((event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const body = commandBody(command).trim();
    if (!body) return;

    if (commandPrefix === "~") {
      void sendAssistantMessage(body);
      setCommand("");
      setCurrentPrefix("~");
      setHistoryIndex(null);
      setPage("command");
      return;
    }

    const submittedCommand = `${commandPrefix}${body}`;
    appendDockMessage({ role: "user", content: submittedCommand });
    setCommand("");
    setCurrentPrefix("~");
    setHistoryIndex(null);
    setPage("command");

    if (commandPrefix === "/") {
      const launchItem = promptLaunchItems.find((item) => {
        const title = item.title.toLowerCase();
        const normalizedBody = body.toLowerCase();
        return title === normalizedBody || title.startsWith(normalizedBody);
      });
      if (launchItem?.onClick) {
        launchItem.onClick();
        appendDockMessage({ role: "assistant", content: `Opened ${launchItem.title}` });
        return;
      }
    }

    if (!onCommandSubmit) {
      appendDockMessage({
        role: "assistant",
        content: `No handler is wired for ${commandPrefix} commands in this view.`,
      });
      return;
    }

    assistantLoadingStore.set(true);
    void Promise.resolve(onCommandSubmit(submittedCommand))
      .then((result) => {
        appendDockMessage({
          role: "assistant",
          content: typeof result === "string" && result.trim() ? result.trim() : `Handled ${submittedCommand}`,
        });
      })
      .catch((error) => {
        appendDockMessage({
          role: "assistant",
          content: `Error: ${error instanceof Error ? error.message : String(error)}`,
        });
      })
      .finally(() => {
        assistantLoadingStore.set(false);
      });
  }, [appendDockMessage, command, commandPrefix, onCommandSubmit, promptLaunchItems, sendAssistantMessage]);

  const applySuggestion = useCallback((value: string) => {
    setCommand(value);
    setCurrentPrefix(commandPrefixFor(value));
    setHistoryIndex(null);
    inputRef.current?.focus();
  }, []);

  const recallAssistantHistory = useCallback((direction: -1 | 1) => {
    if (!assistantUserMessages.length) return false;
    const nextIndex = historyIndex === null
      ? (direction === -1 ? assistantUserMessages.length - 1 : 0)
      : (historyIndex + direction + assistantUserMessages.length) % assistantUserMessages.length;
    setHistoryIndex(nextIndex);
    setCurrentPrefix("~");
    setCommand(`~${assistantUserMessages[nextIndex]}`);
    return true;
  }, [assistantUserMessages, historyIndex]);

  const copyAssistantMessage = useCallback(async (message: AssistantMessage) => {
    await navigator.clipboard.writeText(message.content);
    setCopiedMessageId(message.id);
    window.setTimeout(() => setCopiedMessageId((current) => current === message.id ? null : current), 1200);
  }, []);

  const clearDockPrompt = useCallback(() => {
    if (hasCommandText) {
      setCommand(commandPrefix);
      setHistoryIndex(null);
      inputRef.current?.focus();
      return;
    }
    clearAssistantMessages();
  }, [commandPrefix, hasCommandText]);

  return (
    <motion.div
      layout
      onMouseMove={(e) => mouseX.set(e.pageX)}
      onMouseLeave={() => mouseX.set(Infinity)}
      className={cn(
        "mx-auto hidden h-16 max-w-[calc(100vw-2rem)] items-end rounded-2xl border border-transparent bg-transparent px-4 pb-2.5 md:flex",
        className,
      )}
      data-dock-page={page}
      role="toolbar"
      aria-label="Application dock"
    >
      <div className="relative flex h-full min-w-0 items-end">
        <AnimatePresence>
          {assistantMessages.length > 0 && page === "command" ? (
            <motion.div
              key="assistant-output"
              initial={{ opacity: 0, y: 10, scale: 0.98 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: 10, scale: 0.98 }}
              ref={assistantOutputRef}
              className="assistant-chat-fade-top scrollbar-none absolute bottom-[calc(100%+10px)] left-1/2 flex max-h-[min(42vh,360px)] w-[min(560px,calc(100vw-2rem))] -translate-x-1/2 flex-col-reverse gap-1.5 overflow-y-auto p-0 text-xs text-white"
              role="log"
              aria-live="polite"
            >
              {reversedAssistantMessages.map((message) => (
                <div
                  key={message.id}
                  className={cn(
                    "flex min-w-0 items-start gap-2",
                    message.role === "user"
                      ? "ml-auto max-w-[88%] justify-end"
                      : "mr-auto w-full max-w-full justify-start",
                  )}
                >
                  {message.role === "assistant" ? (
                    <span className="mt-1 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border border-white/10 text-white/55">
                      <Bot className="h-3 w-3" />
                    </span>
                  ) : null}
                  <div className={cn("min-w-0", message.role === "user" ? "text-right" : "text-left")}>
                    <div className={cn("mb-0.5 flex items-center gap-1.5 text-[10px] leading-none text-white/45", message.role === "user" ? "justify-end" : "justify-start")}>
                      <span>{message.role === "user" ? "You" : "Codex"}</span>
                      <span>{formatAssistantMessageTime(message.createdAt, messageTimeNow)}</span>
                      <button
                        type="button"
                        onClick={() => void copyAssistantMessage(message)}
                        className="inline-flex h-4 w-4 items-center justify-center rounded text-white/35 transition-colors hover:bg-white/10 hover:text-white/80"
                        aria-label="Copy message"
                        title="Copy message"
                      >
                        {copiedMessageId === message.id ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
                      </button>
                    </div>
                    <div className="min-w-0 px-0 py-0.5 leading-relaxed text-white">
                      <AssistantMessageContent content={message.content} />
                    </div>
                  </div>
                  {message.role === "user" ? (
                    <span className="mt-1 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border border-white/10 text-white/55">
                      <User className="h-3 w-3" />
                    </span>
                  ) : null}
                </div>
              ))}
            </motion.div>
          ) : null}
        </AnimatePresence>

        <AnimatePresence mode="wait" initial={false}>
          {page === "command" ? (
            <motion.form
              key="command-entry"
              initial={{ opacity: 0, x: 42 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: 42 }}
              transition={{ type: "spring", stiffness: 320, damping: 30 }}
              onSubmit={submitCommand}
              className="relative flex h-11 w-[calc(100vw-8rem)] max-w-[540px] items-center gap-2 rounded-full border border-border bg-card px-2.5 shadow-xl"
              role="search"
              aria-label="Command input"
            >
              <button
                type="button"
                onClick={() => {
                  if (assistantLoading && commandPrefix === "~") {
                    abortAssistantRequest();
                    return;
                  }
                  cyclePrefix();
                }}
                className={cn(
                  "flex h-7 w-7 shrink-0 items-center justify-center font-mono text-base font-semibold transition-colors",
                  prefixClassName,
                )}
                aria-label={assistantLoading && commandPrefix === "~" ? "Stop assistant request" : commandMode.title}
                title={assistantLoading && commandPrefix === "~" ? "Stop assistant request" : `${commandMode.title}. Click to cycle mode.`}
              >
                {assistantLoading && commandPrefix === "~" ? <Loader2 className="h-4 w-4 animate-spin" /> : commandPrefix}
              </button>
              <input
                ref={inputRef}
                value={commandBody(command)}
                onChange={(event) => {
                  const nextValue = event.target.value;
                  if (nextValue.length === 1 && isCommandPrefix(nextValue)) {
                    setCurrentPrefix(nextValue);
                    setCommand(nextValue);
                    setHistoryIndex(null);
                    return;
                  }
                  setCommand(`${commandPrefix}${nextValue}`);
                  setHistoryIndex(null);
                }}
                onKeyDown={(event) => {
                  if ((event.key === "ArrowUp" || event.key === "ArrowDown") && (historyIndex !== null || !commandBody(command).trim())) {
                    if (recallAssistantHistory(event.key === "ArrowUp" ? -1 : 1)) {
                      event.preventDefault();
                      return;
                    }
                  }
                  if (event.key === "Escape") {
                    event.preventDefault();
                    if (assistantLoading) {
                      abortAssistantRequest();
                      return;
                    }
                    setPage("launcher");
                    setCommand("");
                    setCurrentPrefix("~");
                    setHistoryIndex(null);
                  }
                  if ((event.metaKey || event.ctrlKey) && event.key === " ") {
                    event.preventDefault();
                    cyclePrefix();
                  }
                }}
                placeholder={placeholder}
                className="min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground/55"
              />
              {suggestions.length > 0 && (
                <div className="hidden min-w-0 max-w-[45%] items-center gap-1 overflow-hidden bg-transparent sm:flex">
                  {suggestions.map((item) => (
                    <button
                      key={item.value}
                      type="button"
                      onClick={() => applySuggestion(item.value)}
                      className="flex h-7 min-w-0 items-center gap-1.5 rounded-full bg-transparent px-2 text-left text-[11px] text-muted-foreground transition-colors hover:bg-white/10 hover:text-foreground"
                    >
                      <span className={cn("font-mono text-xs font-semibold", item.prefix === "~" ? assistantStatusClass(assistantStatus) : COMMAND_PREFIXES[item.prefix].className)}>{item.prefix}</span>
                      <span className="min-w-0 truncate">{item.label}</span>
                    </button>
                  ))}
                </div>
              )}
              {commandPrefix === "~" && !commandBody(command).trim() && canRetryAssistant && lastAssistantUserMessage ? (
                <button
                  type="button"
                  onClick={() => void sendAssistantMessage(lastAssistantUserMessage.content, { appendUser: false, force: true })}
                  disabled={assistantLoading}
                  className="hidden h-7 shrink-0 rounded-full border border-border px-2 text-[11px] font-medium text-muted-foreground transition-colors hover:border-primary/40 hover:text-foreground disabled:opacity-40 sm:inline-flex sm:items-center"
                >
                  Retry
                </button>
              ) : null}
              {commandPrefix === "~" && assistantLoading ? (
                <button
                  type="button"
                  onClick={abortAssistantRequest}
                  className="hidden h-7 shrink-0 items-center gap-1 rounded-full border border-border px-2 text-[11px] font-medium text-muted-foreground transition-colors hover:border-red-300/40 hover:text-red-200 sm:inline-flex"
                  title="Stop assistant request"
                >
                  <Square className="h-3 w-3" />
                  Stop
                </button>
              ) : null}
              {commandPrefix === "~" && (hasCommandText || assistantMessages.length > 0) ? (
                <button
                  type="button"
                  onClick={clearDockPrompt}
                  disabled={assistantLoading}
                  className="hidden h-7 w-7 shrink-0 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-white/10 hover:text-foreground disabled:opacity-40 sm:inline-flex"
                  aria-label="Clear"
                  title="Clear"
                >
                  x
                </button>
              ) : null}
              {commandPrefix === "~" && assistantDurationMs !== null ? (
                <span className="shrink-0 font-mono text-[10px] tabular-nums text-muted-foreground" title="Assistant request time">
                  {formatRequestDuration(assistantDurationMs)}
                </span>
              ) : null}
              <span
                className={cn(
                  "h-2 w-2 shrink-0 rounded-full",
                  assistantStatus === "ready" ? "bg-emerald-400" : assistantStatus === "offline" ? "bg-red-400" : "bg-amber-300",
                )}
                title={`Assistant ${assistantStatus}`}
                aria-label={`Assistant ${assistantStatus}`}
              />
            </motion.form>
          ) : (
            <motion.div
              key={page}
              initial={{ opacity: 0, x: page === "people" ? -42 : 42 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: page === "people" ? -42 : 42 }}
              transition={{ type: "spring", stiffness: 320, damping: 30 }}
              className="flex items-end gap-3"
            >
              {(page === "people" ? peopleItems : launcherItemsStable).map((item, index) => (
                <IconContainer
                  mouseX={mouseX}
                  key={`${page}-${item.kind || "app"}-${item.title}-${index}`}
                  {...item}
                />
              ))}
            </motion.div>
          )}
        </AnimatePresence>

        <div className="pointer-events-none absolute left-1/2 top-[calc(100%+12px)] flex -translate-x-1/2 items-center gap-1.5" role="tablist" aria-label="Dock pages">
          <span className={cn("h-1.5 w-1.5 rounded-full transition-colors", page === "command" ? "bg-muted-foreground/70" : "bg-muted-foreground/25")} role="tab" aria-selected={page === "command"} aria-label="Command" />
          <span className={cn("h-1.5 w-1.5 rounded-full transition-colors", page === "launcher" ? "bg-muted-foreground/70" : "bg-muted-foreground/25")} role="tab" aria-selected={page === "launcher"} aria-label="Launcher" />
          {hasPeoplePage && <span className={cn("h-1.5 w-1.5 rounded-full transition-colors", page === "people" ? "bg-muted-foreground/70" : "bg-muted-foreground/25")} role="tab" aria-selected={page === "people"} aria-label="People" />}
        </div>
      </div>
    </motion.div>
  );
};

function IconContainer({
  mouseX,
  title,
  icon,
  onClick,
  subtitle,
}: {
  mouseX: MotionValue<number>;
  title: string;
  icon: ReactNode;
  href?: string;
  onClick?: () => void;
  subtitle?: string;
}) {
  const ref = useRef<HTMLButtonElement>(null);

  const distance = useTransform(mouseX, (val) => {
    const bounds = ref.current?.getBoundingClientRect() ?? { x: 0, width: 0 };
    return val - bounds.x - bounds.width / 2;
  });

  const widthTransform = useTransform(distance, [-150, 0, 150], [42, 72, 42]);
  const heightTransform = useTransform(distance, [-150, 0, 150], [42, 72, 42]);
  const widthTransformIcon = useTransform(distance, [-150, 0, 150], [20, 34, 20]);
  const heightTransformIcon = useTransform(distance, [-150, 0, 150], [20, 34, 20]);

  const width = useSpring(widthTransform, { mass: 0.1, stiffness: 150, damping: 12 });
  const height = useSpring(heightTransform, { mass: 0.1, stiffness: 150, damping: 12 });
  const widthIcon = useSpring(widthTransformIcon, { mass: 0.1, stiffness: 150, damping: 12 });
  const heightIcon = useSpring(heightTransformIcon, { mass: 0.1, stiffness: 150, damping: 12 });

  useEffect(() => {
    return () => {
      width.stop();
      height.stop();
      widthIcon.stop();
      heightIcon.stop();
    };
  }, [width, height, widthIcon, heightIcon]);

  const [hovered, setHovered] = useState(false);

  return (
    <motion.button
      ref={ref}
      type="button"
      style={{ width, height }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={onClick}
      className="relative flex cursor-pointer items-center justify-center rounded-full border border-white/10 bg-white/10 text-white shadow-md transition-colors hover:border-primary/30 hover:bg-primary/15"
      aria-label={title}
      title={title}
      role="button"
      tabIndex={0}
    >
      <AnimatePresence>
        {hovered && (
          <motion.div
            initial={{ opacity: 0, y: 10, x: "-50%" }}
            animate={{ opacity: 1, y: 0, x: "-50%" }}
            exit={{ opacity: 0, y: 2, x: "-50%" }}
            className="absolute -top-10 left-1/2 whitespace-nowrap rounded-md border border-border bg-card px-2.5 py-1 text-xs text-card-foreground shadow-xl"
          >
            {title}
            {subtitle && <span className="ml-1 text-muted-foreground">· {subtitle}</span>}
          </motion.div>
        )}
      </AnimatePresence>
      <motion.div
        style={{ width: widthIcon, height: heightIcon }}
        className="flex items-center justify-center overflow-hidden rounded-full"
      >
        {icon}
      </motion.div>
    </motion.button>
  );
}

export type { FloatingDockItem, FloatingDockContext };
