import type { CommandPrefix } from "@/stores/floating-dock-ui-store";

export type CommandPrefixConfig = {
  label: string;
  title: string;
  className: string;
  routeLabel: string;
  mode: "assistant" | "command";
  assistantRoute?: "frontend" | "backend" | "chatgpt";
  supportsLaunchItems: boolean;
};

export type CommandSuggestion = {
  value: string;
  label: string;
  prefix: CommandPrefix;
};

export const COMMAND_PREFIXES: Record<CommandPrefix, CommandPrefixConfig> = {
  "/": {
    label: "General",
    title: "General command",
    routeLabel: "Dock command",
    mode: "command",
    className: "text-muted-foreground hover:text-foreground",
    supportsLaunchItems: true,
  },
  "?": {
    label: "Help",
    title: "Help topics",
    routeLabel: "Inline help",
    mode: "command",
    className: "text-amber-300 hover:text-amber-200",
    supportsLaunchItems: false,
  },
  "~": {
    label: "Backend AI",
    title: "Codex over /api/codex",
    routeLabel: "/api/codex",
    mode: "assistant",
    assistantRoute: "backend",
    className: "text-primary hover:text-primary/85",
    supportsLaunchItems: false,
  },
  "!": {
    label: "ChatGPT",
    title: "ChatGPT via local CDP",
    routeLabel: "CDP localhost",
    mode: "assistant",
    assistantRoute: "chatgpt",
    className: "text-emerald-300 hover:text-emerald-200",
    supportsLaunchItems: false,
  },
};

export const SUGGESTIONS: CommandSuggestion[] = [
  { prefix: "/", value: "/open settings", label: "Open Settings" },
  { prefix: "/", value: "/lock", label: "Lock profile" },
  { prefix: "/", value: "/node identity", label: "Node identity" },
  { prefix: "/", value: "/checklist", label: "Checklist" },
  { prefix: "?", value: "?commands", label: "Prompt commands" },
  { prefix: "?", value: "?node", label: "Node commands" },
  { prefix: "?", value: "?checklist", label: "Checklist commands" },
  { prefix: "?", value: "?open", label: "Open apps" },
  { prefix: "!", value: "!ask chatgpt", label: "Ask ChatGPT" },
];
