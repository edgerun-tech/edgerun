import { For, createEffect, createMemo, createSignal, onCleanup } from "solid-js";
import { Motion } from "solid-motionone";
import { createVirtualList } from "../../lib/hooks/useVirtualList";

function VirtualAnimatedGrid(props) {
  const items = createMemo(() => {
    const value = typeof props.items === "function" ? props.items() : props.items;
    return Array.isArray(value) ? value : [];
  });
  const gap = Math.max(0, Number(props.gap || 8));
  const minColumnWidth = Math.max(80, Number(props.minColumnWidth || 180));
  const [containerWidth, setContainerWidth] = createSignal(0);

  createEffect(() => {
    const resolver = props.containerRef;
    const element = typeof resolver === "function" ? resolver() : null;
    if (!element) return;
    const update = () => setContainerWidth(Math.max(0, element.clientWidth || 0));
    update();
    window.addEventListener("resize", update);
    let resizeObserver;
    if (typeof ResizeObserver !== "undefined") {
      resizeObserver = new ResizeObserver(update);
      resizeObserver.observe(element);
    }
    onCleanup(() => {
      window.removeEventListener("resize", update);
      if (resizeObserver) resizeObserver.disconnect();
    });
  });

  const columnCount = createMemo(() => {
    const width = containerWidth();
    if (width <= 0) return Number(props.fallbackColumns || 2);
    return Math.max(1, Math.floor((width + gap) / (minColumnWidth + gap)));
  });

  const rowCount = createMemo(() => {
    const cols = columnCount();
    return Math.ceil(items().length / cols);
  });

  const virtual = createVirtualList({
    count: rowCount,
    estimateSize: Number(props.estimateRowHeight || 200),
    overscan: Number(props.overscan || 2),
    containerRef: props.containerRef
  });

  const rows = createMemo(() => {
    const cols = columnCount();
    return virtual.virtualItems().map((row) => {
      const start = row.index * cols;
      const end = Math.min(start + cols, items().length);
      return {
        ...row,
        items: items().slice(start, end),
        start
      };
    });
  });

  return (
    <>
      <div style={{ height: `${virtual.virtualTopPad()}px` }} />
      <For each={rows()}>
        {(row) => (
          <Motion.div
            initial={props.animateRows ? { opacity: 0, y: 6 } : void 0}
            animate={props.animateRows ? { opacity: 1, y: 0 } : void 0}
            transition={props.animateRows ? { duration: 0.14 } : void 0}
            class={props.rowClass}
          >
            <div
              class={props.gridClass}
              style={{
                display: "grid",
                "grid-template-columns": `repeat(${columnCount()}, minmax(0, 1fr))`,
                gap: `${gap}px`
              }}
            >
              <For each={row.items}>
                {(item, idx) => props.renderItem?.(item, row.start + idx(), row)}
              </For>
            </div>
          </Motion.div>
        )}
      </For>
      <div style={{ height: `${virtual.virtualBottomPad()}px` }} />
    </>
  );
}

export default VirtualAnimatedGrid;
