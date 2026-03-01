import {
  TbOutlineFileText,
  TbOutlineCloud,
  TbOutlineSettings,
  TbOutlineBook2,
  TbOutlineCommand,
  TbOutlineDeviceDesktop,
  TbOutlineKey
} from "solid-icons/tb";
import { FiLink2 } from "solid-icons/fi";

export const CHAT_HEAD_PREFS_KEY = "intent-ui-chat-head-prefs-v1";
export const CHAT_BUBBLES_KEY = "intent-ui-chat-bubbles-v1";
export const CHAT_HEAD_PRESET_COLORS = ["#1d4ed8", "#0f766e", "#6d28d9", "#b45309", "#be123c", "#374151"];
export const EMOJI_QUICK_SET = ["😀", "🚀", "🔥", "💬", "✅", "🧠", "📌", "👀", "🎯", "🤝", "❤️", "⚡"];
export const THREAD_PAGE_SIZE = 80;
export const THREAD_ROW_ESTIMATE = 92;
export const THREAD_OVERSCAN = 6;

export const LOCAL_BRIDGE_LISTEN = "127.0.0.1:7777";

export const DOMAIN_RESERVATION_STORAGE_KEYS = [
  "intent-ui-domain-reservation-v1",
  "intent-ui-domain-v1",
  "intent-ui-user-domain-v1",
  "edgerun_user_domain"
];

export const DRAWER_PANEL_SHELL_CLASS = "flex h-full min-h-0 flex-col";
export const DRAWER_ICON_BUTTON_CLASS = "inline-flex h-9 w-9 items-center justify-center rounded-md text-neutral-300 transition-colors hover:bg-neutral-800/35 hover:text-[hsl(var(--primary))]";
export const DRAWER_SMALL_BUTTON_CLASS = "inline-flex h-7 items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]";
export const DRAWER_LIST_ROW_CLASS = "w-full rounded-md border border-neutral-800 bg-neutral-900/70 px-2.5 py-2 text-left transition-colors hover:bg-neutral-800/80";
export const DRAWER_STATE_BLOCK_CLASS = "rounded-md border border-neutral-800 bg-neutral-900/55 px-2.5 py-2 text-xs text-neutral-500";
export const DRAWER_SUGGESTION_ICON_BUTTON_CLASS = "inline-flex h-8 w-8 items-center justify-center rounded-md border border-neutral-800 bg-neutral-900/70 text-neutral-200 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]";

export const PANEL_SUGGESTION_TAGS = {
  launcher: ["workflows", "ai", "messages", "storage", "code"],
  files: ["storage", "code"],
  cloud: ["workflows", "network", "compute", "deploy"],
  integrations: ["messages", "storage", "code", "workflows", "network", "compute", "ai", "security"],
  credentials: ["security", "identity"],
  settings: ["workflows", "devices", "network"],
  conversations: ["messages", "ai"],
  devices: ["devices", "network", "workflows"]
};

export const LEFT_DRAWER_PANEL_ITEMS = [
  { id: "launcher", title: "Launcher panel", Icon: TbOutlineBook2 },
  { id: "files", title: "Files panel", Icon: TbOutlineFileText },
  { id: "cloud", title: "Cloud panel", Icon: TbOutlineCloud },
  { id: "integrations", title: "Integrations panel", Icon: FiLink2 },
  { id: "credentials", title: "Credentials panel", Icon: TbOutlineKey },
  { id: "settings", title: "Settings panel", Icon: TbOutlineSettings }
];

export const RIGHT_DRAWER_PANEL_ITEMS = [
  { id: "conversations", title: "Conversations", Icon: TbOutlineCommand },
  { id: "devices", title: "Devices panel", Icon: TbOutlineDeviceDesktop }
];
