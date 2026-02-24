import { For } from 'solid-js';
import { integrationStore } from '../stores/integrations';

export default function IntegrationsPanel() {
  const rows = () => integrationStore.integrations();

  return (
    <div class="h-full w-full bg-[#151515] text-neutral-100 p-3">
      <h2 class="text-sm font-semibold mb-3">Integrations</h2>
      <div class="space-y-2">
        <For each={rows()}>{(item) => (
          <div class="rounded border border-neutral-800 px-2 py-1 flex items-center justify-between">
            <span>{item.name}</span>
            <span class="text-xs text-neutral-400">{item.status}</span>
          </div>
        )}</For>
      </div>
    </div>
  );
}
