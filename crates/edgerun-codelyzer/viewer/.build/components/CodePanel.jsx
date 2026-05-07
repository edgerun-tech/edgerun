import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { classList as _$classList } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { use as _$use } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { memo as _$memo } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div class="panel shrink-0 flex-col"style=width:500px><div class=panel-header><span class="panel-title truncate"></span><button class="btn px-1.5 py-0.5 text-sm leading-none"></button></div><div class="flex-1 min-h-[200px]"></div><div class="px-3 py-2 text-[11px] text-text-muted border-t border-border-default truncate">`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div class="flex px-3 py-2 gap-2 border-b border-border-default"><button>Current</button><button>Diff`),
  _tmpl$3 = /*#__PURE__*/_$template(`<div class="px-3 py-2 border-t border-border-default bg-bg-tertiary"><div class="flex items-center gap-2 text-xs mb-1.5"><span class="badge bg-green-900/30 text-green-300 border border-green-800/50">Pending</span><span class="text-green-300 font-semibold truncate"></span><button class="btn btn-success text-xs px-2 py-0.5">Apply to Disk</button><button class="btn btn-danger text-xs px-2 py-0.5">Discard</button></div><pre class="text-[11px] font-mono bg-bg-primary border border-border-default rounded-sm p-2 max-h-40 overflow-auto whitespace-pre-wrap break-all text-green-300">`);
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
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.firstChild,
      _el$4 = _el$3.nextSibling,
      _el$5 = _el$2.nextSibling,
      _el$6 = _el$5.nextSibling;
    _$insert(_el$3, (() => {
      var _c$ = _$memo(() => !!codeFilePath());
      return () => _c$() ? codeFilePath().split("/").pop() : "Code Viewer";
    })());
    _el$4.$$click = () => setCodePanelOpen(false);
    _$insert(_el$4, _$createComponent(X, {
      "class": "w-4 h-4"
    }));
    _$insert(_el$, (() => {
      var _c$2 = _$memo(() => !!diffCode());
      return () => _c$2() && (() => {
        var _el$7 = _tmpl$2(),
          _el$8 = _el$7.firstChild,
          _el$9 = _el$8.nextSibling;
        _el$8.$$click = () => setCodeTab("current");
        _el$9.$$click = () => setCodeTab("diff");
        _$effect(_p$ => {
          var _v$ = {
              "btn text-xs": true,
              "btn-primary": codeTab() === "current"
            },
            _v$2 = {
              "btn text-xs": true,
              "btn-primary": codeTab() === "diff"
            };
          _p$.e = _$classList(_el$8, _v$, _p$.e);
          _p$.t = _$classList(_el$9, _v$2, _p$.t);
          return _p$;
        }, {
          e: undefined,
          t: undefined
        });
        return _el$7;
      })();
    })(), _el$5);
    var _ref$ = editorContainerRef;
    typeof _ref$ === "function" ? _$use(_ref$, _el$5) : editorContainerRef = _el$5;
    _$insert(_el$6, () => codeFilePath() || "Select a node to view code");
    _$insert(_el$, (() => {
      var _c$3 = _$memo(() => !!pendingEdit());
      return () => _c$3() && (() => {
        var _el$0 = _tmpl$3(),
          _el$1 = _el$0.firstChild,
          _el$10 = _el$1.firstChild,
          _el$11 = _el$10.nextSibling,
          _el$12 = _el$11.nextSibling,
          _el$13 = _el$12.nextSibling,
          _el$14 = _el$1.nextSibling;
        _$insert(_el$11, () => pendingEdit().path);
        _el$12.$$click = applyEdit;
        _el$13.$$click = discardEdit;
        _$insert(_el$14, () => pendingEdit().output);
        return _el$0;
      })();
    })(), null);
    return _el$;
  })();
}
_$delegateEvents(["click"]);