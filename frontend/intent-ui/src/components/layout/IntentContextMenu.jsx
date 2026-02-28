import { For, Show } from "solid-js";

function IntentContextMenu(props) {
  return (
    <Show when={props.open}>
      <div
        class="fixed z-[13000] w-56 overflow-hidden rounded-lg border border-neutral-700 bg-[#121218]/95 p-1 shadow-2xl backdrop-blur-xl"
        style={{ left: `${props.position.x}px`, top: `${props.position.y}px` }}
        onPointerDown={(event) => event.stopPropagation()}
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
                onClick={async () => {
                  props.onClose();
                  await action.run();
                }}
              >
                <Show when={action.icon}>
                  {(Icon) => <Icon size={14} class="mr-2 text-neutral-400" />}
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
