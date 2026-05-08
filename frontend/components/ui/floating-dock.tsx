"use client";
import { cn } from "@/lib/utils";
import { PanelTopClose } from "lucide-react";
import {
  AnimatePresence,
  motion,
  useMotionValue,
  useSpring,
  useTransform,
} from "motion/react";
import type { MotionValue } from "motion/react";

import { useEffect, useMemo, useRef, useState, useCallback, type ReactNode } from "react";

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

type DockPage = "people" | "launcher" | "command";
type CommandPrefix = "/" | "#" | "?" | "~";

type CommandSuggestion = {
  value: string;
  label: string;
  prefix: CommandPrefix;
};

const COMMAND_PREFIXES: Record<CommandPrefix, { label: string; title: string; className: string }> = {
  "/": { label: "General", title: "General command", className: "bg-sky-500 text-white shadow-[0_0_18px_rgba(14,165,233,0.35)]" },
  "#": { label: "Terminal", title: "Send to terminal", className: "bg-cyan-500 text-white shadow-[0_0_18px_rgba(6,182,212,0.35)]" },
  "?": { label: "Help", title: "Help topics", className: "bg-amber-400 text-black shadow-[0_0_18px_rgba(251,191,36,0.35)]" },
  "~": { label: "AI", title: "AI input", className: "bg-fuchsia-500 text-white shadow-[0_0_18px_rgba(217,70,239,0.35)]" },
};

const SUGGESTIONS: CommandSuggestion[] = [
  { prefix: "/", value: "/open settings", label: "Open Settings" },
  { prefix: "#", value: "#bun run build", label: "Build frontend" },
  { prefix: "#", value: "#bun run lint", label: "Lint frontend" },
  { prefix: "#", value: "#git status", label: "Git status" },
  { prefix: "/", value: "/lock", label: "Lock profile" },
  { prefix: "?", value: "?profile container", label: "Profile container" },
  { prefix: "?", value: "?dock shortcuts", label: "Dock shortcuts" },
  { prefix: "~", value: "~show node identity", label: "Show node identity" },
];

function commandPrefixFor(value: string): CommandPrefix {
  const first = value.trimStart().slice(0, 1) as CommandPrefix;
  return first === "#" || first === "?" || first === "~" || first === "/" ? first : "/";
}

function commandBody(value: string) {
  const trimmed = value.trimStart();
  return ["/", "#", "?", "~"].includes(trimmed[0] || "") ? trimmed.slice(1) : trimmed;
}

function nextPrefix(prefix: CommandPrefix): CommandPrefix {
  if (prefix === "/") return "#";
  if (prefix === "#") return "?";
  if (prefix === "?") return "~";
  return "/";
}

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
            className="absolute bottom-full left-1/2 mb-3 flex -translate-x-1/2 flex-col gap-2"
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
                  className="flex h-11 w-11 items-center justify-center rounded-full border border-white/10 bg-white/10 text-white shadow-lg backdrop-blur-xl hover:bg-primary/15"
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
        className="flex h-11 w-11 items-center justify-center rounded-full border border-white/10 bg-white/10 text-white shadow-lg backdrop-blur-xl hover:bg-primary/15"
        aria-label="Open app dock"
      >
        <PanelTopClose className="h-5 w-5" />
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
  const [page, setPage] = useState<DockPage>("launcher");
  const [command, setCommand] = useState("");
  const [currentPrefix, setCurrentPrefix] = useState<CommandPrefix>("/");
  const inputRef = useRef<HTMLInputElement>(null);

  const peopleItems = useMemo(() => context?.items ?? [], [context?.items]);
  const launcherItemsStable = useMemo(() => launcherItems, [launcherItems]);
  const hasPeoplePage = Boolean(peopleItems.length);
  const commandPrefix = commandPrefixFor(command || currentPrefix);
  const commandMode = COMMAND_PREFIXES[commandPrefix];
  const commandQuery = commandBody(command).toLowerCase();
  const suggestions = useMemo(() => {
    return SUGGESTIONS
      .filter((item) => item.prefix === commandPrefix)
      .filter((item) => !commandQuery || item.label.toLowerCase().includes(commandQuery) || item.value.toLowerCase().includes(commandQuery))
      .slice(0, 4);
  }, [commandPrefix, commandQuery]);

  useEffect(() => {
    if (!hasPeoplePage && page === "people") setPage("launcher");
  }, [hasPeoplePage, page]);

  useEffect(() => {
    if (page !== "command") return;
    inputRef.current?.focus();
  }, [page]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!event.ctrlKey || event.metaKey || event.altKey) return;
      if (!["ArrowLeft", "ArrowRight"].includes(event.key)) return;
      event.preventDefault();
      event.stopPropagation();

      if (event.key === "ArrowLeft") {
        setPage((current) => {
          if (current === "command") return "launcher";
          if (current === "launcher" && hasPeoplePage) return "people";
          return current;
        });
        return;
      }

      setPage((current) => {
        if (current === "people") return "launcher";
        if (current === "launcher") return "command";
        return current;
      });
    };

    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () => window.removeEventListener("keydown", onKeyDown, { capture: true });
  }, [hasPeoplePage]);

  const setPrefix = useCallback((prefix: CommandPrefix) => {
    setCurrentPrefix(prefix);
    setCommand(`${prefix}${commandBody(command)}`);
    inputRef.current?.focus();
  }, [command]);

  const cyclePrefix = useCallback(() => {
    setPrefix(nextPrefix(commandPrefix));
  }, [commandPrefix, setPrefix]);

  const submitCommand = useCallback((event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const body = commandBody(command).trim();
    if (!body) return;
    onCommandSubmit?.(`${commandPrefix}${body}`);
    setCommand("");
    setCurrentPrefix("/");
    setPage("launcher");
  }, [command, commandPrefix, onCommandSubmit]);

  const applySuggestion = useCallback((value: string) => {
    setCommand(value);
    setCurrentPrefix(commandPrefixFor(value));
    inputRef.current?.focus();
  }, []);

  return (
    <motion.div
      layout
      onMouseMove={(e) => mouseX.set(e.pageX)}
      onMouseLeave={() => mouseX.set(Infinity)}
      className={cn(
        "mx-auto hidden h-16 items-end rounded-2xl border border-white/10 bg-black/50 px-4 pb-2.5 shadow-2xl shadow-black/40 backdrop-blur-xl md:flex",
        className,
      )}
      data-dock-page={page}
      role="toolbar"
      aria-label="Application dock"
    >
      <div className="relative flex h-full min-w-0 items-end">
        <AnimatePresence mode="wait" initial={false}>
          {page === "command" ? (
            <motion.form
              key="command-entry"
              initial={{ opacity: 0, x: 42 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: 42 }}
              transition={{ type: "spring", stiffness: 320, damping: 30 }}
              onSubmit={submitCommand}
              className="relative flex h-11 w-[min(540px,calc(100vw-8rem))] items-center gap-2 rounded-full border border-border bg-card px-2.5 shadow-xl"
              role="search"
              aria-label="Command input"
            >
              <button
                type="button"
                onClick={cyclePrefix}
                className={cn(
                  "flex h-7 w-7 shrink-0 items-center justify-center rounded-full font-mono text-sm font-bold transition-colors",
                  commandMode.className,
                )}
                aria-label={commandMode.title}
                title={`${commandMode.title}. Click to cycle mode.`}
              >
                {commandPrefix}
              </button>
              <input
                ref={inputRef}
                value={commandBody(command)}
                onChange={(event) => setCommand(`${commandPrefix}${event.target.value}`)}
                onKeyDown={(event) => {
                  if (event.key === "Escape") {
                    event.preventDefault();
                    setPage("launcher");
                    setCommand("");
                    setCurrentPrefix("/");
                  }
                  if ((event.metaKey || event.ctrlKey) && event.key === " ") {
                    event.preventDefault();
                    cyclePrefix();
                  }
                }}
                placeholder={`${commandMode.label.toLowerCase()}...`}
                className="min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground/55"
              />
              {suggestions.length > 0 && (
                <div className="absolute bottom-[calc(100%+10px)] left-0 right-0 overflow-hidden rounded-xl border border-border bg-card/95 p-1 shadow-2xl backdrop-blur-md">
                  {suggestions.map((item) => (
                    <button
                      key={item.value}
                      type="button"
                      onClick={() => applySuggestion(item.value)}
                      className="flex h-9 w-full items-center gap-2 rounded-lg px-3 text-left text-xs hover:bg-accent/20"
                    >
                      <span className={cn("flex h-5 w-5 items-center justify-center rounded-full font-mono text-[10px]", COMMAND_PREFIXES[item.prefix].className)}>{item.prefix}</span>
                      <span className="min-w-0 flex-1 truncate">{item.label}</span>
                      <span className="truncate font-mono text-[10px] text-muted-foreground">{item.value}</span>
                    </button>
                  ))}
                </div>
              )}
            </motion.form>
          ) : (
            <motion.div
              key={page}
              initial={{ opacity: 0, x: page === "people" ? -42 : 42 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: page === "people" ? -42 : 42 }}
              transition={{ type: "spring", stiffness: 320, damping: 30 }}
              className="flex items-end gap-3"
            >
              {(page === "people" ? peopleItems : launcherItemsStable).map((item, index) => (
                <IconContainer
                  mouseX={mouseX}
                  key={`${page}-${item.kind || "app"}-${item.title}-${index}`}
                  {...item}
                />
              ))}
            </motion.div>
          )}
        </AnimatePresence>

        <div className="pointer-events-none absolute left-1/2 top-[calc(100%+6px)] flex -translate-x-1/2 items-center gap-1.5" role="tablist" aria-label="Dock pages">
          {hasPeoplePage && <span className={cn("h-1.5 w-1.5 rounded-full transition-colors", page === "people" ? "bg-primary" : "bg-muted-foreground/35")} role="tab" aria-selected={page === "people"} aria-label="People" />}
          <span className={cn("h-1.5 w-1.5 rounded-full transition-colors", page === "launcher" ? "bg-primary" : "bg-muted-foreground/35")} role="tab" aria-selected={page === "launcher"} aria-label="Launcher" />
          <span className={cn("h-1.5 w-1.5 rounded-full transition-colors", page === "command" ? "bg-primary" : "bg-muted-foreground/35")} role="tab" aria-selected={page === "command"} aria-label="Command" />
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

  const widthTransform = useTransform(distance, [-150, 0, 150], [42, 72, 42]);
  const heightTransform = useTransform(distance, [-150, 0, 150], [42, 72, 42]);
  const widthTransformIcon = useTransform(distance, [-150, 0, 150], [20, 34, 20]);
  const heightTransformIcon = useTransform(distance, [-150, 0, 150], [20, 34, 20]);

  const width = useSpring(widthTransform, { mass: 0.1, stiffness: 150, damping: 12 });
  const height = useSpring(heightTransform, { mass: 0.1, stiffness: 150, damping: 12 });
  const widthIcon = useSpring(widthTransformIcon, { mass: 0.1, stiffness: 150, damping: 12 });
  const heightIcon = useSpring(heightTransformIcon, { mass: 0.1, stiffness: 150, damping: 12 });

  useEffect(() => {
    return () => {
      width.stop();
      height.stop();
      widthIcon.stop();
      heightIcon.stop();
    };
  }, [width, height, widthIcon, heightIcon]);

  const [hovered, setHovered] = useState(false);

  return (
    <motion.button
      ref={ref}
      type="button"
      style={{ width, height }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={onClick}
      className="relative flex cursor-pointer items-center justify-center rounded-full border border-white/10 bg-white/10 text-white shadow-md transition-colors hover:border-primary/30 hover:bg-primary/15"
      aria-label={title}
      title={title}
      role="button"
      tabIndex={0}
    >
      <AnimatePresence>
        {hovered && (
          <motion.div
            initial={{ opacity: 0, y: 10, x: "-50%" }}
            animate={{ opacity: 1, y: 0, x: "-50%" }}
            exit={{ opacity: 0, y: 2, x: "-50%" }}
            className="absolute -top-10 left-1/2 whitespace-nowrap rounded-md border border-border bg-card px-2.5 py-1 text-xs text-card-foreground shadow-xl"
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
