import { For, Show, createMemo } from "solid-js";

function IntentContextMenu(props) {
  const menuStyle = createMemo(() => {
    const width = 224;
    const estimatedHeight = 56 + (Math.max(1, (props.actions || []).length) * 34);
    const viewportWidth = typeof window !== "undefined" ? window.innerWidth : 1280;
    const viewportHeight = typeof window !== "undefined" ? window.innerHeight : 720;
    const left = Math.max(8, Math.min(props.position.x, viewportWidth - width - 8));
    const top = Math.max(8, Math.min(props.position.y, viewportHeight - estimatedHeight - 8));
    return { left: `${left}px`, top: `${top}px` };
  });

  return (
    <Show when={props.open}>
      <div
        class="fixed z-[13000] w-56 overflow-hidden rounded-lg border border-neutral-700 bg-[#121218]/95 p-1 shadow-2xl backdrop-blur-xl"
        role="menu"
        aria-label="Intent actions"
        data-testid="intent-context-menu"
        style={menuStyle()}
        onPointerDown={(event) => event.stopPropagation()}
        onContextMenu={(event) => {
          event.preventDefault();
          event.stopPropagation();
        }}
      >
        <p class="px-2 py-1 text-[11px] uppercase tracking-wide text-neutral-500">IntentUI Actions</p>
        <div class="my-1 h-px bg-neutral-800" />
        <For each={props.actions}>
          {(action) => (
            <Show
              when={action.label !== "__sep__"}
              fallback={<div class="my-1 h-px bg-neutral-800" />}
            >
              <button
                type="button"
                class="flex w-full items-center rounded-md px-2 py-1.5 text-left text-sm text-neutral-200 transition-colors hover:bg-neutral-800"
                data-testid={`intent-context-action-${String(action.label || "").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "")}`}
                onClick={async () => {
                  props.onClose();
                  await action.run();
                }}
              >
                <Show when={action.icon}>
                  {(resolvedIcon) => {
                    const Icon = resolvedIcon();
                    return Icon ? <Icon size={14} class="mr-2 text-neutral-400" /> : null;
                  }}
                </Show>
                {action.label}
              </button>
            </Show>
          )}
        </For>
      </div>
    </Show>
  );
}

export default IntentContextMenu;
