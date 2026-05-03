"use client";

import React from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Bot, X, Send, Loader2, Sparkles, Terminal, FolderOpen, Activity, Wifi, WifiOff } from "lucide-react";
import { launchAppById } from "@/stores/app-launcher";
import { buildRepoContext, fileSystemStore } from "@/stores/file-system-store";

// Parse special actions from agent response
function parseActions(text: string): { text: string; actions: Array<{ type: string; payload: unknown }> } {
  const actions: Array<{ type: string; payload: unknown }> = [];
  
  // Simple pattern: look for action markers
  const parts = text.split("[action:");
  for (let i = 1; i < parts.length; i++) {
    const endIdx = parts[i].indexOf("]");
    if (endIdx > 0) {
      const content = parts[i].slice(0, endIdx);
      const colonIdx = content.indexOf(":");
      if (colonIdx > 0) {
        const actionType = content.slice(0, colonIdx);
        const actionPayload = content.slice(colonIdx + 1);
        actions.push({ type: actionType, payload: actionPayload });
      }
    }
  }
  
  // Remove action markers from display text
  const cleanText = text.replace(/\[action:[^\]]+\]/g, "").trim();
  return { text: cleanText, actions };
}

// Execute platform actions
function executeActions(actions: Array<{ type: string; payload: unknown }>) {
  for (const action of actions) {
    switch (action.type) {
      case "open-app":
        const appId = action.payload as string;
        launchAppById(appId, null);
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
    // Just set to true - if API fails it will show in the chat error
    setServerOk(true);
  }, []);

  // Auto-scroll
  React.useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // Focus input
  React.useEffect(() => {
    if (open) setTimeout(() => inputRef.current?.focus(), 50);
  }, [open]);

  // Keyboard shortcuts
  React.useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setOpen((v) => !v);
      }
      if (e.key === "/" && !open) {
        const tag = document.activeElement?.tagName;
        if (tag !== "INPUT" && tag !== "TEXTAREA") {
          e.preventDefault();
          setOpen(true);
        }
      }
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open]);

  const sendMessage = async () => {
    if (!input.trim()) return;
    if (serverOk === false) {
      setMessages((prev) => [...prev, {
        id: `m-${Date.now()}`,
        role: "assistant",
        content: "⚠ OpenCode server not running. Start with:\n```bash\nopencode serve --port 4096 --cors http://127.0.0.1:3000\n```",
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

      // Use local API - calls OpenCode Zen directly via SDK (no external server needed)
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
      const rawText = data.text || "No response";
      
      // Parse actions from response
      const { text, actions } = parseActions(rawText);
      executeActions(actions);

      setMessages((prev) => [...prev, {
        id: `a-${Date.now()}`,
        role: "assistant",
        content: text,
        timestamp: new Date(),
      }]);
    } catch (e) {
      setMessages((prev) => [...prev, {
        id: `e-${Date.now()}`,
        role: "assistant",
        content: `Error: ${e instanceof Error ? e.message : String(e)}. Make sure OPENCODE_API_KEY is set in .env.local`,
        timestamp: new Date(),
      }]);
    } finally {
      setLoading(false);
    }
  };

  // Render message content with inline styles
  const renderContent = (content: string) => {
    // Simple inline rendering - blend text without containers
    const lines = content.split("\n");
    return lines.map((line, i) => {
      // Code blocks
      if (line.startsWith("```")) {
        return <div key={i} className="font-mono text-xs bg-secondary/50 rounded px-1 mx-1 my-0.5 inline" />;
      }
      // Bold
      if (line.startsWith("**")) {
        return <div key={i} className="font-medium text-foreground" />;
      }
      return line ? (
        <div key={i} className="text-foreground/90 leading-relaxed">{line}</div>
      ) : (
        <div key={i} className="h-2" />
      );
    });
  };

  return (
    <>
      {/* Floating button */}
      <AnimatePresence>
        {!open && (
          <motion.button
            initial={{ scale: 0, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            exit={{ scale: 0, opacity: 0 }}
            onClick={() => setOpen(true)}
            className="fixed bottom-4 right-4 h-12 w-12 rounded-full bg-primary text-primary-foreground shadow-lg hover:bg-primary/90 flex items-center justify-center z-50 transition"
          >
            <Sparkles className="h-5 w-5" />
            {serverOk === false && (
              <span className="absolute -top-1 -right-1 h-3 w-3 rounded-full bg-red-500 border-2 border-background" />
            )}
            {serverOk === true && (
              <span className="absolute -top-1 -right-1 h-3 w-3 rounded-full bg-green-500 border-2 border-background" />
            )}
          </motion.button>
        )}
      </AnimatePresence>

      {/* Chat panel */}
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
              {/* Header */}
              <div className="flex items-center justify-between px-4 py-3 border-b border-border/30">
                <div className="flex items-center gap-2">
                  <Sparkles className="h-4 w-4 text-primary" />
                  <span className="text-sm font-medium">Assistant</span>
                  <span className={`text-xs ${serverOk ? "text-green-500" : "text-red-500"}`}>
                    {serverOk ? "●" : "○"}
                  </span>
                </div>
                <button onClick={() => setOpen(false)} className="p-1 rounded hover:bg-secondary/50">
                  <X className="h-4 w-4 text-muted-foreground" />
                </button>
              </div>

              {/* Messages */}
              <div className="flex-1 overflow-y-auto px-4 py-2 space-y-1">
                {messages.length === 0 && !loading && (
                  <p className="text-center text-xs text-muted-foreground py-8">
                    Press Enter to chat with OpenCode
                  </p>
                )}
                {messages.map((msg) => (
                  <div
                    key={msg.id}
                    className={`text-sm ${
                      msg.role === "user" ? "text-right" : "text-left"
                    }`}
                  >
                    <div
                      className={`inline-block max-w-[85%] px-3 py-2 rounded-xl text-left ${
                        msg.role === "user"
                          ? "bg-primary text-primary-foreground"
                          : "text-foreground/90"
                      }`}
                    >
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

              {/* Input */}
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
                    placeholder="AskAnything..."
                    className="flex-1 bg-transparent text-sm outline-none resize-none"
                    rows={1}
                    disabled={loading}
                  />
                  <button
                    onClick={sendMessage}
                    disabled={!input.trim() || loading}
                    className="p-1.5 rounded-md bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
                  >
                    {loading ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <Send className="h-4 w-4" />
                    )}
                  </button>
                </div>
              </div>
            </motion.div>
          </>
        )}
      </AnimatePresence>
    </>
  );
};