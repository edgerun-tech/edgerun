"use client";
import { cn } from "@/lib/utils";
import { sendCodexMessage } from "@/lib/codex-stream";
import { AssistantMessageContent } from "@/components/os/assistant-message-content";
import { COMMAND_PREFIXES, CommandPrefixConfig, type CommandSuggestion, SUGGESTIONS } from "@/components/ui/floating-dock-config";
import { parseDockCommandInputEvent } from "@/components/ui/floating-dock-events";
import { resolveDockCommandRoute, type RelayDestination, type RouteMode } from "@/components/ui/floating-dock-route";
import { useStore } from "@nanostores/react";
import { atom } from "nanostores";
import { Bot, Check, Copy, Loader2, PanelTopClose, Square, User } from "lucide-react";
import {
  AnimatePresence,
  motion,
  useMotionValue,
  useSpring,
  useTransform,
} from "motion/react";
import type { MotionValue } from "motion/react";

import {
  useEffect,
  useMemo,
  useRef,
  useCallback,
  type FormEvent,
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
} from "react";
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
import {
  commandPrefixFor,
  type CommandPrefix,
  type DockPageUpdater,
  useFloatingDockUiStore,
} from "@/stores/floating-dock-ui-store";

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

type AssistantRoute = "backend" | "chatgpt";

type AssistantStatusEvent = CustomEvent<{ status?: AssistantStatus }>;

type CommandMode = "assistant" | "command";

type CommandInputContext = {
  commandMode: CommandMode;
  commandTextTrimmed: string;
  hasCommandText: boolean;
  isAssistantMode: boolean;
  supportsLaunchItems: boolean;
  assistantRoute: AssistantRoute;
  placeholder: string;
  canShowRetry: boolean;
  canShowStop: boolean;
  canShowClear: boolean;
  canShowTiming: boolean;
  canRecallHistory: boolean;
  requestDurationMs: number | null;
};

type CommandSubmitActions = Record<CommandMode, (body: string) => void>;
function resolveCommandInputContext({
  commandTextTrimmed,
  commandModeConfig,
  assistantStatus,
  assistantLoading,
  assistantElapsedMs,
  assistantLastDurationMs,
  canRetryAssistantMessage,
  assistantMessagesCount,
  assistantUserMessageCount,
}: {
  commandTextTrimmed: string;
  commandModeConfig: CommandPrefixConfig;
  assistantStatus: AssistantStatus;
  assistantLoading: boolean;
  assistantElapsedMs: number;
  assistantLastDurationMs: number | null;
  canRetryAssistantMessage: boolean;
  assistantMessagesCount: number;
  assistantUserMessageCount: number;
}): CommandInputContext {
  const isAssistantMode = commandModeConfig.mode === "assistant";
  const hasCommandText = Boolean(commandTextTrimmed);
  const assistantRoute = isAssistantMode ? getAssistantRoute(commandModeConfig) : "backend";
  const requestDurationMs = isAssistantMode ? (assistantLoading ? assistantElapsedMs : assistantLastDurationMs) : null;

  return {
    commandMode: commandModeConfig.mode,
    commandTextTrimmed,
    hasCommandText,
    isAssistantMode,
    supportsLaunchItems: commandModeConfig.supportsLaunchItems,
    assistantRoute,
    placeholder: isAssistantMode
      ? (assistantRoute === "chatgpt"
        ? "ask chatgpt via local cdp..."
        : assistantPlaceholder(assistantStatus))
      : `${commandModeConfig.label.toLowerCase()}...`,
    canShowRetry: isAssistantMode && !hasCommandText && canRetryAssistantMessage,
    canShowStop: isAssistantMode && assistantLoading,
    canShowClear: isAssistantMode && (hasCommandText || assistantMessagesCount > 0),
    canShowTiming: isAssistantMode && requestDurationMs !== null,
    canRecallHistory: isAssistantMode && assistantUserMessageCount > 0,
    requestDurationMs,
  };
}

function assistantStatusClass(status: AssistantStatus) {
  if (status === "ready") return "text-emerald-300 hover:text-emerald-200";
  if (status === "offline") return "text-red-300 hover:text-red-200";
  return "text-amber-300 hover:text-amber-200";
}

function assistantPlaceholder(status: AssistantStatus) {
  if (status === "ready") return "ask backend codex...";
  if (status === "offline") return "assistant offline...";
  return "assistant checking...";
}

function stripActions(text: string) {
  return text.replace(/\[action:[^\]]+\]/g, "").trim();
}

function getPrefixClassName(prefix: CommandPrefix, status: AssistantStatus) {
  const mode = COMMAND_PREFIXES[prefix];
  return mode.mode === "assistant" && mode.assistantRoute === "backend"
    ? assistantStatusClass(status)
    : mode.className;
}

function getAssistantRoute(mode: CommandPrefixConfig): AssistantRoute {
  return mode.assistantRoute === "chatgpt" ? "chatgpt" : "backend";
}

function formatRequestDuration(ms: number) {
  const seconds = Math.max(0, Math.floor(ms / 1000));
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  if (minutes > 0) return `${minutes}:${remainingSeconds.toString().padStart(2, "0")}`;
  return `${remainingSeconds}s`;
}

function formatRelayResponse(result: unknown, destination: RelayDestination) {
  const label = destination === "backend"
    ? "/api/codex"
    : destination === "chatgpt"
      ? "CDP local ChatGPT"
      : "frontend relay";
  if (typeof result === "string" && result.trim()) return result.trim();
    if (destination === "chatgpt") {
      if (result && typeof result === "object") {
        const data = result as {
        ok?: boolean;
        sent?: boolean;
        text?: unknown;
        response?: unknown;
        composer?: {
          afterText?: string;
          selector?: string | null;
          error?: string;
          reason?: string;
        };
      };
      if (typeof data.text === "string" && data.text.trim()) return data.text.trim();
      if (typeof data.response === "string" && data.response.trim()) return data.response.trim();
      if (data.ok === false) {
        return `CDP send to ${label} failed${data.composer?.error ? `: ${data.composer.error}` : ""}`;
      }
      if (data.sent || data.ok) return `Sent to ${label}.`;
      if (data.composer?.selector && data.composer?.afterText) return `Sent to ${label} using ${data.composer.selector}.`;
      if (data.composer?.error) return `CDP send failed: ${data.composer.error}`;
    }
    return `Sent to ${label}.`;
  }
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

function useDockCommandInput({
  hasPeoplePage,
  setPage,
  setCommand,
  resetCommand,
  focusDockInput,
  submitCurrentCommand,
}: {
  hasPeoplePage: boolean;
  setPage: (page: DockPageUpdater) => void;
  setCommand: (prefix: CommandPrefix, body?: string) => void;
  resetCommand: () => void;
  focusDockInput: () => void;
  submitCurrentCommand: () => void;
}) {
  useEffect(() => {
    const openCommandInput = (event: Event) => {
      const resolvedInput = parseDockCommandInputEvent(event);
      if (!resolvedInput) return;
      const { prefix, body, shouldSubmit } = resolvedInput;
      setCommand(prefix, body);
      setPage("command");
      focusDockInput();
      if (shouldSubmit) requestAnimationFrame(() => submitCurrentCommand());
    };

    window.addEventListener("edgerun:dock-command-input", openCommandInput);
    return () => window.removeEventListener("edgerun:dock-command-input", openCommandInput);
  }, [setCommand, setPage, focusDockInput, submitCurrentCommand]);

  const handleDockKeyDown = useCallback((event: KeyboardEvent) => {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
      event.preventDefault();
      event.stopPropagation();
      resetCommand();
      setPage("command");
      focusDockInput();
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
  }, [focusDockInput, hasPeoplePage, setPage, resetCommand]);

  useEffect(() => {
    window.addEventListener("keydown", handleDockKeyDown, { capture: true });
    return () => window.removeEventListener("keydown", handleDockKeyDown, { capture: true });
  }, [handleDockKeyDown]);
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
  const openStore = useMemo(() => atom(false), []);
  const open = useStore(openStore);
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
                    openStore.set(false);
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
        onClick={() => openStore.set(!open)}
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
  const {
    page,
    commandText,
    commandPrefix,
    commandTextTrimmed,
    historyIndex,
    copiedMessageId,
    messageTimeNow,
    setPage,
    setHistoryIndex,
    setCommand,
    setCommandFromInput,
    setCopiedMessageId,
    clearCopiedMessageIdIfCurrent,
    setMessageTimeNow,
    resetCommand,
    clearHistoryIndex,
    cycleCommandPrefix: cycleCommandPrefixInStore,
  } = useFloatingDockUiStore({
    initialPrefix: "~",
  });
  const assistantStatus = useStore(assistantStatusStore);
  const assistantLoading = useStore(assistantLoadingStore);
  const assistantMessages = useStore(assistantMessagesStore);
  const assistantStartedAt = useStore(assistantStartedAtStore);
  const assistantElapsedMs = useStore(assistantElapsedMsStore);
  const assistantLastDurationMs = useStore(assistantLastDurationStore);
  const inputRef = useRef<HTMLInputElement>(null);
  const assistantOutputRef = useRef<HTMLDivElement>(null);
  const formRef = useRef<HTMLFormElement>(null);

  const peopleItems = useMemo(() => context?.items ?? [], [context?.items]);
  const launcherItemsStable = useMemo(() => launcherItems, [launcherItems]);
  const hasPeoplePage = Boolean(peopleItems.length);
  const commandMode = COMMAND_PREFIXES[commandPrefix];
  const prefixClassName = getPrefixClassName(commandPrefix, assistantStatus);
  const commandQuery = commandText.toLowerCase();
  const lastAssistantUserMessage = useMemo(() => getLastAssistantUserMessage(assistantMessages), [assistantMessages]);
  const assistantUserMessages = useMemo(() => getAssistantUserMessageContents(assistantMessages), [assistantMessages]);
  const canRetryAssistant = Boolean(lastAssistantUserMessage) && hasRetryableAssistantError(assistantMessages);
  const commandInputContext = useMemo(
    () => resolveCommandInputContext({
      commandTextTrimmed,
      commandModeConfig: commandMode,
      assistantStatus,
      assistantLoading,
      assistantElapsedMs,
      assistantLastDurationMs,
      canRetryAssistantMessage: canRetryAssistant,
      assistantMessagesCount: assistantMessages.length,
      assistantUserMessageCount: assistantUserMessages.length,
    }),
    [assistantLastDurationMs, assistantLoading, assistantMessages.length, assistantUserMessages.length, assistantElapsedMs, assistantStatus, canRetryAssistant, commandTextTrimmed, commandMode],
  );
  const canShowRetry = commandInputContext.canShowRetry;
  const canShowAssistantStop = commandInputContext.canShowStop;
  const canShowClearControl = commandInputContext.canShowClear;
  const canShowAssistantTiming = commandInputContext.canShowTiming;
  const canRecallAssistantHistory = commandInputContext.canRecallHistory;
  const retryableAssistantMessage = canShowRetry ? lastAssistantUserMessage : null;
  const promptLaunchItems = useMemo(() => [...launcherItemsStable, ...peopleItems], [launcherItemsStable, peopleItems]);
  const suggestions = useMemo(() => {
    const launchSuggestions: CommandSuggestion[] = commandInputContext.supportsLaunchItems
      ? promptLaunchItems.map((item) => ({
        prefix: "/" as const,
        value: `/${item.title}`,
        label: item.title,
      }))
      : [];

    const uniqueSuggestions = new Map<string, CommandSuggestion>();
    for (const item of [...SUGGESTIONS, ...launchSuggestions]) {
      const key = `${item.prefix}:${item.value}`;
      if (!uniqueSuggestions.has(key)) uniqueSuggestions.set(key, item);
    }

      return [...uniqueSuggestions.values()]
      .filter((item) => item.prefix === commandPrefix)
      .filter((item) => !commandQuery || item.label.toLowerCase().includes(commandQuery) || item.value.toLowerCase().includes(commandQuery))
      .slice(0, 4);
  }, [commandInputContext.supportsLaunchItems, commandPrefix, commandQuery, promptLaunchItems]);
  const reversedAssistantMessages = useMemo(() => [...assistantMessages].reverse(), [assistantMessages]);

  const focusDockInput = useCallback(() => {
    requestAnimationFrame(() => inputRef.current?.focus());
  }, []);

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
  }, [setMessageTimeNow]);

  useEffect(() => {
    if (!hasPeoplePage && page === "people") setPage("launcher");
  }, [hasPeoplePage, page, setPage]);

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

  useDockCommandInput({
    hasPeoplePage,
    setPage,
    setCommand,
    resetCommand,
    focusDockInput,
    submitCurrentCommand: () => formRef.current?.requestSubmit(),
  });

  const cycleCommandPrefix = useCallback(() => {
    cycleCommandPrefixInStore();
    focusDockInput();
  }, [cycleCommandPrefixInStore, focusDockInput]);

  const resolveCommandRoute = (explicitRoute: RouteMode) => resolveDockCommandRoute(explicitRoute);

  const appendAssistantStreamStatus = useCallback((assistantId: string, content: string) => {
    updateAssistantMessages((current) => current.map((item) => (
      item.id === assistantId
        ? { ...item, content }
        : item
    )));
  }, []);

  const replaceAssistantStreamStatus = useCallback((
    assistantId: string,
    content: string,
    shouldReplace: (item: AssistantMessage) => boolean = () => true,
  ) => {
    updateAssistantMessages((current) => current.map((item) => (
      item.id === assistantId && shouldReplace(item)
        ? { ...item, content }
        : item
    )));
  }, []);

  const sendToBackend = useCallback(async (message: string, assistantId: string, controller: AbortController) => {
    await sendCodexMessage(message, {
      onStatus: (text) => {
        replaceAssistantStreamStatus(
          assistantId,
          text,
          (item) => item.content.startsWith("Sending to") || item.content.startsWith("Network error."),
        );
      },
      onMessage: (text) => {
        replaceAssistantStreamStatus(
          assistantId,
          stripActions(text || "Codex completed without a final message."),
          () => true,
        );
      },
    }, { signal: controller.signal });
  }, [replaceAssistantStreamStatus]);

  const sendToRelay = useCallback(async (
    message: string,
    assistantId: string,
    relay: ReturnType<typeof bootstrapBrowserCdpRelay>,
    selectedRelayDestination: RelayDestination,
    relayPrefix: "!" | "~",
  ) => {
    const result = await relay.relay(message, selectedRelayDestination, {
      source: "floating-dock",
      route: "dock-prompt",
      prefix: relayPrefix,
    });
    updateAssistantMessages((current) => current.map((item) => (
      item.id === assistantId
        ? { ...item, content: formatRelayResponse(result, selectedRelayDestination) }
        : item
    )));
  }, []);

  const sendAssistantMessage = useCallback(async (message: string, options: { appendUser?: boolean; route?: RouteMode } = {}) => {
    const explicitRoute = options.route || "auto";
    const route = resolveCommandRoute(explicitRoute);
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
    const resolvedRouteLabel = route.resolvedRouteLabel;
    updateAssistantMessages((current) => [
      ...current.slice(-3),
      { id: assistantId, role: "assistant", content: `Sending to ${resolvedRouteLabel}...` },
    ]);

    try {
      const shouldSendToBackend = route.forceBackendMode || (explicitRoute === "auto" && !route.relayCanHandleSelectedDestination);
      if (shouldSendToBackend) {
        await sendToBackend(message, assistantId, controller);
        setAssistantStatus("ready");
        return;
      }

      if (!route.relay || !route.selectedRelayDestination || !route.relayCanHandleSelectedDestination) {
        throw new Error(`No relay route available for ${route.routeLabel}.`);
      }

      await sendToRelay(message, assistantId, route.relay, route.selectedRelayDestination, route.relayPrefix);
      setAssistantStatus("ready");
      return;
    } catch (error) {
      if (isAssistantAbortError(error)) {
        replaceAssistantStreamStatus(assistantId, "Stopped.");
        setAssistantStatus("ready");
        return;
      }
      appendAssistantStreamStatus(assistantId, `Error: ${error instanceof Error ? error.message : String(error)}`);
      setAssistantStatus("offline");
    } finally {
      finishAssistantRequest(startedAt, assistantStatusStore.get());
    }
  }, [appendAssistantStreamStatus, replaceAssistantStreamStatus, sendToBackend, sendToRelay]);

  const appendDockMessage = useCallback((message: Omit<AssistantMessage, "id">) => {
    appendAssistantMessage(message, 6);
  }, []);

  const resetCommandDraft = useCallback(() => {
    resetCommand();
    clearHistoryIndex();
  }, [clearHistoryIndex, resetCommand]);

  const finalizeSubmittedCommand = useCallback(() => {
    resetCommandDraft();
    setPage("command");
  }, [resetCommandDraft, setPage]);

  const commandSubmitActions: CommandSubmitActions = useMemo(() => ({
    assistant: (body) => {
      void sendAssistantMessage(body, { route: commandInputContext.assistantRoute });
      finalizeSubmittedCommand();
    },
    command: (body) => {
      const submittedCommand = `${commandPrefix}${body}`;
      appendDockMessage({ role: "user", content: submittedCommand });
      finalizeSubmittedCommand();

      if (commandInputContext.supportsLaunchItems) {
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
    },
  }), [commandInputContext.assistantRoute, commandInputContext.supportsLaunchItems, appendDockMessage, commandPrefix, finalizeSubmittedCommand, onCommandSubmit, promptLaunchItems, sendAssistantMessage]);

  const submitCommand = useCallback((event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const body = commandInputContext.commandTextTrimmed;
    if (!body) return;
    const action = commandSubmitActions[commandInputContext.commandMode];
    action(body);
  }, [commandInputContext.commandMode, commandInputContext.commandTextTrimmed, commandSubmitActions]);

  const applySuggestion = useCallback((value: string) => {
    setCommand(commandPrefixFor(value), value);
    clearHistoryIndex();
    focusDockInput();
  }, [clearHistoryIndex, focusDockInput, setCommand]);

  const recallAssistantHistory = useCallback((direction: -1 | 1) => {
    if (!canRecallAssistantHistory) return false;
    const nextIndex = historyIndex === null
      ? (direction === -1 ? assistantUserMessages.length - 1 : 0)
      : (historyIndex + direction + assistantUserMessages.length) % assistantUserMessages.length;
    setHistoryIndex(nextIndex);
    setCommand(commandPrefix, assistantUserMessages[nextIndex]);
    return true;
  }, [assistantUserMessages, canRecallAssistantHistory, commandPrefix, historyIndex, setCommand, setHistoryIndex]);

  const copyAssistantMessage = useCallback(async (message: AssistantMessage) => {
    await navigator.clipboard.writeText(message.content);
    setCopiedMessageId(message.id);
    window.setTimeout(() => {
      clearCopiedMessageIdIfCurrent(message.id);
    }, 1200);
  }, [clearCopiedMessageIdIfCurrent, setCopiedMessageId]);

  const clearDockPrompt = useCallback(() => {
    if (commandInputContext.hasCommandText) {
      clearHistoryIndex();
      setCommand(commandPrefix);
      focusDockInput();
      return;
    }
    clearAssistantMessages();
  }, [clearHistoryIndex, commandInputContext.hasCommandText, commandPrefix, focusDockInput, setCommand]);

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
            <CommandInputBar
              key="command-entry"
              formRef={formRef}
              inputRef={inputRef}
              commandPrefix={commandPrefix}
              commandModeTitle={commandMode.title}
              isAssistantMode={commandInputContext.isAssistantMode}
              assistantLoading={assistantLoading}
              prefixClassName={prefixClassName}
              commandText={commandText}
              placeholder={commandInputContext.placeholder}
              onSubmit={submitCommand}
              onPrefixClick={() => {
                if (assistantLoading && commandInputContext.isAssistantMode) {
                  abortAssistantRequest();
                  return;
                }
                cycleCommandPrefix();
              }}
              onInputChange={(nextValue) => {
                setCommandFromInput(nextValue, commandPrefix);
                clearHistoryIndex();
              }}
              onInputKeyDown={(event) => {
                if ((event.key === "ArrowUp" || event.key === "ArrowDown") && canRecallAssistantHistory && (historyIndex !== null || !commandInputContext.commandTextTrimmed)) {
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
                  resetCommandDraft();
                }
                if ((event.metaKey || event.ctrlKey) && event.key === " ") {
                  event.preventDefault();
                  cycleCommandPrefix();
                }
              }}
              suggestions={suggestions}
              onSuggestionSelect={applySuggestion}
              canShowRetry={Boolean(retryableAssistantMessage)}
              onRetry={() => {
                if (!retryableAssistantMessage) return;
                void sendAssistantMessage(retryableAssistantMessage.content, {
                  appendUser: false,
                  route: commandInputContext.assistantRoute,
                });
              }}
              canShowStop={canShowAssistantStop}
              onStop={abortAssistantRequest}
              canShowClear={canShowClearControl}
              onClear={clearDockPrompt}
              clearDisabled={assistantLoading}
              canShowTiming={canShowAssistantTiming}
              timingMs={commandInputContext.requestDurationMs}
              assistantStatus={assistantStatus}
              showAssistantModeIndicator={commandInputContext.isAssistantMode}
            />
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

type CommandInputBarProps = {
  formRef: RefObject<HTMLFormElement>;
  inputRef: RefObject<HTMLInputElement>;
  commandPrefix: CommandPrefix;
  commandModeTitle: string;
  isAssistantMode: boolean;
  assistantLoading: boolean;
  prefixClassName: string;
  commandText: string;
  placeholder: string;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
  onPrefixClick: () => void;
  onInputChange: (nextValue: string) => void;
  onInputKeyDown: (event: KeyboardEvent<HTMLInputElement>) => void;
  suggestions: CommandSuggestion[];
  onSuggestionSelect: (value: string) => void;
  canShowRetry: boolean;
  onRetry: () => void;
  canShowStop: boolean;
  onStop: () => void;
  canShowClear: boolean;
  onClear: () => void;
  clearDisabled: boolean;
  canShowTiming: boolean;
  timingMs: number | null;
  assistantStatus: AssistantStatus;
  showAssistantModeIndicator: boolean;
};

function CommandInputBar({
  formRef,
  inputRef,
  commandPrefix,
  commandModeTitle,
  isAssistantMode,
  assistantLoading,
  prefixClassName,
  commandText,
  placeholder,
  onSubmit,
  onPrefixClick,
  onInputChange,
  onInputKeyDown,
  suggestions,
  onSuggestionSelect,
  canShowRetry,
  onRetry,
  canShowStop,
  onStop,
  canShowClear,
  onClear,
  clearDisabled,
  canShowTiming,
  timingMs,
  assistantStatus,
  showAssistantModeIndicator,
}: CommandInputBarProps) {
  return (
    <motion.form
      ref={formRef}
      initial={{ opacity: 0, x: 42 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: 42 }}
      transition={{ type: "spring", stiffness: 320, damping: 30 }}
      onSubmit={onSubmit}
      className="relative flex h-11 w-[calc(100vw-8rem)] max-w-[540px] items-center gap-2 rounded-full border border-border bg-card px-2.5 shadow-xl"
      role="search"
      aria-label="Command input"
    >
      <button
        type="button"
        onClick={onPrefixClick}
        className={cn(
          "flex h-7 w-7 shrink-0 items-center justify-center font-mono text-base font-semibold transition-colors",
          prefixClassName,
        )}
        aria-label={assistantLoading && isAssistantMode ? "Stop assistant request" : commandModeTitle}
        title={assistantLoading && isAssistantMode ? "Stop assistant request" : `${commandModeTitle}. Click to cycle mode.`}
      >
        {assistantLoading && isAssistantMode ? <Loader2 className="h-4 w-4 animate-spin" /> : commandPrefix}
      </button>
      <input
        ref={inputRef}
        value={commandText}
        onChange={(event) => onInputChange(event.target.value)}
        onKeyDown={onInputKeyDown}
        placeholder={placeholder}
        className="min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground/55"
      />
      {suggestions.length > 0 && (
        <div className="hidden min-w-0 max-w-[45%] items-center gap-1 overflow-hidden bg-transparent sm:flex">
          {suggestions.map((item) => (
            <button
              key={item.value}
              type="button"
              onClick={() => onSuggestionSelect(item.value)}
              className="flex h-7 min-w-0 items-center gap-1.5 rounded-full bg-transparent px-2 text-left text-[11px] text-muted-foreground transition-colors hover:bg-white/10 hover:text-foreground"
            >
              <span className={cn("font-mono text-xs font-semibold", getPrefixClassName(item.prefix, assistantStatus))}>{item.prefix}</span>
              <span className="min-w-0 truncate">{item.label}</span>
            </button>
          ))}
        </div>
      )}
      {canShowRetry ? (
        <button
          type="button"
          onClick={onRetry}
          disabled={assistantLoading}
          className="hidden h-7 shrink-0 rounded-full border border-border px-2 text-[11px] font-medium text-muted-foreground transition-colors hover:border-primary/40 hover:text-foreground disabled:opacity-40 sm:inline-flex sm:items-center"
        >
          Retry
        </button>
      ) : null}
      {canShowStop ? (
        <button
          type="button"
          onClick={onStop}
          className="hidden h-7 shrink-0 items-center gap-1 rounded-full border border-border px-2 text-[11px] font-medium text-muted-foreground transition-colors hover:border-red-300/40 hover:text-red-200 sm:inline-flex"
          title="Stop assistant request"
        >
          <Square className="h-3 w-3" />
          Stop
        </button>
      ) : null}
      {canShowClear ? (
        <button
          type="button"
          onClick={onClear}
          disabled={clearDisabled}
          className="hidden h-7 w-7 shrink-0 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-white/10 hover:text-foreground disabled:opacity-40 sm:inline-flex"
          aria-label="Clear"
          title="Clear"
        >
          x
        </button>
      ) : null}
      {canShowTiming ? (
        <span className="shrink-0 font-mono text-[10px] tabular-nums text-muted-foreground" title="Assistant request time">
          {timingMs !== null ? formatRequestDuration(timingMs) : "0s"}
        </span>
      ) : null}
      {showAssistantModeIndicator ? (
        <span
          className={cn(
            "h-2 w-2 shrink-0 rounded-full",
            assistantStatus === "ready" ? "bg-emerald-400" : assistantStatus === "offline" ? "bg-red-400" : "bg-amber-300",
          )}
          title={`Assistant ${assistantStatus}`}
          aria-label={`Assistant ${assistantStatus}`}
        />
      ) : null}
    </motion.form>
  );
}

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
  const hoveredStore = useMemo(() => atom(false), []);
  const hovered = useStore(hoveredStore);

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

  return (
    <motion.button
      ref={ref}
      type="button"
      style={{ width, height }}
      onMouseEnter={() => hoveredStore.set(true)}
      onMouseLeave={() => hoveredStore.set(false)}
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
