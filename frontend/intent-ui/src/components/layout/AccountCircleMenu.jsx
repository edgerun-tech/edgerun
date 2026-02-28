import { Show } from "solid-js";
import { TbOutlineChevronDown } from "solid-icons/tb";

function AccountCircleMenu(props) {
  return (
    <div class="fixed right-3 top-3 z-[12000] flex items-start justify-end">
      <span class="hidden" data-testid="profile-runtime-mode">Session mode: {props.sessionModeLabel}</span>
      <div class="relative">
        <button
          type="button"
          class="inline-flex h-10 items-center gap-2 rounded-full border border-neutral-800/80 bg-[#0f0f0f]/75 px-3 text-xs text-neutral-200 shadow-lg backdrop-blur-xl transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
          onPointerDown={(event) => event.stopPropagation()}
          onClick={props.onToggle}
          data-testid="account-circle-trigger"
        >
          <span class="inline-flex h-6 w-6 items-center justify-center rounded-full border border-neutral-700 bg-neutral-900 text-[10px] font-semibold">ER</span>
          <span class="uppercase tracking-wide">Account</span>
          <TbOutlineChevronDown size={12} />
        </button>
        <Show when={props.open}>
          <div
            class="absolute right-0 top-full mt-2 w-72 rounded-xl border border-neutral-700 bg-[#101216]/95 p-3 shadow-2xl backdrop-blur-xl"
            onPointerDown={(event) => event.stopPropagation()}
            data-testid="account-circle-menu"
          >
            <p class="text-[10px] uppercase tracking-wide text-neutral-500">Session Mode</p>
            <p class="mt-1 text-xs text-neutral-200" data-testid="profile-runtime-mode-menu">{props.sessionModeLabel}</p>
            <div class="mt-2 space-y-1 rounded-md border border-neutral-800 bg-neutral-900/60 p-2 text-[11px] text-neutral-400">
              <p>Profile: <span class="text-neutral-200">{props.shortProfileId}</span></p>
              <p>Backend: <span class="text-neutral-200">{props.backend || "not linked"}</span></p>
              <p data-testid="account-domain-value">Domain: <span class="text-neutral-200">{props.registeredDomain || "Not registered"}</span></p>
            </div>
            <div class="mt-3 grid grid-cols-1 gap-1.5">
              <button
                type="button"
                class="inline-flex items-center justify-center rounded border border-neutral-700 bg-neutral-900 px-2 py-1.5 text-[11px] text-neutral-200 hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
                onClick={props.onResetSession}
                data-testid="account-reset-session"
              >
                Reset session
              </button>
            </div>
          </div>
        </Show>
      </div>
    </div>
  );
}

export default AccountCircleMenu;
