import { createSignal, onMount, onCleanup, Show } from 'solid-js';

export default function OfflineIndicator() {
  const [online, setOnline] = createSignal(true);

  onMount(() => {
    const refresh = () => setOnline(navigator.onLine);
    refresh();
    window.addEventListener('online', refresh);
    window.addEventListener('offline', refresh);
    onCleanup(() => {
      window.removeEventListener('online', refresh);
      window.removeEventListener('offline', refresh);
    });
  });

  return (
    <Show when={!online()}>
      <div class="fixed right-4 top-4 z-[10005] rounded border border-rose-700 bg-rose-900/80 px-3 py-1 text-xs text-rose-100">
        Offline mode
      </div>
    </Show>
  );
}
