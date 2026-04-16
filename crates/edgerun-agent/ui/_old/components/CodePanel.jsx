/** @jsxImportSource solid-js **/
/** @jsxImportSource solid-js **/
/**
 * Code viewer panel with Monaco editor integration.
 */
import { onMount, createEffect } from "solid-js";
import { setCodePanelOpen, codeFilePath, setCodeFilePath, setCodeContent, diffCode, setDiffCode, codeTab, setCodeTab, pendingEdit, setPendingEdit, focusedNodeId, nodesMap } from "../store.js";
import { getFileContent } from "../ws.js";
import { X } from "lucide-solid";
let monacoEditor = null;
let monacoInstance = null;
let monacoLoaded = false;
const LANG_MAP = {
  c: "c",
  h: "c",
  cpp: "cpp",
  hpp: "cpp",
  cc: "cpp",
  cxx: "cpp",
  js: "javascript",
  mjs: "javascript",
  ts: "typescript",
  tsx: "typescript",
  rs: "rust",
  py: "python",
  go: "go",
  java: "java",
  json: "json",
  md: "markdown"
};
export function CodePanel() {
  let editorContainerRef;
  onMount(() => {
    initMonaco(editorContainerRef);
  });
  async function initMonaco(container) {
    if (monacoLoaded) return;
    if (window.monaco) {
      monacoLoaded = true;
      monacoInstance = window.monaco;
      createEditorInstance(container);
      return;
    }

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    window.require = {
      paths: {
        vs: "https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.45.0/min/vs"
      }
    };
    const loaderScript = document.createElement("script");
    loaderScript.src = "https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.45.0/min/vs/loader.js";
    loaderScript.onload = () => {
      monacoLoaded = true;
      // eslint-disable-next-line @typescript-eslint/no-unsafe-call, @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-member-access
      window.require(["vs/editor/editor.main"], () => {
        monacoInstance = window.monaco;
        createEditorInstance(container);
      });
    };
    document.head.appendChild(loaderScript);
  }
  function createEditorInstance(container) {
    if (!monacoInstance) return;
    monacoEditor = monacoInstance.editor.create(container, {
      value: "// Select a function to view code",
      language: "javascript",
      theme: "vs-dark",
      readOnly: true,
      minimap: {
        enabled: false
      },
      scrollBeyondLastLine: false,
      fontSize: 13,
      lineNumbers: "on",
      renderLineHighlight: "line",
      automaticLayout: true
    });
  }
  async function loadFile(filePath) {
    setCodeFilePath(filePath);
    try {
      const content = await getFileContent(filePath);
      setCodeContent(content);
      setDiffCode("");
      setCodeTab("current");
      const ext = filePath.split(".").pop()?.toLowerCase() ?? "";
      const lang = LANG_MAP[ext] ?? "plaintext";
      if (monacoEditor && monacoInstance) {
        const model = monacoEditor.getModel();
        if (model) {
          monacoInstance.editor.setModelLanguage(model, lang);
        }
        monacoEditor.setValue(content);
      }
    } catch (e) {
      setCodeContent(`// Error loading file: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  // Watch for focused node changes
  createEffect(() => {
    const id = focusedNodeId();
    if (id) {
      const node = nodesMap().get(id);
      if (node?.file) {
        loadFile(node.file);
      }
    }
  });
  function applyEdit() {
    const edit = pendingEdit();
    if (!edit) return;
    void wsApplyEdit(edit.path, edit.content ?? "").then(result => {
      if (result.error) {
        console.error("Failed to apply edit:", result.error);
      } else {
        setPendingEdit(null);
        loadFile(edit.path);
      }
    }).catch(console.error);
  }
  function discardEdit() {
    setPendingEdit(null);
  }
  return <div class="panel shrink-0 flex-col" style={{
    width: "500px"
  }}>
      {/* Header */}
      <div class="panel-header">
        <span class="panel-title truncate">{codeFilePath() ? codeFilePath().split("/").pop() : "Code Viewer"}</span>
        <button class="btn px-1.5 py-0.5 text-sm leading-none" onClick={() => setCodePanelOpen(false)}>
          <X class="w-4 h-4" />
        </button>
      </div>

      {/* Tabs */}
      {diffCode() && <div class="flex px-3 py-2 gap-2 border-b border-border-default">
          <button classList={{
        "btn text-xs": true,
        "btn-primary": codeTab() === "current"
      }} onClick={() => setCodeTab("current")}>
            Current
          </button>
          <button classList={{
        "btn text-xs": true,
        "btn-primary": codeTab() === "diff"
      }} onClick={() => setCodeTab("diff")}>
            Diff
          </button>
        </div>}

      {/* Editor */}
      <div ref={editorContainerRef} class="flex-1 min-h-[200px]" />

      {/* Info bar */}
      <div class="px-3 py-2 text-[11px] text-text-muted border-t border-border-default truncate">
        {codeFilePath() || "Select a node to view code"}
      </div>

      {/* Pending edit zone */}
      {pendingEdit() && <div class="px-3 py-2 border-t border-border-default bg-bg-tertiary">
          <div class="flex items-center gap-2 text-xs mb-1.5">
            <span class="badge bg-green-900/30 text-green-300 border border-green-800/50">Pending</span>
            <span class="text-green-300 font-semibold truncate">{pendingEdit().path}</span>
            <button class="btn btn-success text-xs px-2 py-0.5" onClick={applyEdit}>
              Apply to Disk
            </button>
            <button class="btn btn-danger text-xs px-2 py-0.5" onClick={discardEdit}>
              Discard
            </button>
          </div>
          <pre class="text-[11px] font-mono bg-bg-primary border border-border-default rounded-sm p-2 max-h-40 overflow-auto whitespace-pre-wrap break-all text-green-300">
            {pendingEdit().output}
          </pre>
        </div>}
    </div>;
}
// Benchmark comment