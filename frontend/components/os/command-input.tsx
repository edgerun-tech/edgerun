"use client";

import React from "react";
import { motion, AnimatePresence } from "framer-motion";
import { X, Send, Loader2, Sparkles } from "lucide-react";
import { launchAppById } from "@/stores/app-launcher";
import { buildRepoContext, fileSystemStore } from "@/stores/file-system-store";

function parseActions(text: string): { text: string; actions: Array<{ type: string; payload: unknown }> } {
  const actions: Array<{ type: string; payload: unknown }> = [];
  const parts = text.split("[action:");
  for (let i = 1; i < parts.length; i++) {
    const endIdx = parts[i].indexOf("]");
    if (endIdx > 0) {
      const content = parts[i].slice(0, endIdx);
      const colonIdx = content.indexOf(":");
      if (colonIdx > 0) {
        actions.push({ type: content.slice(0, colonIdx), payload: content.slice(colonIdx + 1) });
      }
    }
  }
  return { text: text.replace(/\[action:[^\]]+\]/g, "").trim(), actions };
}

function executeActions(actions: Array<{ type: string; payload: unknown }>) {
  for (const action of actions) {
    switch (action.type) {
      case "open-app":
        launchAppById(action.payload as string, null);
        break;
      case "show-stats":
        launchAppById("resource-monitor", null);
        break;
      case "open-terminal":
        launchAppById("terminal", null);
        break;
      case "open-files":
        launchAppById("file-browser", null);
        break;
    }
  }
}

interface Message {
  id: string;
  role: "user" | "assistant" | "tool";
  content: string;
  timestamp: Date;
}

export const CommandPalette = () => {
  const [open, setOpen] = React.useState(false);
  const [input, setInput] = React.useState("");
  const [loading, setLoading] = React.useState(false);
  const [messages, setMessages] = React.useState<Message[]>([]);
  const [serverOk, setServerOk] = React.useState<boolean | null>(null);
  const inputRef = React.useRef<HTMLTextAreaElement>(null);
  const bottomRef = React.useRef<HTMLDivElement>(null);

  React.useEffect(() => {
    setServerOk(true);
  }, []);

  React.useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  React.useEffect(() => {
    if (open) setTimeout(() => inputRef.current?.focus(), 50);
  }, [open]);

  React.useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setOpen((v) => !v);
      }
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const sendMessage = async () => {
    if (!input.trim()) return;
    if (serverOk === false) {
      setMessages((prev) => [...prev, {
        id: `m-${Date.now()}`,
        role: "assistant",
        content: "OpenCode server not running.",
        timestamp: new Date(),
      }]);
      return;
    }

    const userMsg = input.trim();
    setInput("");
    setMessages((prev) => [...prev, { id: `u-${Date.now()}`, role: "user", content: userMsg, timestamp: new Date() }]);
    setLoading(true);

    try {
      const repoContext = fileSystemStore.get().rootHandle ? await buildRepoContext() : "";
      const chatRes = await fetch("/api/chat", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          messages: messages.map(m => ({ role: m.role, content: m.content })).concat({ role: "user", content: userMsg }),
          repoContext,
        }),
      });

      if (!chatRes.ok) throw new Error(`HTTP ${chatRes.status}`);
      const data = await chatRes.json();
      const { text, actions } = parseActions(data.text || "No response");
      executeActions(actions);
      setMessages((prev) => [...prev, { id: `a-${Date.now()}`, role: "assistant", content: text, timestamp: new Date() }]);
    } catch (e) {
      setMessages((prev) => [...prev, {
        id: `e-${Date.now()}`,
        role: "assistant",
        content: `Error: ${e instanceof Error ? e.message : String(e)}`,
        timestamp: new Date(),
      }]);
    } finally {
      setLoading(false);
    }
  };

  const renderContent = (content: string) => {
    return content.split("\n").map((line, i) => line ? (
      <div key={i} className="text-foreground/90 leading-relaxed">{line}</div>
    ) : (
      <div key={i} className="h-2" />
    ));
  };

  return (
    <AnimatePresence>
      {open && (
        <>
          <motion.div
            className="fixed inset-0 bg-black/20 z-40"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={() => setOpen(false)}
          />
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 20 }}
            className="fixed bottom-16 right-4 w-[420px] max-h-[65vh] flex flex-col rounded-2xl shadow-2xl z-50 border border-border/50 overflow-hidden"
            style={{ background: "var(--window-bg)" }}
          >
            <div className="flex items-center justify-between px-4 py-3 border-b border-border/30">
              <div className="flex items-center gap-2">
                <Sparkles className="h-4 w-4 text-primary" />
                <span className="text-sm font-medium">Assistant</span>
                <span className={`text-xs ${serverOk ? "text-green-500" : "text-red-500"}`}>{serverOk ? "●" : "○"}</span>
              </div>
              <button onClick={() => setOpen(false)} className="p-1 rounded hover:bg-secondary/50">
                <X className="h-4 w-4 text-muted-foreground" />
              </button>
            </div>

            <div className="flex-1 overflow-y-auto px-4 py-2 space-y-1">
              {messages.length === 0 && !loading && (
                <p className="text-center text-xs text-muted-foreground py-8">Press Ctrl+K to open this assistant.</p>
              )}
              {messages.map((msg) => (
                <div key={msg.id} className={`text-sm ${msg.role === "user" ? "text-right" : "text-left"}`}>
                  <div className={`inline-block max-w-[85%] px-3 py-2 rounded-xl text-left ${msg.role === "user" ? "bg-primary text-primary-foreground" : "text-foreground/90"}`}>
                    {renderContent(msg.content)}
                  </div>
                </div>
              ))}
              {loading && (
                <div className="flex items-center gap-2 text-muted-foreground">
                  <Loader2 className="h-3 w-3 animate-spin" />
                  <span className="text-xs">Thinking...</span>
                </div>
              )}
              <div ref={bottomRef} />
            </div>

            <div className="p-3 border-t border-border/30">
              <div className="flex items-center gap-2 bg-secondary/30 rounded-lg px-3 py-2">
                <textarea
                  ref={inputRef}
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !e.shiftKey) {
                      e.preventDefault();
                      sendMessage();
                    }
                  }}
                  placeholder="Ask anything..."
                  className="flex-1 bg-transparent text-sm outline-none resize-none"
                  rows={1}
                  disabled={loading}
                />
                <button
                  onClick={sendMessage}
                  disabled={!input.trim() || loading}
                  className="p-1.5 rounded-md bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
                >
                  {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : <Send className="h-4 w-4" />}
                </button>
              </div>
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
};