import { Show, createEffect, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import VirtualAnimatedList from "../common/VirtualAnimatedList";

const DEFAULT_MIN_WIDTH = 260;
const DEFAULT_MIN_HEIGHT = 150;

function clamp(value, min, max) {
  return Math.min(Math.max(value, min), max);
}

function normalizeLayout(layout, minWidth, minHeight) {
  const width = Math.max(minWidth, Number(layout?.width || minWidth));
  const height = Math.max(minHeight, Number(layout?.height || minHeight));
  const viewportWidth = typeof window === "undefined" ? width : window.innerWidth;
  const viewportHeight = typeof window === "undefined" ? height : window.innerHeight;
  const x = clamp(Number(layout?.x || 16), 0, Math.max(0, viewportWidth - width));
  const y = clamp(Number(layout?.y || 16), 0, Math.max(0, viewportHeight - height));
  return { x, y, width, height };
}

function FloatingFeedPanel(props) {
  if (typeof window === "undefined") return null;
  const panelId = String(props.panelId || "").trim() || "panel";
  const storageKey = `intent-ui-floating-panel-layout-${panelId}-v1`;
  const minWidth = Number(props.minWidth || DEFAULT_MIN_WIDTH);
  const minHeight = Number(props.minHeight || DEFAULT_MIN_HEIGHT);
  const maxItems = Number(props.maxItems || 20);
  const [layout, setLayout] = createSignal(normalizeLayout(props.defaultLayout || {}, minWidth, minHeight));
  const [dragging, setDragging] = createSignal(false);
  const [resizing, setResizing] = createSignal(false);
  const entries = createMemo(() => {
    const list = props.entries ? props.entries() : [];
    return Array.isArray(list) ? list.slice(0, maxItems) : [];
  });
  const onMinimize = typeof props.onMinimize === "function" ? props.onMinimize : null;

  let dragStartMouseX = 0;
  let dragStartMouseY = 0;
  let dragStartX = 0;
  let dragStartY = 0;
  let resizeStartMouseX = 0;
  let resizeStartMouseY = 0;
  let resizeStartWidth = 0;
  let resizeStartHeight = 0;
  let scrollAreaRef;

  const persistLayout = (next) => {
    try {
      localStorage.setItem(storageKey, JSON.stringify(next));
    } catch {
      // ignore persistence failures
    }
  };

  const loadLayout = () => {
    try {
      const parsed = JSON.parse(localStorage.getItem(storageKey) || "null");
      if (parsed && typeof parsed === "object") {
        setLayout(normalizeLayout(parsed, minWidth, minHeight));
        return;
      }
    } catch {
      // ignore parse failures
    }
    setLayout(normalizeLayout(props.defaultLayout || {}, minWidth, minHeight));
  };

  const onPointerMove = (event) => {
    if (dragging()) {
      const nextX = dragStartX + (event.clientX - dragStartMouseX);
      const nextY = dragStartY + (event.clientY - dragStartMouseY);
      setLayout((prev) => normalizeLayout({ ...prev, x: nextX, y: nextY }, minWidth, minHeight));
      return;
    }
    if (resizing()) {
      const nextWidth = resizeStartWidth + (event.clientX - resizeStartMouseX);
      const nextHeight = resizeStartHeight + (event.clientY - resizeStartMouseY);
      setLayout((prev) => normalizeLayout({ ...prev, width: nextWidth, height: nextHeight }, minWidth, minHeight));
    }
  };

  const onPointerUp = () => {
    if (dragging() || resizing()) {
      persistLayout(layout());
    }
    setDragging(false);
    setResizing(false);
  };

  const onWindowResize = () => {
    setLayout((prev) => {
      const next = normalizeLayout(prev, minWidth, minHeight);
      persistLayout(next);
      return next;
    });
  };

  onMount(() => {
    loadLayout();
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
    window.addEventListener("resize", onWindowResize);
  });

  onCleanup(() => {
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("pointerup", onPointerUp);
    window.removeEventListener("resize", onWindowResize);
  });

  createEffect(() => {
    if (!dragging() && !resizing()) {
      persistLayout(layout());
    }
  });

  return (
    <div
      class="pointer-events-none fixed z-[10002]"
      style={{
        left: `${layout().x}px`,
        top: `${layout().y}px`,
        width: `${layout().width}px`,
        height: `${layout().height}px`
      }}
      data-testid={`floating-feed-panel-${panelId}`}
    >
      <div class="pointer-events-auto flex h-full flex-col overflow-hidden rounded-xl border border-neutral-800/85 bg-[#101116]/84 shadow-[0_18px_38px_rgba(0,0,0,0.42)] backdrop-blur-xl">
        <div class="flex items-center border-b border-neutral-800/80" data-testid={`floating-feed-panel-header-${panelId}`}>
          <button
            type="button"
            class="min-w-0 flex-1 cursor-move px-3 py-2 text-left text-[10px] uppercase tracking-wide text-white"
            onPointerDown={(event) => {
              event.preventDefault();
              setDragging(true);
              dragStartMouseX = event.clientX;
              dragStartMouseY = event.clientY;
              dragStartX = layout().x;
              dragStartY = layout().y;
            }}
            data-testid={`floating-feed-panel-drag-${panelId}`}
          >
            {String(props.title || "").toUpperCase()}
          </button>
          <Show when={onMinimize}>
            <button
              type="button"
              class="mr-1 rounded border border-neutral-700 bg-neutral-900/70 px-1.5 py-0.5 text-[10px] text-neutral-200 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
              title="Minimize panel"
              aria-label={`Minimize ${panelId} panel`}
              onClick={() => onMinimize?.(panelId)}
              data-testid={`floating-feed-panel-minimize-${panelId}`}
            >
              _
            </button>
          </Show>
        </div>
        <div
          class="h-full overflow-y-auto px-2 py-2 text-white"
          ref={scrollAreaRef}
          style={{
            "mask-image": "linear-gradient(to bottom, rgba(0,0,0,1) 0%, rgba(0,0,0,1) 32%, rgba(0,0,0,0) 100%)",
            "-webkit-mask-image": "linear-gradient(to bottom, rgba(0,0,0,1) 0%, rgba(0,0,0,1) 32%, rgba(0,0,0,0) 100%)"
          }}
          data-testid={`floating-feed-panel-scroll-${panelId}`}
        >
          <Show when={entries().length > 0} fallback={<p class="text-[11px] text-white">{props.emptyLabel || "No events yet."}</p>}>
            <div class="space-y-1">
              <VirtualAnimatedList
                items={entries}
                estimateSize={24}
                overscan={5}
                containerRef={() => scrollAreaRef}
                animateRows
                renderItem={(entry) => (
                  <div class="text-white">
                    {props.renderEntry ? props.renderEntry(entry) : <p class="truncate text-[10px] text-white">{String(entry)}</p>}
                  </div>
                )}
              />
            </div>
          </Show>
        </div>
      </div>
      <button
        type="button"
        class="pointer-events-auto absolute bottom-1.5 right-1.5 h-3 w-3 cursor-se-resize rounded-sm border border-neutral-700 bg-neutral-900/80"
        title="Resize panel"
        onPointerDown={(event) => {
          event.preventDefault();
          setResizing(true);
          resizeStartMouseX = event.clientX;
          resizeStartMouseY = event.clientY;
          resizeStartWidth = layout().width;
          resizeStartHeight = layout().height;
        }}
        data-testid={`floating-feed-panel-resizer-${panelId}`}
      />
    </div>
  );
}

export default FloatingFeedPanel;
