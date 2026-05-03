"use client";

import React from "react";
import { AnimatePresence, motion } from "framer-motion";
import { 
  Copy, Check, Terminal, FolderOpen, X, Maximize2, RefreshCw,
  Settings, Trash2, Download, Upload, FileText, Sparkles, Plus
} from "lucide-react";

type MenuItem = {
  label: string;
  icon?: React.ReactNode;
  action: () => void;
  danger?: boolean;
};

export const ContextMenuProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [visible, setVisible] = React.useState(false);
  const [pos, setPos] = React.useState({ x: 0, y: 0 });
  const [items, setItems] = React.useState<MenuItem[]>([]);
  const [copied, setCopied] = React.useState(false);

  const handleCopy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch (e) {
      console.error("Copy failed:", e);
    }
  };

  React.useEffect(() => {
    const handler = (e: MouseEvent) => {
      e.preventDefault();

      const target = e.target as HTMLElement;
      let menu: MenuItem[] = [];

      // Get selected text for copy
      const selection = window.getSelection()?.toString();

      if (selection && selection.length > 0) {
        menu = [
          {
            label: copied ? "Copied!" : "Copy",
            icon: copied ? <Check className="h-4 w-4 text-green-400" /> : <Copy className="h-4 w-4" />,
            action: () => handleCopy(selection)
          },
        ];
      } else if (target.closest("[data-terminal]")) {
        menu = [
          { label: "Clear Terminal", icon: <Trash2 className="h-4 w-4" />, action: () => console.log("clear terminal") },
          { label: "Copy Output", icon: <Copy className="h-4 w-4" />, action: () => console.log("copy terminal") },
          { label: "New Tab", icon: <Plus className="h-4 w-4" />, action: () => console.log("new tab") },
        ];
      } else if (target.closest("[data-app]")) {
        menu = [
          { label: "Maximize", icon: <Maximize2 className="h-4 w-4" />, action: () => console.log("maximize") },
          { label: "Minimize", icon: <X className="h-4 w-4" />, action: () => console.log("minimize") },
          { label: "Restart", icon: <RefreshCw className="h-4 w-4" />, action: () => console.log("restart") },
        ];
      } else if (target.closest("[data-file]")) {
        menu = [
          { label: "Open", icon: <FileText className="h-4 w-4" />, action: () => console.log("open file") },
          { label: "Copy Path", icon: <Copy className="h-4 w-4" />, action: () => console.log("copy path") },
          { label: "Rename", icon: <FileText className="h-4 w-4" />, action: () => console.log("rename") },
        ];
      } else if (target.closest("[data-message]")) {
        // AI message context menu
        const msgContent = target.closest("[data-message]")?.textContent;
        menu = [
          {
            label: copied ? "Copied!" : "Copy",
            icon: copied ? <Check className="h-4 w-4 text-green-400" /> : <Copy className="h-4 w-4" />,
            action: () => msgContent && handleCopy(msgContent)
          },
          { label: "Regenerate", icon: <RefreshCw className="h-4 w-4" />, action: () => console.log("regenerate") },
        ];
      } else {
        // Desktop context
        menu = [
          { label: "Open Terminal", icon: <Terminal className="h-4 w-4" />, action: () => console.log("terminal") },
          { label: "Open Folder", icon: <FolderOpen className="h-4 w-4" />, action: () => console.log("folder") },
          { label: "AI Assistant", icon: <Sparkles className="h-4 w-4 text-primary" />, action: () => console.log("ai") },
        ];
      }

      setItems(menu);

      // Calculate position to keep menu within viewport
      const menuWidth = 200; // min-w-[180px] + padding
      const menuHeight = menu.length * 40 + 16; // estimated item height + padding
      const padding = 8;

      let x = e.clientX;
      let y = e.clientY;

      // Adjust horizontal position if menu would overflow right
      if (x + menuWidth > window.innerWidth - padding) {
        x = Math.max(padding, window.innerWidth - menuWidth - padding);
      }

      // Adjust vertical position if menu would overflow bottom
      if (y + menuHeight > window.innerHeight - padding) {
        y = Math.max(padding, window.innerHeight - menuHeight - padding);
      }

      setPos({ x, y });
      setVisible(true);
    };

    const close = () => setVisible(false);

    window.addEventListener("contextmenu", handler);
    window.addEventListener("click", close);
    window.addEventListener("keydown", (e) => {
      if (e.key === "Escape") close();
    });

    return () => {
      window.removeEventListener("contextmenu", handler);
      window.removeEventListener("click", close);
    };
  }, [copied]);

  return (
    <>
      {children}

      <AnimatePresence>
        {visible && (
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            className="fixed z-[9999] bg-[#1F2023] border border-[#444] rounded-xl shadow-xl py-1 min-w-[180px] overflow-hidden"
            style={{ top: pos.y + 'px', left: pos.x + 'px' }}
          >
            {items.map((item, i) => (
              <button
                key={i}
                onClick={() => {
                  item.action();
                  setVisible(false);
                }}
                className={`w-full px-3 py-2 text-sm text-white hover:bg-[#2E3033] cursor-pointer flex items-center gap-3 transition-colors ${
                  item.danger ? "text-red-400 hover:bg-red-500/20" : ""
                }`}
              >
                {item.icon && <span className="opacity-70">{item.icon}</span>}
                {item.label}
              </button>
            ))}
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
};