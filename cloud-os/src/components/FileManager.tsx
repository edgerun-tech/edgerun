import { createSignal, onMount, For } from 'solid-js';

type FileEntry = {
  id?: string;
  name: string;
  type: 'file' | 'folder';
  size?: number;
  modified?: string;
};

export default function FileManager() {
  const [files, setFiles] = createSignal<FileEntry[]>([]);
  const [error, setError] = createSignal('');

  onMount(async () => {
    try {
      const res = await fetch('/api/fs/?path=/');
      const data = await res.json();
      setFiles(Array.isArray(data.files) ? data.files : []);
    } catch (e: any) {
      setError(e?.message || 'Failed to load files');
    }
  });

  return (
    <div class="h-full w-full bg-[#151515] text-neutral-100 p-3">
      <h2 class="text-sm font-semibold mb-3">File Manager</h2>
      {error() ? <p class="text-xs text-rose-300">{error()}</p> : null}
      <div class="space-y-1 overflow-auto max-h-[70vh]">
        <For each={files()}>{(entry) => (
          <div class="rounded border border-neutral-800 px-2 py-1 text-sm">
            <span class="mr-2 text-neutral-500">{entry.type === 'folder' ? '[DIR]' : '[FILE]'}</span>
            <span>{entry.name}</span>
          </div>
        )}</For>
      </div>
    </div>
  );
}
