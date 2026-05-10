import { commandBody, isCommandPrefix, type CommandPrefix } from "@/stores/floating-dock-ui-store";
import { isRecord } from "@/platform/auth/helpers";

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
  const rawPrefix = normalizePrefix(payload.prefix)
  const prefix = isCommandPrefix(rawPrefix) ? rawPrefix : "~";
  const body = typeof payload.value === "string" ? commandBody(payload.value) : "";
  const shouldSubmit = payload.submit === true && body.trim().length > 0;
  return { prefix, body, shouldSubmit };
}

function normalizePrefix(value: unknown): string {
  return typeof value === "string" ? value.trimStart().slice(0, 1) : "";
}
