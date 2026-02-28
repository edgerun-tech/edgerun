import { Show } from "solid-js";
import { Kbd, KbdGroup } from "../../registry/ui/kbd";

function LayerIndicator(props) {
  return (
    <Show when={props.visible}>
      <div class="pointer-events-none fixed bottom-3 left-3 z-[12000]">
        <div class="flex items-center gap-2 rounded-lg border border-neutral-800/80 bg-[#0f0f0f]/70 px-2 py-1.5 shadow-lg backdrop-blur-xl">
          <KbdGroup class="gap-1">
            <Kbd
              class={`transition ${props.layer === 0 ? "border-[hsl(var(--primary))]/60 bg-[hsl(var(--primary))]/20 text-[hsl(var(--primary))] opacity-100" : "border-neutral-700 text-neutral-300 opacity-50"}`}
            >
              0
            </Kbd>
            <Kbd
              class={`transition ${props.layer === 1 ? "border-[hsl(var(--primary))]/60 bg-[hsl(var(--primary))]/20 text-[hsl(var(--primary))] opacity-100" : "border-neutral-700 text-neutral-300 opacity-50"}`}
            >
              1
            </Kbd>
            <Kbd
              class={`transition ${props.layer === 2 ? "border-[hsl(var(--primary))]/60 bg-[hsl(var(--primary))]/20 text-[hsl(var(--primary))] opacity-100" : "border-neutral-700 text-neutral-300 opacity-50"}`}
            >
              2
            </Kbd>
          </KbdGroup>
          <span class="text-[10px] uppercase tracking-wide text-neutral-500">Layer</span>
        </div>
      </div>
    </Show>
  );
}

export default LayerIndicator;
