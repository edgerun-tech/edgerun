import { For, Show } from "solid-js";
import { Motion } from "solid-motionone";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

function cn(...classes) {
  return twMerge(clsx(classes));
}

function parseTiles(data) {
  if (Array.isArray(data)) return data;
  if (Array.isArray(data?.tiles)) return data.tiles;
  return [];
}

function toneClasses(tone) {
  if (tone === "success") return "border-emerald-500/35 bg-emerald-500/10 text-emerald-100";
  if (tone === "warning") return "border-amber-500/35 bg-amber-500/10 text-amber-100";
  if (tone === "danger") return "border-rose-500/35 bg-rose-500/10 text-rose-100";
  return "border-blue-500/25 bg-blue-500/10 text-blue-100";
}

function BentoGrid(props) {
  const ui = () => props.response?.ui;
  const tiles = () => parseTiles(props.response?.data);

  return (
    <Motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -8 }}
      transition={{ duration: 0.2 }}
      class={cn("overflow-hidden rounded-xl border border-neutral-700 bg-neutral-900/60", props.class)}
    >
      <div class="border-b border-neutral-700 px-4 py-3">
        <h3 class="text-sm font-medium text-white">{ui()?.title || "Bento Grid"}</h3>
        <Show when={ui()?.description}>
          <p class="mt-1 text-xs text-neutral-400">{ui().description}</p>
        </Show>
      </div>

      <div class="p-3">
        <div class="grid auto-rows-[82px] grid-cols-6 gap-2">
          <For each={tiles()}>
            {(tile) => (
              <article
                class={cn(
                  "rounded-lg border p-3",
                  tile?.span || "col-span-3",
                  toneClasses(tile?.tone)
                )}
              >
                <p class="text-[11px] uppercase tracking-wide text-neutral-300/90">{tile?.title || "Signal"}</p>
                <p class="mt-2 text-lg font-semibold leading-tight">{tile?.value ?? "--"}</p>
                <Show when={tile?.note}>
                  <p class="mt-1 text-xs text-neutral-300/90">{tile.note}</p>
                </Show>
              </article>
            )}
          </For>
        </div>
      </div>
    </Motion.div>
  );
}

export { BentoGrid };
