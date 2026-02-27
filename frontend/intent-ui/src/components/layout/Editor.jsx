import { createEffect, createSignal, onCleanup, onMount } from "solid-js";
import loader from "@monaco-editor/loader";

function detectLanguage(path) {
  if (!path) return "plaintext";
  const ext = path.split(".").pop()?.toLowerCase() || "";
  if (["js", "jsx", "mjs", "cjs"].includes(ext)) return "javascript";
  if (["ts", "tsx"].includes(ext)) return "typescript";
  if (ext === "json") return "json";
  if (ext === "css") return "css";
  if (ext === "html") return "html";
  if (ext === "md") return "markdown";
  if (ext === "rs") return "rust";
  if (ext === "go") return "go";
  if (ext === "py") return "python";
  if (ext === "sh") return "shell";
  if (ext === "yaml" || ext === "yml") return "yaml";
  return "plaintext";
}

function Editor(props) {
  let containerRef;
  let monaco;
  let editor;
  let model;
  let resizeObserver;
  let suppressChange = false;
  let lastPath = props.path || "untitled.txt";
  let saveStateTimer = null;
  const [saveLabel, setSaveLabel] = createSignal("Save");

  onMount(async () => {
    monaco = await loader.init();
    const initialPath = props.path || "untitled.txt";
    const uri = monaco.Uri.parse(`file:///${initialPath.replace(/^\/+/, "")}`);
    model = monaco.editor.createModel(
      props.value || "",
      detectLanguage(initialPath),
      uri
    );
    editor = monaco.editor.create(containerRef, {
      model,
      theme: "vs-dark",
      automaticLayout: true,
      minimap: { enabled: false },
      fontSize: 13,
      lineHeight: 20,
      fontFamily: "JetBrains Mono, Fira Code, ui-monospace, SFMono-Regular, Menlo, monospace",
      padding: { top: 12 },
      smoothScrolling: true,
      scrollBeyondLastLine: false
    });
    editor.onDidChangeModelContent(() => {
      if (suppressChange) return;
      props.onChange?.(editor.getValue());
    });
    resizeObserver = new ResizeObserver(() => {
      editor?.layout();
    });
    resizeObserver.observe(containerRef);
  });

  createEffect(() => {
    if (!editor || !model) return;
    const nextValue = props.value || "";
    if (nextValue !== editor.getValue()) {
      suppressChange = true;
      model.setValue(nextValue);
      suppressChange = false;
    }
  });

  createEffect(() => {
    if (!editor || !model || !monaco) return;
    const nextPath = props.path || "untitled.txt";
    if (nextPath === lastPath) return;
    lastPath = nextPath;
    monaco.editor.setModelLanguage(model, detectLanguage(nextPath));
  });

  onCleanup(() => {
    if (saveStateTimer) clearTimeout(saveStateTimer);
    resizeObserver?.disconnect();
    editor?.dispose();
    model?.dispose();
  });

  const saveNow = () => {
    const value = editor?.getValue() ?? props.value ?? "";
    if (props.onSave) {
      props.onSave(value);
    } else {
      props.onChange?.(value);
    }
    setSaveLabel("Saved");
    if (saveStateTimer) clearTimeout(saveStateTimer);
    saveStateTimer = setTimeout(() => setSaveLabel("Save"), 1200);
  };

  return <div class="h-full flex flex-col bg-[#111] text-neutral-200">
      <div class="px-3 py-2 border-b border-neutral-800 text-xs text-neutral-400 flex items-center justify-between gap-2">
        <span class="truncate">{props.path || "untitled.txt"}</span>
        <button
          type="button"
          onClick={saveNow}
          class="rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-[11px] text-neutral-200 hover:bg-neutral-800"
        >
          {saveLabel()}
        </button>
      </div>
      <div ref={containerRef} class="flex-1 min-h-0" />
    </div>;
}

export {
  Editor as default
};
