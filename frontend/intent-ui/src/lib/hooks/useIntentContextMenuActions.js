import { createMemo, createSignal } from "solid-js";
import {
  TbOutlineApps,
  TbOutlineBook2,
  TbOutlineClipboard,
  TbOutlineCode,
  TbOutlineCopy,
  TbOutlineDeviceDesktop,
  TbOutlineFileText,
  TbOutlineHistory,
  TbOutlineKey,
  TbOutlineRefresh,
  TbOutlineTerminal
} from "solid-icons/tb";
import { publishEvent } from "../../stores/eventbus";
import { pushClipboardEntry } from "../../stores/clipboard-history";
import { toggleIntentBar } from "../../stores/ui-actions";
import { openWindow } from "../../stores/windows";
import { startNewCodexSession } from "../../stores/workflow-ui";

function resolveCopyText(target) {
  if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
    const start = target.selectionStart ?? 0;
    const end = target.selectionEnd ?? start;
    if (end > start) return target.value.slice(start, end);
  }
  if (typeof window === "undefined") return "";
  return window.getSelection()?.toString() || "";
}

function pasteIntoTarget(target, text) {
  if (!target || !text) return false;
  if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
    const start = target.selectionStart ?? target.value.length;
    const end = target.selectionEnd ?? start;
    target.focus();
    target.setRangeText(text, start, end, "end");
    target.dispatchEvent(new Event("input", { bubbles: true }));
    return true;
  }
  if (target instanceof HTMLElement && target.isContentEditable) {
    target.focus();
    document.execCommand("insertText", false, text);
    return true;
  }
  return false;
}

async function readClipboardText(setClipboardText) {
  if (!navigator.clipboard?.readText) {
    setClipboardText("");
    return;
  }
  try {
    const text = await navigator.clipboard.readText();
    setClipboardText(String(text || ""));
    pushClipboardEntry(text, "clipboard-read");
  } catch {
    setClipboardText("");
  }
}

export function useIntentContextMenuActions() {
  const [menuOpen, setMenuOpen] = createSignal(false);
  const [menuPos, setMenuPos] = createSignal({ x: 0, y: 0 });
  const [clipboardText, setClipboardText] = createSignal("");
  const [copyText, setCopyText] = createSignal("");
  const [contextTarget, setContextTarget] = createSignal(null);

  const canPaste = createMemo(() => clipboardText().trim().length > 0);
  const canCopy = createMemo(() => copyText().trim().length > 0);
  const contextActions = [
    { label: "Open Files", icon: TbOutlineFileText, run: () => openWindow("files") },
    { label: "Open Editor", icon: TbOutlineCode, run: () => openWindow("editor") },
    { label: "Open Terminal", icon: TbOutlineTerminal, run: () => openWindow("terminal") },
    { label: "Open Credentials", icon: TbOutlineKey, run: () => openWindow("credentials") },
    { label: "Open ONVIF", icon: TbOutlineDeviceDesktop, run: () => openWindow("onvif") },
    { label: "Open Widgets", icon: TbOutlineApps, run: () => openWindow("widgets") },
    {
      label: "New Codex Session",
      icon: TbOutlineHistory,
      run: () => {
        startNewCodexSession();
        toggleIntentBar();
      }
    },
    { label: "Open Guide", icon: TbOutlineBook2, run: () => openWindow("guide") },
    { label: "Reload UI", icon: TbOutlineRefresh, run: () => window.location.reload() }
  ];

  const contextMenuActions = createMemo(() => {
    const actions = [];
    if (canCopy()) {
      actions.push({
        label: "Copy",
        icon: TbOutlineCopy,
        run: async () => {
          try {
            await navigator.clipboard?.writeText(copyText());
            pushClipboardEntry(copyText(), "copy-action");
            publishEvent("clipboard.copied", { text: copyText() }, { source: "intent-ui" });
          } catch {
            // ignore clipboard permission failures
          }
        }
      });
    }
    if (canPaste()) {
      actions.push({
        label: "Paste",
        icon: TbOutlineClipboard,
        run: async () => {
          const text = clipboardText();
          if (!text) return;
          const target = contextTarget() || document.activeElement;
          const inserted = pasteIntoTarget(target, text);
          pushClipboardEntry(text, "paste-action");
          publishEvent("clipboard.pasted", { text }, { source: "intent-ui" });
          if (!inserted && document.activeElement instanceof HTMLElement && document.activeElement.isContentEditable) {
            document.execCommand("insertText", false, text);
          }
        }
      });
    }
    if (actions.length > 0) {
      actions.push({ label: "__sep__", run: () => {} });
    }
    actions.push(...contextActions);
    return actions;
  });

  const handleRootContextMenu = (event) => {
    event.preventDefault();
    event.stopPropagation();
    setContextTarget(event.target instanceof Element ? event.target : null);
    setCopyText(resolveCopyText(event.target));
    void readClipboardText(setClipboardText);
    setMenuPos({ x: event.clientX, y: event.clientY });
    setMenuOpen(true);
  };

  return {
    menuOpen,
    setMenuOpen,
    menuPos,
    contextMenuActions,
    handleRootContextMenu
  };
}
