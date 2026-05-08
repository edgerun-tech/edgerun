import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { classList as _$classList } from "solid-js/web";
import { className as _$className } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { memo as _$memo } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div class="panel shrink-0"style=width:480px><div class=panel-header><span class=panel-title>Diagnostics</span><div class="flex gap-1.5"><button class="btn px-2 py-0.5 text-xs"></button><button class="btn px-2 py-0.5 text-xs"></button></div></div><div class="flex-1 overflow-y-auto p-2">`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div class="px-3 py-2 text-xs text-text-muted border-b border-border-default truncate">`),
  _tmpl$3 = /*#__PURE__*/_$template(`<div class="text-center py-8 text-text-muted text-xs">Running linters…`),
  _tmpl$4 = /*#__PURE__*/_$template(`<div class="text-center py-8 text-text-muted text-xs">No linting issues found.`),
  _tmpl$5 = /*#__PURE__*/_$template(`<div class=space-y-1.5>`),
  _tmpl$6 = /*#__PURE__*/_$template(`<button><div class="flex items-start gap-2"><span class="shrink-0 mt-0.5"></span><div class="flex-1 min-w-0"><div class="font-medium truncate"></div><div class="text-[11px] text-text-muted mt-0.5">:<!>:</div></div><span class="badge shrink-0 mt-0.5 capitalize">`);
/** @jsxImportSource solid-js **/
/**
 * Diagnostics panel with clickable navigation to code.
 */
import { For, createEffect } from "solid-js";
import { setDiagnosticsPanelOpen, diagnosticsFile, setDiagnosticsFile, diagnosticsItems, setDiagnosticsItems, diagnosticsLoading, setDiagnosticsLoading, setCodePanelOpen, setCodeFilePath, focusedNodeId, nodesMap } from "../store.js";
import { X, RefreshCw } from "lucide-solid";
import { getDiagnostics } from "../ws.js";
export function DiagnosticsPanel() {
  async function runDiagnostics() {
    const file = diagnosticsFile();
    if (!file) return;
    setDiagnosticsLoading(true);
    try {
      const items = await getDiagnostics(file);
      setDiagnosticsItems(items);
    } catch (e) {
      console.error("Diagnostics error:", e);
    } finally {
      setDiagnosticsLoading(false);
    }
  }

  // Auto-load when file changes
  createEffect(() => {
    const file = diagnosticsFile();
    if (file) {
      void runDiagnostics();
    }
  });

  // Track focused node's file
  createEffect(() => {
    const nodeId = focusedNodeId();
    if (nodeId) {
      const node = nodesMap().get(nodeId);
      if (node?.file && node.file !== diagnosticsFile()) {
        setDiagnosticsFile(node.file);
      }
    }
  });
  function navigateToFile(file, line) {
    setCodeFilePath(file);
    setCodePanelOpen(true);
    // Monaco will navigate to line after file loads — simplified
    console.log(`Navigate to ${file}:${line}`);
  }
  const severityColors = {
    error: "text-red-400 bg-red-900/20",
    warning: "text-yellow-400 bg-yellow-900/20",
    info: "text-blue-400 bg-blue-900/20"
  };
  const severityIcons = {
    error: "❌",
    warning: "⚠️",
    info: "ℹ️"
  };
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.firstChild,
      _el$4 = _el$3.nextSibling,
      _el$5 = _el$4.firstChild,
      _el$6 = _el$5.nextSibling,
      _el$7 = _el$2.nextSibling;
    _el$5.$$click = runDiagnostics;
    _$insert(_el$5, _$createComponent(RefreshCw, {
      get classList() {
        return {
          "w-3.5 h-3.5 animate-spin": diagnosticsLoading(),
          "w-3.5 h-3.5": !diagnosticsLoading()
        };
      }
    }));
    _el$6.$$click = () => setDiagnosticsPanelOpen(false);
    _$insert(_el$6, _$createComponent(X, {
      "class": "w-4 h-4"
    }));
    _$insert(_el$, (() => {
      var _c$ = _$memo(() => !!diagnosticsFile());
      return () => _c$() && (() => {
        var _el$8 = _tmpl$2();
        _$insert(_el$8, diagnosticsFile);
        return _el$8;
      })();
    })(), _el$7);
    _$insert(_el$7, (() => {
      var _c$2 = _$memo(() => !!diagnosticsLoading());
      return () => _c$2() ? _tmpl$3() : _$memo(() => diagnosticsItems().length === 0)() ? _tmpl$4() : (() => {
        var _el$1 = _tmpl$5();
        _$insert(_el$1, _$createComponent(For, {
          get each() {
            return diagnosticsItems();
          },
          children: item => (() => {
            var _el$10 = _tmpl$6(),
              _el$11 = _el$10.firstChild,
              _el$12 = _el$11.firstChild,
              _el$13 = _el$12.nextSibling,
              _el$14 = _el$13.firstChild,
              _el$15 = _el$14.nextSibling,
              _el$16 = _el$15.firstChild,
              _el$18 = _el$16.nextSibling,
              _el$17 = _el$18.nextSibling,
              _el$19 = _el$13.nextSibling;
            _el$10.$$click = () => navigateToFile(item.file, item.line);
            _$insert(_el$12, () => severityIcons[item.severity] ?? "•");
            _$insert(_el$14, () => item.message);
            _$insert(_el$15, () => item.file, _el$16);
            _$insert(_el$15, () => item.line, _el$18);
            _$insert(_el$15, () => item.column, null);
            _$insert(_el$15, (() => {
              var _c$3 = _$memo(() => !!item.code);
              return () => _c$3() && ` [${item.code}]`;
            })(), null);
            _$insert(_el$19, () => item.severity);
            _$effect(_p$ => {
              var _v$ = `w-full text-left px-2.5 py-2 rounded-md text-xs border border-border-default hover:border-border-focus transition-colors ${severityColors[item.severity] ?? ""}`,
                _v$2 = {
                  "bg-red-900/30 text-red-300": item.severity === "error",
                  "bg-yellow-900/30 text-yellow-300": item.severity === "warning",
                  "bg-blue-900/30 text-blue-300": item.severity === "info"
                };
              _v$ !== _p$.e && _$className(_el$10, _p$.e = _v$);
              _p$.t = _$classList(_el$19, _v$2, _p$.t);
              return _p$;
            }, {
              e: undefined,
              t: undefined
            });
            return _el$10;
          })()
        }));
        return _el$1;
      })();
    })());
    _$effect(() => _el$5.disabled = diagnosticsLoading());
    return _el$;
  })();
}
_$delegateEvents(["click"]);