import { For, Show, createMemo } from "solid-js";
import { Motion } from "solid-motionone";
import { createVirtualList } from "../../lib/hooks/useVirtualList";

function VirtualAnimatedList(props) {
  const items = createMemo(() => {
    const value = typeof props.items === "function" ? props.items() : props.items;
    return Array.isArray(value) ? value : [];
  });
  const {
    virtualItems,
    totalHeight,
    virtualTopPad,
    virtualBottomPad
  } = createVirtualList({
    count: () => items().length,
    estimateSize: Number(props.estimateSize || 80),
    overscan: Number(props.overscan || 4),
    containerRef: props.containerRef,
    scrollTop: props.scrollTop,
    viewportHeight: props.viewportHeight
  });

  const renderRow = (row) => {
    const item = items()[row.index];
    if (!item) return null;
    const content = props.renderItem?.(item, row.index, row);
    if (!props.animateRows) return content;
    return (
      <Motion.div
        initial={{ opacity: 0, y: 6 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.14 }}
      >
        {content}
      </Motion.div>
    );
  };

  return (
    <Show
      when={props.layout === "absolute"}
      fallback={
        <>
          <div style={{ height: `${virtualTopPad()}px` }} />
          <For each={virtualItems()}>{(row) => renderRow(row)}</For>
          <div style={{ height: `${virtualBottomPad()}px` }} />
        </>
      }
    >
      <div class={props.class} style={{ height: `${totalHeight()}px` }}>
        <For each={virtualItems()}>
          {(row) => (
            <div
              class={props.rowClass}
              data-index={row.index}
              style={{ transform: `translateY(${row.offset}px)` }}
            >
              {renderRow(row)}
            </div>
          )}
        </For>
      </div>
    </Show>
  );
}

export default VirtualAnimatedList;
