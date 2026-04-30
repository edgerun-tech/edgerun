"use client";

import React from "react";
import { motion, AnimatePresence } from "framer-motion";
import { ArrowUp } from "lucide-react";

export const CommandPalette = () => {
  const [visible, setVisible] = React.useState(false);
  const [input, setInput] = React.useState("");
  const [suggestions, setSuggestions] = React.useState<string[]>([]);
  const [showSuggestions, setShowSuggestions] = React.useState(false);
  const textareaRef = React.useRef<HTMLTextAreaElement>(null);

  // 🔥 keyboard control
  React.useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setVisible((v) => !v);
      }

      if (e.key === "/" && !visible) {
        const active = document.activeElement as HTMLElement;
        if (!["INPUT", "TEXTAREA"].includes(active?.tagName)) {
          e.preventDefault();
          setVisible(true);
        }
      }

      if (e.key === "Escape") {
        setVisible(false);
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [visible]);

  // 🔥 autofocus
  React.useEffect(() => {
    if (visible) {
      setTimeout(() => textareaRef.current?.focus(), 50);
    }
  }, [visible]);

  // 🔥 autosuggest logic
  React.useEffect(() => {
    if (input.startsWith("!")) {
      setSuggestions(["!terminal", "!run", "!logs"]);
      setShowSuggestions(true);
    } else if (input.startsWith("/")) {
      setSuggestions(["/email", "/logs", "/dashboard"]);
      setShowSuggestions(true);
    } else {
      setShowSuggestions(false);
    }
  }, [input]);

  // 🔥 command handler
  const handleSubmit = () => {
    if (!input.trim()) return;

    if (input.startsWith("!terminal")) {
      const cmd = input.replace("!terminal", "").trim();
      console.log("EXEC:", cmd);
    }

    setInput("");
    setVisible(false);
  };

  return (
    <>
      <AnimatePresence>
        {visible && (
          <>
            {/* background */}
            <motion.div
              className="fixed inset-0 bg-black/40 backdrop-blur-sm z-40"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
            />

            {/* palette */}
            <motion.div
              initial={{ opacity: 0, y: 20, scale: 0.96 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: 20, scale: 0.96 }}
              transition={{ duration: 0.18 }}
              className="fixed bottom-6 left-1/2 -translate-x-1/2 w-[min(700px,90vw)] z-50"
            >
              <div className="relative bg-[#1F2023] border border-[#444] rounded-2xl shadow-xl p-3">

                {/* suggestions */}
                {showSuggestions && (
                  <div className="absolute bottom-full mb-2 w-full bg-[#1F2023] border border-[#444] rounded-xl shadow-lg overflow-hidden">
                    {suggestions.map((s, i) => (
                      <div
                        key={i}
                        className="px-3 py-2 hover:bg-[#2E3033] cursor-pointer text-sm"
                        onClick={() => {
                          setInput(s + " ");
                          setShowSuggestions(false);
                        }}
                      >
                        {s}
                      </div>
                    ))}
                  </div>
                )}

                {/* input */}
                <textarea
                  ref={textareaRef}
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !e.shiftKey) {
                      e.preventDefault();
                      handleSubmit();
                    }
                  }}
                  placeholder="Type command..."
                  className="w-full bg-transparent text-white outline-none resize-none text-sm px-2 py-2"
                  rows={1}
                />

                {/* actions */}
                <div className="flex justify-end pt-2">
                  <button
                    onClick={handleSubmit}
                    className="h-8 w-8 flex items-center justify-center rounded-full bg-white text-black hover:bg-white/80 transition"
                  >
                    <ArrowUp className="h-4 w-4" />
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