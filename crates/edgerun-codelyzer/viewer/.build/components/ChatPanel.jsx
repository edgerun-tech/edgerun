import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { classList as _$classList } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { memo as _$memo } from "solid-js/web";
import { use as _$use } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div class="panel shrink-0"style=width:420px><div class=panel-header><span class=panel-title>💬 Qwen Chat</span><div class="flex gap-1.5"><button class="btn px-2 py-0.5 text-xs"title="Include selected node as context"></button><button class="btn px-2 py-0.5 text-xs"></button></div></div><div class="flex-1 overflow-y-auto p-3 space-y-3"></div><div class="border-t border-border-default p-2.5 bg-bg-tertiary"><div class="flex gap-2 items-end"><textarea class="input flex-1 resize-none max-h-[120px] leading-relaxed"placeholder="Ask about the codebase…"rows=1></textarea><button class="btn btn-primary px-3 py-2">`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div>`),
  _tmpl$3 = /*#__PURE__*/_$template(`<span class=whitespace-pre-wrap>`),
  _tmpl$4 = /*#__PURE__*/_$template(`<div class="mt-2 space-y-2">`),
  _tmpl$5 = /*#__PURE__*/_$template(`<div class="self-start bg-bg-elevated text-text-muted text-xs px-3 py-2 rounded-lg border border-border-default rounded-bl-sm"><span class=italic>Thinking…`),
  _tmpl$6 = /*#__PURE__*/_$template(`<div class="mb-1.5 px-2 py-1 text-[11px] bg-accent-blue-bg border border-accent-blue rounded-md text-blue-300 flex items-center justify-between"><span>Context: <strong></strong> (<!>)</span><button class="text-accent-red hover:text-red-400 text-xs">×`),
  _tmpl$7 = /*#__PURE__*/_$template(`<div class="bg-bg-elevated border border-border-default rounded-lg overflow-hidden"><button class="flex items-center gap-1.5 px-3 py-2 bg-bg-active text-xs cursor-pointer hover:bg-bg-hover w-full"><span>🔧</span><strong></strong><span class="ml-auto text-text-muted text-[10px]">`),
  _tmpl$8 = /*#__PURE__*/_$template(`<div class="p-2.5 space-y-2"><div><div class="text-[10px] text-text-muted uppercase mb-1">Arguments:</div><pre class="bg-bg-primary border border-border-default rounded-sm p-1.5 text-[11px] font-mono overflow-x-auto text-blue-300 max-h-48 overflow-y-auto whitespace-pre-wrap break-all">`),
  _tmpl$9 = /*#__PURE__*/_$template(`<div><div class="text-[10px] text-text-muted uppercase mb-1">Output:</div><pre class="bg-bg-primary border border-border-default rounded-sm p-1.5 text-[11px] font-mono overflow-x-auto text-blue-300 max-h-48 overflow-y-auto whitespace-pre-wrap break-all">`);
/**
 * Chat panel with markdown rendering and tool call display.
 */
import { createSignal, For, onMount } from "solid-js";
import { marked } from "marked";
import DOMPurify from "dompurify";
import { chatMessages, setChatMessages, chatBusy, setChatBusy, chatContextNode, setChatContextNode, chatSessionId, setChatSessionId, chatPanelOpen, setChatPanelOpen, focusedNodeId, nodesMap } from "../store.js";
import { sendChatMessage } from "../ws.js";
import { X, Paperclip, Send } from "lucide-solid";
export function ChatPanel() {
  let messagesRef;
  let inputRef;
  const [inputText, setInputText] = createSignal("");
  onMount(() => {
    inputRef?.focus();
  });
  function scrollToBottom() {
    if (messagesRef) {
      messagesRef.scrollTop = messagesRef.scrollHeight;
    }
  }
  function handleSend() {
    const text = inputText().trim();
    if (!text || chatBusy()) return;
    setInputText("");
    if (inputRef) {
      inputRef.style.height = "auto";
    }
    void sendChatMessage(text, chatContextNode(), chatSessionId(), msg => {
      setChatMessages(prev => [...prev, msg]);
      scrollToBottom();
    });
    scrollToBottom();
  }
  function handleKeyDown(e) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }
  function handleInput(e) {
    const target = e.currentTarget;
    setInputText(target.value);
    target.style.height = "auto";
    target.style.height = `${Math.min(target.scrollHeight, 120)}px`;
  }
  function attachContext() {
    const id = focusedNodeId();
    if (id) {
      const node = nodesMap().get(id);
      if (node) {
        setChatContextNode(node);
        return;
      }
    }
    setChatContextNode(null);
  }
  function removeContext() {
    setChatContextNode(null);
  }
  function extractPathFromArgs(argsStr) {
    const m = argsStr.match(/path="([^"]*)"/);
    return m?.[1] ?? null;
  }
  function toolDisplayName(name) {
    return name.replace(/_/g, " ").replace(/\b\w/g, c => c.toUpperCase());
  }
  function escapeHtml(s) {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
  }
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.firstChild,
      _el$4 = _el$3.nextSibling,
      _el$5 = _el$4.firstChild,
      _el$6 = _el$5.nextSibling,
      _el$7 = _el$2.nextSibling,
      _el$8 = _el$7.nextSibling,
      _el$9 = _el$8.firstChild,
      _el$0 = _el$9.firstChild,
      _el$1 = _el$0.nextSibling;
    _el$5.$$click = attachContext;
    _$insert(_el$5, _$createComponent(Paperclip, {
      "class": "w-3.5 h-3.5"
    }));
    _el$6.$$click = () => setChatPanelOpen(false);
    _$insert(_el$6, _$createComponent(X, {
      "class": "w-4 h-4"
    }));
    var _ref$ = messagesRef;
    typeof _ref$ === "function" ? _$use(_ref$, _el$7) : messagesRef = _el$7;
    _$insert(_el$7, _$createComponent(For, {
      get each() {
        return chatMessages();
      },
      children: msg => (() => {
        var _el$10 = _tmpl$2();
        _$insert(_el$10, (() => {
          var _c$3 = _$memo(() => msg.role === "assistant");
          return () => _c$3() ? // eslint-disable-next-line solid/no-innerhtml
          (() => {
            var _el$11 = _tmpl$2();
            _$effect(() => _el$11.innerHTML = DOMPurify.sanitize(marked.parse(msg.text)));
            return _el$11;
          })() : (() => {
            var _el$12 = _tmpl$3();
            _$insert(_el$12, () => msg.text);
            return _el$12;
          })();
        })(), null);
        _$insert(_el$10, (() => {
          var _c$4 = _$memo(() => !!(msg.tools && msg.tools.length > 0));
          return () => _c$4() && (() => {
            var _el$13 = _tmpl$4();
            _$insert(_el$13, _$createComponent(For, {
              get each() {
                return msg.tools;
              },
              children: tool => _$createComponent(ToolCallCard, {
                get name() {
                  return tool.name;
                },
                get args() {
                  return tool.args;
                },
                get output() {
                  return tool.output;
                },
                get content() {
                  return tool.content;
                },
                get filePath() {
                  return extractPathFromArgs(tool.args);
                }
              })
            }));
            return _el$13;
          })();
        })(), null);
        _$effect(_$p => _$classList(_el$10, {
          "max-w-full rounded-lg text-xs leading-relaxed break-words": true,
          "self-end bg-accent-blue-bg text-text-heading rounded-br-sm px-3 py-2": msg.role === "user",
          "self-start bg-bg-elevated text-text-primary border border-border-default rounded-bl-sm px-3 py-2": msg.role === "assistant",
          "self-start bg-red-900/30 text-accent-red border border-red-800/50 px-3 py-2 rounded-lg": msg.role === "error",
          "self-center bg-transparent text-text-muted italic text-[11px] px-2 py-1": msg.role === "system"
        }, _$p));
        return _el$10;
      })()
    }), null);
    _$insert(_el$7, (() => {
      var _c$ = _$memo(() => !!chatBusy());
      return () => _c$() && _tmpl$5();
    })(), null);
    _$insert(_el$8, (() => {
      var _c$2 = _$memo(() => !!chatContextNode());
      return () => _c$2() && (() => {
        var _el$15 = _tmpl$6(),
          _el$16 = _el$15.firstChild,
          _el$17 = _el$16.firstChild,
          _el$18 = _el$17.nextSibling,
          _el$19 = _el$18.nextSibling,
          _el$21 = _el$19.nextSibling,
          _el$20 = _el$21.nextSibling,
          _el$22 = _el$16.nextSibling;
        _$insert(_el$18, () => chatContextNode().name);
        _$insert(_el$16, () => chatContextNode().file ?? "", _el$21);
        _el$22.$$click = removeContext;
        return _el$15;
      })();
    })(), _el$9);
    _el$0.$$keydown = handleKeyDown;
    _el$0.$$input = handleInput;
    var _ref$2 = inputRef;
    typeof _ref$2 === "function" ? _$use(_ref$2, _el$0) : inputRef = _el$0;
    _el$1.$$click = handleSend;
    _$insert(_el$1, _$createComponent(Send, {
      "class": "w-3.5 h-3.5"
    }));
    _$effect(() => _el$1.disabled = chatBusy() || !inputText().trim());
    _$effect(() => _el$0.value = inputText());
    return _el$;
  })();
}
function ToolCallCard(props) {
  const [expanded, setExpanded] = createSignal(false);
  return (() => {
    var _el$23 = _tmpl$7(),
      _el$24 = _el$23.firstChild,
      _el$25 = _el$24.firstChild,
      _el$26 = _el$25.nextSibling,
      _el$27 = _el$26.nextSibling;
    _el$24.$$click = () => setExpanded(!expanded());
    _$insert(_el$26, () => toolDisplayNameSimple(props.name));
    _$insert(_el$27, () => expanded() ? "▾" : "▸");
    _$insert(_el$23, (() => {
      var _c$5 = _$memo(() => !!expanded());
      return () => _c$5() && (() => {
        var _el$28 = _tmpl$8(),
          _el$29 = _el$28.firstChild,
          _el$30 = _el$29.firstChild,
          _el$31 = _el$30.nextSibling;
        _$insert(_el$31, () => props.args);
        _$insert(_el$28, (() => {
          var _c$6 = _$memo(() => !!props.output);
          return () => _c$6() && (() => {
            var _el$32 = _tmpl$9(),
              _el$33 = _el$32.firstChild,
              _el$34 = _el$33.nextSibling;
            _$insert(_el$34, () => props.output.substring(0, 2000));
            return _el$32;
          })();
        })(), null);
        return _el$28;
      })();
    })(), null);
    return _el$23;
  })();
}
function toolDisplayNameSimple(name) {
  return name.replace(/_/g, " ").replace(/\b\w/g, c => c.toUpperCase());
}
_$delegateEvents(["click", "input", "keydown"]);