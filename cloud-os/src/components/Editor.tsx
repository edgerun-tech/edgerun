import { createSignal, createEffect } from 'solid-js';

type Props = {
  value?: string;
  path?: string;
  onChange?: (value: string) => void;
};

export default function Editor(props: Props) {
  const [text, setText] = createSignal(props.value || '');

  createEffect(() => {
    setText(props.value || '');
  });

  return (
    <div class="h-full w-full bg-[#111] text-neutral-100 p-3 flex flex-col gap-2">
      <div class="text-xs text-neutral-400">{props.path || 'Untitled file'}</div>
      <textarea
        class="flex-1 w-full rounded border border-neutral-700 bg-black/40 p-3 font-mono text-sm outline-none"
        value={text()}
        onInput={(e) => {
          const value = e.currentTarget.value;
          setText(value);
          props.onChange?.(value);
        }}
      />
    </div>
  );
}
