"use client";

import React from "react";
import { AnimatePresence, motion } from "framer-motion";

type MenuItem = {
  label: string;
  action: () => void;
};

export const ContextMenuProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [visible, setVisible] = React.useState(false);
  const [pos, setPos] = React.useState({ x: 0, y: 0 });
  const [items, setItems] = React.useState<MenuItem[]>([]);

  React.useEffect(() => {
    const handler = (e: MouseEvent) => {
      e.preventDefault();

      // 🔥 dynamic context detection
      const target = e.target as HTMLElement;

      let menu: MenuItem[] = [];

      if (target.closest("[data-app]")) {
        menu = [
          { label: "Open", action: () => console.log("open app") },
          { label: "Close", action: () => console.log("close") },
        ];
      } else if (target.closest("[data-terminal]")) {
        menu = [
          { label: "Copy", action: () => console.log("copy") },
          { label: "Clear", action: () => console.log("clear") },
        ];
      } else {
        menu = [
          { label: "New Window", action: () => console.log("new window") },
          { label: "Open Terminal", action: () => console.log("terminal") },
        ];
      }

      setItems(menu);
      setPos({ x: e.clientX, y: e.clientY });
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
  }, []);

  return (
    <>
      {children}

      <AnimatePresence>
        {visible && (
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            className="fixed z-[9999] bg-[#1F2023] border border-[#444] rounded-xl shadow-xl py-1 min-w-[180px]"
            style={{ top: pos.y, left: pos.x }}
          >
            {items.map((item, i) => (
              <div
                key={i}
                onClick={() => {
                  item.action();
                  setVisible(false);
                }}
                className="px-3 py-2 text-sm text-white hover:bg-[#2E3033] cursor-pointer"
              >
                {item.label}
              </div>
            ))}
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
};