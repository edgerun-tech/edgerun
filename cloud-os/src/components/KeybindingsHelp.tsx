import { createSignal, Show } from 'solid-js';

export default function KeybindingsHelp() {
  const [open, setOpen] = createSignal(false);

  return (
    <>
      <button
        type="button"
        class="fixed right-4 bottom-4 z-[10004] rounded border border-neutral-700 bg-black/60 px-2 py-1 text-xs text-neutral-200"
        onClick={() => setOpen((v) => !v)}
      >
        Keys
      </button>
      <Show when={open()}>
        <div class="fixed right-4 bottom-12 z-[10004] w-64 rounded border border-neutral-700 bg-black/90 p-3 text-xs text-neutral-200">
          <p class="font-semibold mb-2">Keybindings</p>
          <p><kbd>Ctrl/Meta</kbd> + <kbd>Space</kbd>: focus command input</p>
          <p><kbd>Esc</kbd>: close overlays</p>
        </div>
      </Show>
    </>
  );
}
