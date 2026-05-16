/** @jsxImportSource solid-js **/
/** @jsxImportSource solid-js **/
/**
 * Keyboard shortcuts handler.
 */
import { onMount, onCleanup } from "solid-js";
import { setCodePanelOpen, setChatPanelOpen, setFileExplorerOpen, setFocusedNodeWithHighlight, setSearchQuery, searchOpen, setSearchOpen } from "../store.js";
export function KeyboardShortcuts() {
  let ctrlDown = false;
  let shiftDown = false;
  function onKeyDown(e) {
    ctrlDown = e.ctrlKey || e.metaKey;
    shiftDown = e.shiftKey;

    // Don't intercept when typing in inputs
    const tag = e.target.tagName;
    const isInput = tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";

    // Ctrl+K = focus search
    if (ctrlDown && e.key === "k") {
      e.preventDefault();
      setSearchQuery("");
      setSearchOpen(true);
      setTimeout(() => {
        const input = document.querySelector("#search-input");
        input?.focus();
      }, 50);
      return;
    }
    if (isInput) return;

    // Ctrl+Shift+C = toggle code viewer
    if (ctrlDown && shiftDown && e.key === "C") {
      e.preventDefault();
      setCodePanelOpen(v => !v);
      return;
    }

    // Ctrl+Shift+L = toggle chat
    if (ctrlDown && shiftDown && (e.key === "l" || e.key === "L")) {
      e.preventDefault();
      setChatPanelOpen(v => !v);
      return;
    }

    // Ctrl+B = toggle file explorer
    if (ctrlDown && (e.key === "b" || e.key === "B")) {
      e.preventDefault();
      setFileExplorerOpen(v => !v);
      return;
    }

    // Escape = clear selection (only if not in input)
    if (e.key === "Escape" && !isInput) {
      setFocusedNodeWithHighlight(null);
      if (searchOpen()) {
        setSearchOpen(false);
        setSearchQuery("");
      }
      return;
    }

    // Delete/Backspace = clear selection
    if ((e.key === "Delete" || e.key === "Backspace") && !isInput) {
      setFocusedNodeWithHighlight(null);
    }

    // Ctrl+1/2/3 = switch views
    if (ctrlDown && e.key >= "1" && e.key <= "3") {
      e.preventDefault();
      const views = ["functions", "files", "directories"];
      const idx = parseInt(e.key) - 1;
      // Handled by the select element in Sidebar
      const select = document.querySelector("#view-select");
      if (select) {
        select.value = views[idx] ?? "functions";
        select.dispatchEvent(new Event("change"));
      }
    }
  }
  function onKeyUp(e) {
    ctrlDown = e.ctrlKey || e.metaKey;
    shiftDown = e.shiftKey;
  }

  // Reset on window blur
  function onBlur() {
    ctrlDown = false;
    shiftDown = false;
  }
  onMount(() => {
    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("keyup", onKeyUp);
    window.addEventListener("blur", onBlur);
  });
  onCleanup(() => {
    document.removeEventListener("keydown", onKeyDown);
    document.removeEventListener("keyup", onKeyUp);
    window.removeEventListener("blur", onBlur);
  });

  // This component renders nothing — it's just a logic hook
  return null;
}