import { For } from 'solid-js';
import { getDockItems } from '../lib/config/integrations.config';
import { openWindow } from '../stores/windows';

export default function Dock() {
  const items = () => getDockItems();

  return (
    <div class="fixed bottom-4 left-1/2 z-[10002] -translate-x-1/2 rounded-xl border border-neutral-700 bg-black/60 px-3 py-2 backdrop-blur">
      <div class="flex items-center gap-2">
        <For each={items()}>{(item) => (
          <button
            type="button"
            class="rounded-md border border-neutral-700 px-2 py-1 text-xs text-neutral-100 hover:bg-neutral-800"
            onClick={() => openWindow(item.id)}
          >
            {item.name}
          </button>
        )}</For>
      </div>
    </div>
  );
}
