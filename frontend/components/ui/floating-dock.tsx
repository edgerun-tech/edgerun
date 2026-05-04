"use client";
import { cn } from "@/lib/utils";
import { IconLayoutNavbarCollapse } from "@tabler/icons-react";
import {
  AnimatePresence,
  motion,
  useMotionValue,
  useSpring,
  useTransform,
} from "motion/react";
import type { MotionValue } from "motion/react";

import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";

type FloatingDockItem = {
  title: string;
  icon: ReactNode;
  href?: string;
  onClick?: () => void;
  kind?: "app" | "person" | "trigger";
  subtitle?: string;
};

type FloatingDockContext = {
  mode?: "apps" | "chat-heads" | "triggers";
  label?: string;
  items?: FloatingDockItem[];
};

export const FloatingDock = ({
  items,
  context,
  desktopClassName,
  mobileClassName,
  onCommandSubmit,
}: {
  items: FloatingDockItem[];
  context?: FloatingDockContext;
  desktopClassName?: string;
  mobileClassName?: string;
  onCommandSubmit?: (command: string) => void;
}) => {
  const visibleItems = context?.items?.length ? context.items : items;

  return (
    <>
      <FloatingDockDesktop
        items={visibleItems}
        context={context}
        className={desktopClassName}
        onCommandSubmit={onCommandSubmit}
      />
      <FloatingDockMobile items={visibleItems} className={mobileClassName} />
    </>
  );
};

const FloatingDockMobile = ({
  items,
  className,
}: {
  items: FloatingDockItem[];
  className?: string;
}) => {
  const [open, setOpen] = useState(false);
  return (
    <div className={cn("relative block md:hidden", className)}>
      <AnimatePresence>
        {open && (
          <motion.div
            layoutId="nav"
            className="absolute bottom-full mb-2 flex flex-col gap-2"
          >
            {items.map((item, idx) => (
              <motion.div
                key={item.title}
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: 10 }}
                transition={{ delay: (items.length - 1 - idx) * 0.05 }}
              >
                <button
                  type="button"
                  onClick={() => {
                    item.onClick?.();
                    setOpen(false);
                  }}
                  className="flex h-10 w-10 items-center justify-center rounded-full border border-border bg-card hover:bg-accent"
                  aria-label={item.title}
                  title={item.title}
                >
                  <div className="h-4 w-4">{item.icon}</div>
                </button>
              </motion.div>
            ))}
          </motion.div>
        )}
      </AnimatePresence>
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="flex h-10 w-10 items-center justify-center rounded-full border border-border bg-card hover:bg-accent"
        aria-label="Open app dock"
      >
        <IconLayoutNavbarCollapse className="h-5 w-5" />
      </button>
    </div>
  );
};

const FloatingDockDesktop = ({
  items,
  context,
  className,
  onCommandSubmit,
}: {
  items: FloatingDockItem[];
  context?: FloatingDockContext;
  className?: string;
  onCommandSubmit?: (command: string) => void;
}) => {
  const mouseX = useMotionValue(Infinity);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [commandMode, setCommandMode] = useState(false);
  const [command, setCommand] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const selectedTitle = items[selectedIndex]?.title;

  useEffect(() => {
    setSelectedIndex(0);
  }, [items, context?.mode]);

  useEffect(() => {
    if (!commandMode) return;
    inputRef.current?.focus();
  }, [commandMode]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!event.ctrlKey || event.metaKey || event.altKey) return;
      if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key)) return;
      event.preventDefault();
      event.stopPropagation();

      if (event.key === "ArrowUp" || event.key === "ArrowDown") {
        setCommandMode((value) => !value);
        return;
      }

      if (items.length === 0) return;
      const delta = event.key === "ArrowRight" ? 1 : -1;
      setSelectedIndex((current) => {
        const next = (current + delta + items.length) % items.length;
        items[next]?.onClick?.();
        return next;
      });
    };

    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () => window.removeEventListener("keydown", onKeyDown, { capture: true });
  }, [items]);

  function submitCommand(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const value = command.trim();
    if (!value) return;
    onCommandSubmit?.(value);
    setCommand("");
    setCommandMode(false);
  }

  return (
    <motion.div
      layout
      onMouseMove={(e) => mouseX.set(e.pageX)}
      onMouseLeave={() => mouseX.set(Infinity)}
      className={cn(
        "mx-auto hidden h-[58px] items-end overflow-hidden rounded-2xl border border-border bg-background/90 px-4 pb-2 md:flex",
        className,
      )}
      data-dock-mode={context?.mode || "apps"}
    >
      <div className="relative flex h-full min-w-0 items-end">
        <AnimatePresence mode="wait">
          {commandMode ? (
            <motion.form
              key="command-entry"
              initial={{ opacity: 0, y: 26 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 26 }}
              transition={{ type: "spring", stiffness: 320, damping: 30 }}
              onSubmit={submitCommand}
              className="flex h-10 w-[min(520px,calc(100vw-8rem))] items-center gap-2 rounded-full border border-border bg-card px-4"
            >
              <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-muted-foreground">
                {context?.label || "Command"}
              </span>
              <input
                ref={inputRef}
                value={command}
                onChange={(event) => setCommand(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Escape") {
                    event.preventDefault();
                    setCommandMode(false);
                    setCommand("");
                  }
                }}
                placeholder="Type command..."
                className="min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground/45"
              />
            </motion.form>
          ) : (
            <motion.div
              key={`${context?.mode || "apps"}-${selectedTitle || "none"}`}
              initial={{ opacity: 0, y: -18 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 28 }}
              transition={{ type: "spring", stiffness: 320, damping: 30 }}
              className="flex items-end gap-4"
            >
              {context?.label && (
                <div className="mb-1 hidden h-8 items-center rounded-full border border-border/70 bg-card/80 px-3 font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground xl:flex">
                  {context.label}
                </div>
              )}
              {items.map((item, index) => (
                <IconContainer
                  mouseX={mouseX}
                  key={`${item.kind || "app"}-${item.title}`}
                  selected={index === selectedIndex}
                  {...item}
                />
              ))}
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </motion.div>
  );
};

function IconContainer({
  mouseX,
  title,
  icon,
  onClick,
  selected,
  subtitle,
}: {
  mouseX: MotionValue<number>;
  title: string;
  icon: ReactNode;
  href?: string;
  onClick?: () => void;
  selected?: boolean;
  subtitle?: string;
}) {
  const ref = useRef<HTMLButtonElement>(null);

  const distance = useTransform(mouseX, (val) => {
    const bounds = ref.current?.getBoundingClientRect() ?? { x: 0, width: 0 };
    return val - bounds.x - bounds.width / 2;
  });

  const widthTransform = useTransform(distance, [-150, 0, 150], [40, 80, 40]);
  const heightTransform = useTransform(distance, [-150, 0, 150], [40, 80, 40]);
  const widthTransformIcon = useTransform(distance, [-150, 0, 150], [20, 40, 20]);
  const heightTransformIcon = useTransform(distance, [-150, 0, 150], [20, 40, 20]);

  const width = useSpring(widthTransform, { mass: 0.1, stiffness: 150, damping: 12 });
  const height = useSpring(heightTransform, { mass: 0.1, stiffness: 150, damping: 12 });
  const widthIcon = useSpring(widthTransformIcon, { mass: 0.1, stiffness: 150, damping: 12 });
  const heightIcon = useSpring(heightTransformIcon, { mass: 0.1, stiffness: 150, damping: 12 });

  const [hovered, setHovered] = useState(false);

  return (
    <motion.button
      ref={ref}
      type="button"
      style={{ width, height }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={onClick}
      className={cn(
        "relative flex cursor-pointer items-center justify-center rounded-full border border-border bg-card transition-colors hover:bg-accent",
        selected && "ring-2 ring-primary/35",
      )}
      aria-label={title}
      title={title}
    >
      <AnimatePresence>
        {hovered && (
          <motion.div
            initial={{ opacity: 0, y: 10, x: "-50%" }}
            animate={{ opacity: 1, y: 0, x: "-50%" }}
            exit={{ opacity: 0, y: 2, x: "-50%" }}
            className="absolute -top-10 left-1/2 whitespace-nowrap rounded-md border border-border bg-card px-2 py-0.5 text-xs text-card-foreground"
          >
            {title}
            {subtitle && <span className="ml-1 text-muted-foreground">· {subtitle}</span>}
          </motion.div>
        )}
      </AnimatePresence>
      <motion.div
        style={{ width: widthIcon, height: heightIcon }}
        className="flex items-center justify-center overflow-hidden rounded-full"
      >
        {icon}
      </motion.div>
    </motion.button>
  );
}

export type { FloatingDockItem, FloatingDockContext };
