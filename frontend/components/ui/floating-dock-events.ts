import { commandBody, isCommandPrefix, type CommandPrefix } from "@/stores/floating-dock-ui-store";

type UnknownRecord = Record<string, unknown>;

type DockCommandInputEventDetail = {
  prefix?: unknown;
  value?: unknown;
  submit?: unknown;
};

export type ResolvedDockCommandInput = {
  prefix: CommandPrefix;
  body: string;
  shouldSubmit: boolean;
};

export function parseDockCommandInputEvent(event: Event): ResolvedDockCommandInput | null {
  if (!(event instanceof CustomEvent)) return null;
  const detail = event.detail;
  if (!isRecord(detail)) return null;
  const payload = detail as DockCommandInputEventDetail;
  const prefix = isCommandPrefix(normalizePrefix(payload.prefix)) ? payload.prefix : "~";
  const body = typeof payload.value === "string" ? commandBody(payload.value) : "";
  const shouldSubmit = payload.submit === true && body.trim().length > 0;
  return { prefix, body, shouldSubmit };
}

function isRecord(value: unknown): value is UnknownRecord {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function normalizePrefix(value: unknown): string {
  return typeof value === "string" ? value.trimStart().slice(0, 1) : "";
}
