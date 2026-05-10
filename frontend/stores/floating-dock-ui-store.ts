"use client";

import { atom, computed, type WritableAtom } from "nanostores";
import { useStore } from "@nanostores/react";
import { useCallback, useMemo } from "react";

type PrefixSequence = readonly ["/", "?", "~", "!"];
export const COMMAND_PREFIX_SEQUENCE: PrefixSequence = ["/", "?", "~", "!"];

export type DockPage = "people" | "launcher" | "command";
export type CommandPrefix = (typeof COMMAND_PREFIX_SEQUENCE)[number];
const COMMAND_PREFIX_SET = new Set<CommandPrefix>(COMMAND_PREFIX_SEQUENCE);

export type FloatingDockUiState = {
  page: DockPage;
  command: string;
  currentPrefix: CommandPrefix;
  historyIndex: number | null;
  copiedMessageId: string | null;
  messageTimeNow: number;
};

export type DockPageUpdater = DockPage | ((page: DockPage) => DockPage);
export type HistoryIndexUpdater = number | null | ((index: number | null) => number | null);

type DockCommandTransition =
  | { type: "set"; prefix: CommandPrefix; body?: string }
  | { type: "setWithCurrentBody"; prefix: CommandPrefix }
  | { type: "setFromInput"; value: string; basePrefix: CommandPrefix }
  | { type: "reset" };

export type FloatingDockUiStoreConfig = {
  initialPage?: DockPage;
  initialCommand?: string;
  initialPrefix?: CommandPrefix;
  initialHistoryIndex?: number | null;
  initialMessageTimeNow?: number;
  extractCommandText?: (command: string) => string;
};

type FloatingDockUiStoreActions = {
  readonly page: DockPage;
  readonly command: string;
  readonly currentPrefix: CommandPrefix;
  readonly commandPrefix: CommandPrefix;
  readonly historyIndex: number | null;
  readonly copiedMessageId: string | null;
  readonly messageTimeNow: number;
  readonly commandText: string;
  readonly commandTextTrimmed: string;
  readonly hasCommandText: boolean;
  setPage: (next: DockPageUpdater) => void;
  setHistoryIndex: (next: HistoryIndexUpdater) => void;
  setCopiedMessageId: (value: string | null) => void;
  clearCopiedMessageIdIfCurrent: (value: string) => void;
  setMessageTimeNow: (value: number) => void;
  setCommand: (prefix: CommandPrefix, body?: string) => void;
  setCommandFromInput: (value: string, basePrefix: CommandPrefix) => void;
  resetCommand: () => void;
  clearHistoryIndex: () => void;
  cycleCommandPrefix: () => void;
};

export function isCommandPrefix(value: string): value is CommandPrefix {
  return COMMAND_PREFIX_SET.has(value as CommandPrefix);
}

export function commandPrefixFor(value: string, fallback: CommandPrefix = "/"): CommandPrefix {
  const first = value.trimStart().slice(0, 1);
  return isCommandPrefix(first) ? first : fallback;
}

export function commandBody(value: string) {
  const trimmed = value.trimStart();
  const first = trimmed[0];
  return isCommandPrefix(first || "") ? trimmed.slice(1) : trimmed;
}

export function nextPrefix(prefix: CommandPrefix): CommandPrefix {
  const index = COMMAND_PREFIX_SEQUENCE.indexOf(prefix);
  return COMMAND_PREFIX_SEQUENCE[(index + 1) % COMMAND_PREFIX_SEQUENCE.length] as CommandPrefix;
}

function reduceDockCommand(
  current: FloatingDockUiState,
  transition: DockCommandTransition,
): FloatingDockUiState {
  switch (transition.type) {
    case "set":
      return {
        ...current,
        currentPrefix: transition.prefix,
        command: `${transition.prefix}${transition.body ?? ""}`,
      };
    case "setWithCurrentBody":
      return {
        ...current,
        currentPrefix: transition.prefix,
        command: `${transition.prefix}${commandBody(current.command)}`,
      };
    case "setFromInput": {
      const trimmedValue = transition.value.trimStart();
      const parsedPrefix = trimmedValue.slice(0, 1);
      if (isCommandPrefix(parsedPrefix)) {
        return {
          ...current,
          currentPrefix: parsedPrefix,
          command: `${parsedPrefix}${trimmedValue.slice(1)}`,
        };
      }
      return {
        ...current,
        currentPrefix: transition.basePrefix,
        command: `${transition.basePrefix}${transition.value}`,
      };
    }
    case "reset":
      return {
        ...current,
        currentPrefix: "~",
        command: "~",
      };
    default:
      return current;
  }
}

const DEFAULT_EXTRACT_COMMAND_TEXT = (command: string) => commandBody(command);

export function useFloatingDockUiStore({
  initialPrefix = "~",
  initialPage = "command",
  initialCommand = "",
  initialHistoryIndex = null,
  initialMessageTimeNow = Date.now(),
  extractCommandText,
}: FloatingDockUiStoreConfig = {}): FloatingDockUiStoreActions {
  const extractBody = useMemo(() => extractCommandText ?? DEFAULT_EXTRACT_COMMAND_TEXT, [extractCommandText]);
  const store = useMemo(() => atom<FloatingDockUiState>({
    page: initialPage,
    command: initialCommand,
    currentPrefix: initialPrefix,
    historyIndex: initialHistoryIndex,
    copiedMessageId: null,
    messageTimeNow: initialMessageTimeNow,
  }), [initialCommand, initialHistoryIndex, initialMessageTimeNow, initialPage, initialPrefix]);

  const state = useStore(store);
  const commandText = useStore(useMemo(
    () => computed(store as WritableAtom<FloatingDockUiState>, (current) => extractBody(current.command)),
    [store, extractBody],
  ));
  const commandTextTrimmed = useStore(useMemo(
    () => computed(store as WritableAtom<FloatingDockUiState>, (current) => extractBody(current.command).trim()),
    [store, extractBody],
  ));
  const commandPrefix = useStore(useMemo(
    () => computed(store as WritableAtom<FloatingDockUiState>, (current) => (
      commandPrefixFor(current.command || "", current.currentPrefix)
    )),
    [store],
  ));
  const hasCommandText = commandTextTrimmed.length > 0;

  const setState = useCallback((
    updater: (current: FloatingDockUiState) => FloatingDockUiState,
  ) => {
    const current = store.get();
    const next = updater(current);
    if (next !== current) {
      store.set(next);
    }
  }, [store]);

  const setPage = useCallback((next: DockPageUpdater) => {
    setState((current) => {
      const value = typeof next === "function" ? next(current.page) : next;
      return value === current.page ? current : { ...current, page: value };
    });
  }, [setState]);

  const setHistoryIndex = useCallback((next: HistoryIndexUpdater) => {
    setState((current) => {
      const value = typeof next === "function" ? next(current.historyIndex) : next;
      return value === current.historyIndex ? current : { ...current, historyIndex: value };
    });
  }, [setState]);

  const setCopiedMessageId = useCallback((value: string | null) => {
    setState((current) => (current.copiedMessageId === value ? current : {
      ...current,
      copiedMessageId: value,
    }));
  }, [setState]);

  const clearCopiedMessageIdIfCurrent = useCallback((value: string) => {
    const current = store.get();
    if (current.copiedMessageId !== value) return;
    store.set({ ...current, copiedMessageId: null });
  }, [store]);

  const setMessageTimeNow = useCallback((value: number) => {
    setState((current) => (
      current.messageTimeNow === value ? current : { ...current, messageTimeNow: value }
    ));
  }, [setState]);

  const dispatchDockCommand = useCallback((transition: DockCommandTransition) => {
    setState((current) => reduceDockCommand(current, transition));
  }, [setState]);

  const setCommand = useCallback((prefix: CommandPrefix, body = "") => {
    dispatchDockCommand({ type: "set", prefix, body });
  }, [dispatchDockCommand]);

  const setCommandFromInput = useCallback((value: string, basePrefix: CommandPrefix) => {
    dispatchDockCommand({ type: "setFromInput", value, basePrefix });
  }, [dispatchDockCommand]);

  const resetCommand = useCallback(() => {
    dispatchDockCommand({ type: "reset" });
  }, [dispatchDockCommand]);

  const clearHistoryIndex = useCallback(() => {
    setHistoryIndex(null);
  }, [setHistoryIndex]);

  const cycleCommandPrefix = useCallback(() => {
    setState((current) => {
      const basePrefix = commandPrefixFor(current.command, current.currentPrefix);
      const next = nextPrefix(basePrefix);
      return reduceDockCommand(current, { type: "setWithCurrentBody", prefix: next });
    });
  }, [setState]);

  return {
    page: state.page,
    command: state.command,
    currentPrefix: state.currentPrefix,
    commandPrefix,
    historyIndex: state.historyIndex,
    copiedMessageId: state.copiedMessageId,
    messageTimeNow: state.messageTimeNow,
    commandText,
    commandTextTrimmed,
    hasCommandText,
    setPage,
    setHistoryIndex,
    setCopiedMessageId,
    clearCopiedMessageIdIfCurrent,
    setMessageTimeNow,
    setCommand,
    setCommandFromInput,
    resetCommand,
    clearHistoryIndex,
    cycleCommandPrefix,
  };
}
