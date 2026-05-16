/** @jsxImportSource solid-js **/
/**
 * Chat panel with markdown rendering and tool call display.
 */
import { createSignal, For, onMount } from "solid-js";
import { marked } from "marked";
import DOMPurify from "dompurify";
import {
  chatMessages,
  setChatMessages,
  chatBusy,
  setChatBusy,
  chatContextNode,
  setChatContextNode,
  chatSessionId,
  setChatSessionId,
  chatPanelOpen,
  setChatPanelOpen,
  focusedNodeId,
  nodesMap,
} from "../store.js";
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

    void sendChatMessage(text, chatContextNode(), chatSessionId(), (msg) => {
      setChatMessages((prev) => [...prev, msg]);
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
    return name.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
  }

  function escapeHtml(s) {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
  }

  return (
    <div class="panel shrink-0" style={{ width: "420px" }}>
      {/* Header */}
      <div class="panel-header">
        <span class="panel-title">💬 Qwen Chat</span>
        <div class="flex gap-1.5">
          <button
            class="btn px-2 py-0.5 text-xs"
            title="Include selected node as context"
            onClick={attachContext}
          >
            <Paperclip class="w-3.5 h-3.5" />
          </button>
          <button class="btn px-2 py-0.5 text-xs" onClick={() => setChatPanelOpen(false)}>
            <X class="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Messages */}
      <div ref={messagesRef} class="flex-1 overflow-y-auto p-3 space-y-3">
        <For each={chatMessages()}>
          {(msg) => (
            <div
              classList={{
                "max-w-full rounded-lg text-xs leading-relaxed break-words": true,
                "self-end bg-accent-blue-bg text-text-heading rounded-br-sm px-3 py-2": msg.role === "user",
                "self-start bg-bg-elevated text-text-primary border border-border-default rounded-bl-sm px-3 py-2": msg.role === "assistant",
                "self-start bg-red-900/30 text-accent-red border border-red-800/50 px-3 py-2 rounded-lg": msg.role === "error",
                "self-center bg-transparent text-text-muted italic text-[11px] px-2 py-1": msg.role === "system",
              }}
            >
              {msg.role === "assistant" ? (
                // eslint-disable-next-line solid/no-innerhtml
                <div innerHTML={DOMPurify.sanitize(marked.parse(msg.text))} />
              ) : (
                <span class="whitespace-pre-wrap">{msg.text}</span>
              )}

              {/* Tool calls */}
              {msg.tools && msg.tools.length > 0 && (
                <div class="mt-2 space-y-2">
                  <For each={msg.tools}>
                    {(tool) => (
                      <ToolCallCard
                        name={tool.name}
                        args={tool.args}
                        output={tool.output}
                        content={tool.content}
                        filePath={extractPathFromArgs(tool.args)}
                      />
                    )}
                  </For>
                </div>
              )}
            </div>
          )}
        </For>
        {chatBusy() && (
          <div class="self-start bg-bg-elevated text-text-muted text-xs px-3 py-2 rounded-lg border border-border-default rounded-bl-sm">
            <span class="italic">Thinking…</span>
          </div>
        )}
      </div>

      {/* Input area */}
      <div class="border-t border-border-default p-2.5 bg-bg-tertiary">
        {chatContextNode() && (
          <div class="mb-1.5 px-2 py-1 text-[11px] bg-accent-blue-bg border border-accent-blue rounded-md text-blue-300 flex items-center justify-between">
            <span>
              Context: <strong>{chatContextNode().name}</strong> ({chatContextNode().file ?? ""})
            </span>
            <button class="text-accent-red hover:text-red-400 text-xs" onClick={removeContext}>
              ×
            </button>
          </div>
        )}
        <div class="flex gap-2 items-end">
          <textarea
            ref={inputRef}
            class="input flex-1 resize-none max-h-[120px] leading-relaxed"
            placeholder="Ask about the codebase…"
            rows={1}
            value={inputText()}
            onInput={handleInput}
            onKeyDown={handleKeyDown}
          />
          <button
            class="btn btn-primary px-3 py-2"
            disabled={chatBusy() || !inputText().trim()}
            onClick={handleSend}
          >
            <Send class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </div>
  );
}

function ToolCallCard(props) {
  const [expanded, setExpanded] = createSignal(false);

  return (
    <div class="bg-bg-elevated border border-border-default rounded-lg overflow-hidden">
      <button
        class="flex items-center gap-1.5 px-3 py-2 bg-bg-active text-xs cursor-pointer hover:bg-bg-hover w-full"
        onClick={() => setExpanded(!expanded())}
      >
        <span>🔧</span>
        <strong>{toolDisplayNameSimple(props.name)}</strong>
        <span class="ml-auto text-text-muted text-[10px]">{expanded() ? "▾" : "▸"}</span>
      </button>
      {expanded() && (
        <div class="p-2.5 space-y-2">
          <div>
            <div class="text-[10px] text-text-muted uppercase mb-1">Arguments:</div>
            <pre class="bg-bg-primary border border-border-default rounded-sm p-1.5 text-[11px] font-mono overflow-x-auto text-blue-300 max-h-48 overflow-y-auto whitespace-pre-wrap break-all">
              {props.args}
            </pre>
          </div>
          {props.output && (
            <div>
              <div class="text-[10px] text-text-muted uppercase mb-1">Output:</div>
              <pre class="bg-bg-primary border border-border-default rounded-sm p-1.5 text-[11px] font-mono overflow-x-auto text-blue-300 max-h-48 overflow-y-auto whitespace-pre-wrap break-all">
                {props.output.substring(0, 2000)}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function toolDisplayNameSimple(name) {
  return name.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}
