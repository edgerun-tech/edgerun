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

import { useEffect, useRef, useState, type ReactNode } from "react";

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
  items?: FloatingDockItem[];
};

type DockLane = "launcher" | "context";
type DockMode = "icons" | "command";

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
  const mobileItems = context?.items?.length ? context.items : items;

  return (
    <>
      <FloatingDockDesktop
        launcherItems={items}
        context={context}
        className={desktopClassName}
        onCommandSubmit={onCommandSubmit}
      />
      <FloatingDockMobile items={mobileItems} className={mobileClassName} />
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
  launcherItems,
  context,
  className,
  onCommandSubmit,
}: {
  launcherItems: FloatingDockItem[];
  context?: FloatingDockContext;
  className?: string;
  onCommandSubmit?: (command: string) => void;
}) => {
  const mouseX = useMotionValue(Infinity);
  const [lane, setLane] = useState<DockLane>("launcher");
  const [mode, setMode] = useState<DockMode>("icons");
  const [command, setCommand] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const hasContextLane = Boolean(context?.items?.length);
  const contextItems = context?.items ?? [];
  const visibleItems = lane === "context" && hasContextLane ? contextItems : launcherItems;

  useEffect(() => {
    if (!hasContextLane && lane === "context") setLane("launcher");
  }, [hasContextLane, lane]);

  useEffect(() => {
    if (mode !== "command") return;
    inputRef.current?.focus();
  }, [mode]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!event.ctrlKey || event.metaKey || event.altKey) return;
      if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key)) return;
      event.preventDefault();
      event.stopPropagation();

      if (event.key === "ArrowDown") {
        setMode("command");
        return;
      }

      if (event.key === "ArrowUp") {
        setMode("icons");
        return;
      }

      if (!hasContextLane) return;
      if (event.key === "ArrowRight") setLane("context");
      if (event.key === "ArrowLeft") setLane("launcher");
    };

    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () => window.removeEventListener("keydown", onKeyDown, { capture: true });
  }, [hasContextLane]);

  function submitCommand(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const value = command.trim();
    if (!value) return;
    onCommandSubmit?.(value);
    setCommand("");
    setMode("icons");
  }

  return (
    <motion.div
      layout
      onMouseMove={(e) => mouseX.set(e.pageX)}
      onMouseLeave={() => mouseX.set(Infinity)}
      className={cn(
        "mx-auto hidden h-[58px] items-end rounded-2xl border border-border bg-background/90 px-4 pb-2 md:flex",
        className,
      )}
      data-dock-lane={lane}
      data-dock-mode={mode}
    >
      <div className="relative flex h-full min-w-0 items-end">
        <AnimatePresence mode="wait" initial={false}>
          {mode === "command" ? (
            <motion.form
              key="command-entry"
              initial={{ opacity: 0, y: 28 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 28 }}
              transition={{ type: "spring", stiffness: 320, damping: 30 }}
              onSubmit={submitCommand}
              className="flex h-10 w-[min(520px,calc(100vw-8rem))] items-center gap-2 rounded-full border border-border bg-card px-4"
            >
              <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-muted-foreground">
                Command
              </span>
              <input
                ref={inputRef}
                value={command}
                onChange={(event) => setCommand(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Escape") {
                    event.preventDefault();
                    setMode("icons");
                    setCommand("");
                  }
                }}
                placeholder="Type command..."
                className="min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground/45"
              />
            </motion.form>
          ) : (
            <motion.div
              key={lane}
              initial={{ opacity: 0, x: lane === "context" ? 42 : -42 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: lane === "context" ? -42 : 42, y: 0 }}
              transition={{ type: "spring", stiffness: 320, damping: 30 }}
              className="flex items-end gap-4"
            >
              {visibleItems.map((item) => (
                <IconContainer
                  mouseX={mouseX}
                  key={`${lane}-${item.kind || "app"}-${item.title}`}
                  {...item}
                />
              ))}
            </motion.div>
          )}
        </AnimatePresence>

        <div className="pointer-events-none absolute left-1/2 top-[calc(100%+6px)] flex -translate-x-1/2 items-center gap-1.5">
          <span className={cn("h-1.5 w-1.5 rounded-full transition-colors", lane === "launcher" && mode === "icons" ? "bg-primary" : "bg-muted-foreground/35")} />
          {hasContextLane && <span className={cn("h-1.5 w-1.5 rounded-full transition-colors", lane === "context" && mode === "icons" ? "bg-primary" : "bg-muted-foreground/35")} />}
          <span className={cn("h-1.5 w-1.5 rounded-full transition-colors", mode === "command" ? "bg-primary" : "bg-muted-foreground/35")} />
        </div>
      </div>
    </motion.div>
  );
};

function IconContainer({
  mouseX,
  title,
  icon,
  onClick,
  subtitle,
}: {
  mouseX: MotionValue<number>;
  title: string;
  icon: ReactNode;
  href?: string;
  onClick?: () => void;
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
      className="relative flex cursor-pointer items-center justify-center rounded-full border border-border bg-card transition-colors hover:bg-accent"
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
