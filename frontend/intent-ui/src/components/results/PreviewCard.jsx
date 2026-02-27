import { Show, For, createSignal, onCleanup } from "solid-js";
import { Motion } from "solid-motionone";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import { TbOutlineFingerprint } from "solid-icons/tb";
function cn(...classes) {
  return twMerge(clsx(classes));
}
function PreviewCard(props) {
  const DEFAULT_AUTH_TIMEOUT_MS = 1e4;
  const [pendingAuthIntent, setPendingAuthIntent] = createSignal(null);
  const [authCountdownMs, setAuthCountdownMs] = createSignal(0);
  let authInterval = null;
  const clearAuthPending = () => {
    if (authInterval) {
      clearInterval(authInterval);
      authInterval = null;
    }
    setPendingAuthIntent(null);
    setAuthCountdownMs(0);
  };
  const startAuthPending = (action) => {
    const timeoutMs = Number(action.authTimeoutMs) > 0 ? Number(action.authTimeoutMs) : DEFAULT_AUTH_TIMEOUT_MS;
    const deadline = Date.now() + timeoutMs;
    clearAuthPending();
    setPendingAuthIntent(action.intent);
    setAuthCountdownMs(timeoutMs);
    authInterval = setInterval(() => {
      const remaining = Math.max(0, deadline - Date.now());
      setAuthCountdownMs(remaining);
      if (remaining === 0) {
        clearAuthPending();
      }
    }, 80);
  };
  onCleanup(() => {
    clearAuthPending();
  });
  const ui = () => props.response.ui;
  const formattedData = () => {
    if (!props.response.data) return null;
    if (typeof props.response.data === "string") {
      return props.response.data;
    }
    if (typeof props.response.data === "object") {
      const entries = Object.entries(props.response.data).slice(0, 5);
      return entries.map(([key, value]) => ({
        key,
        value: typeof value === "object" ? JSON.stringify(value) : String(value)
      }));
    }
    return String(props.response.data);
  };
  return <Motion.div
    initial={{ opacity: 0, y: 8 }}
    animate={{ opacity: 1, y: 0 }}
    exit={{ opacity: 0, y: -8 }}
    transition={{ duration: 0.2 }}
    class={cn(
      "bg-neutral-800/50 rounded-xl border border-neutral-700 overflow-hidden",
      props.class
    )}
  >
      {
    /* Header */
  }
      <Show when={ui()?.title || ui()?.metadata}>
        <div class="px-4 py-3 border-b border-neutral-700 bg-neutral-800/50">
          <div class="flex items-center justify-between">
            <Show when={ui()?.title}>
              <h3 class="text-sm font-medium text-white">{ui().title}</h3>
            </Show>
            <Show when={ui()?.metadata?.source}>
              <span class="text-xs text-neutral-400">{ui().metadata.source}</span>
            </Show>
          </div>
          <Show when={ui()?.description}>
            <p class="text-xs text-neutral-500 mt-1">{ui().description}</p>
          </Show>
        </div>
      </Show>

      {
    /* Content */
  }
      <div class="p-4">
        <Show
    when={typeof formattedData() === "string"}
    fallback={<div class="space-y-2">
              <For each={formattedData()}>
                {(item) => <div class="flex gap-2 text-sm">
                    <span class="text-neutral-500 min-w-[100px]">{item.key}:</span>
                    <span class="text-neutral-300 truncate">{item.value}</span>
                  </div>}
              </For>
            </div>}
  >
          <p class="text-sm text-neutral-300 whitespace-pre-wrap">
            {formattedData()}
          </p>
        </Show>

        {
    /* Metadata badges */
  }
        <Show when={ui()?.metadata?.itemCount}>
          <div class="flex items-center gap-2 mt-3 pt-3 border-t border-neutral-700">
            <span class="text-xs text-neutral-500">
              {ui().metadata.itemCount} items
            </span>
            <Show when={ui()?.metadata?.duration}>
              <span class="text-xs text-neutral-500">•</span>
              <span class="text-xs text-neutral-500">{ui().metadata.duration}</span>
            </Show>
          </div>
        </Show>
      </div>

      {
    /* Actions */
  }
      <Show when={ui()?.actions?.length}>
        <div class="px-4 py-3 bg-neutral-800/30 border-t border-neutral-700 flex flex-wrap gap-2" role="group" aria-label="Actions">
          <For each={ui()?.actions}>
            {(action) => <button
    type="button"
    onClick={() => {
      if (!action.authenticated) {
        props.onAction?.(action.intent, action);
        return;
      }
      const isPending = pendingAuthIntent() === action.intent;
      if (!isPending) {
        startAuthPending(action);
        return;
      }
      clearAuthPending();
      props.onAction?.(action.intent, action);
    }}
    class={cn(
      "relative overflow-hidden px-3 py-1.5 rounded-lg text-xs font-medium transition-colors cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2 focus-visible:ring-offset-neutral-800",
      action.variant === "primary" && "bg-blue-600 text-white hover:bg-blue-500",
      action.variant === "danger" && "bg-red-600 text-white hover:bg-red-500",
      action.variant === "ghost" && "bg-transparent text-neutral-400 hover:text-white hover:bg-neutral-700",
      !action.variant || action.variant === "secondary" && "bg-neutral-700 text-neutral-300 hover:bg-neutral-600",
      action.authenticated && pendingAuthIntent() === action.intent && "ring-1 ring-cyan-400/70"
    )}
  >
                <span class="relative z-10 inline-flex items-center gap-1.5">
                  <Show when={action.authenticated}>
                    <TbOutlineFingerprint
    size={13}
    class={pendingAuthIntent() === action.intent ? "text-cyan-300 animate-pulse" : "text-cyan-200"}
  />
                  </Show>
                  <Show
    when={action.authenticated && pendingAuthIntent() === action.intent}
    fallback={action.label}
  >
                    Confirm fingerprint ({Math.max(1, Math.ceil(authCountdownMs() / 1e3))}s)
                  </Show>
                </span>
                <Show when={action.authenticated && pendingAuthIntent() === action.intent}>
                  <span
    class={cn(
      "pointer-events-none absolute bottom-0 left-0 h-px transition-[width,background-color] duration-100",
      authCountdownMs() > 6e3 ? "bg-cyan-300" : authCountdownMs() > 3e3 ? "bg-amber-300" : "bg-red-300"
    )}
    style={{ width: `${Math.max(0, Math.min(100, authCountdownMs() / ((Number(action.authTimeoutMs) > 0 ? Number(action.authTimeoutMs) : DEFAULT_AUTH_TIMEOUT_MS) / 100)))}%` }}
  />
                </Show>
              </button>}
          </For>
        </div>
      </Show>
    </Motion.div>;
}
export {
  PreviewCard
};
