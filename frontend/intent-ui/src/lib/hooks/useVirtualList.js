import { createMemo, createSignal, createEffect, onCleanup } from "solid-js";

/**
 * @typedef {object} VirtualListOptions
 * @property {number | (() => number)} count
 * @property {number} estimateSize
 * @property {number=} overscan
 * @property {() => HTMLElement | undefined=} containerRef
 * @property {number | (() => number)=} scrollTop
 * @property {number | (() => number)=} viewportHeight
 */

/** @param {VirtualListOptions} options */
function createVirtualList(options) {
  const [internalScrollTop, setInternalScrollTop] = createSignal(0);
  const [internalViewportHeight, setInternalViewportHeight] = createSignal(0);

  createEffect(() => {
    const refResolver = options.containerRef;
    const element = typeof refResolver === "function" ? refResolver() : null;
    if (!element) return;
    const update = () => {
      setInternalScrollTop(Math.max(0, element.scrollTop || 0));
      setInternalViewportHeight(Math.max(0, element.clientHeight || 0));
    };
    update();
    element.addEventListener("scroll", update, { passive: true });
    window.addEventListener("resize", update);
    onCleanup(() => {
      element.removeEventListener("scroll", update);
      window.removeEventListener("resize", update);
    });
  });

  const resolveCount = () => {
    const value = typeof options.count === "function" ? options.count() : options.count;
    return Number.isFinite(value) && value > 0 ? Math.floor(value) : 0;
  };
  const resolveScrollTop = () => {
    const value = typeof options.scrollTop === "function" ? options.scrollTop() : options.scrollTop;
    if (Number.isFinite(value)) return Math.max(0, Number(value));
    return internalScrollTop();
  };
  const resolveViewportHeight = () => {
    const value = typeof options.viewportHeight === "function" ? options.viewportHeight() : options.viewportHeight;
    if (Number.isFinite(value)) return Math.max(0, Number(value));
    return internalViewportHeight();
  };
  const estimateSize = Math.max(1, Number(options.estimateSize || 1));
  const overscan = Math.max(0, Number(options.overscan || 0));
  const range = createMemo(() => {
    const count = resolveCount();
    if (count <= 0) return { start: 0, end: 0 };
    const viewport = resolveViewportHeight();
    if (viewport <= 0) return { start: 0, end: count };
    const scroll = resolveScrollTop();
    const start = Math.max(0, Math.floor(scroll / estimateSize) - overscan);
    const visibleCount = Math.ceil(viewport / estimateSize) + overscan * 2;
    const end = Math.min(count, Math.max(start + 1, start + visibleCount));
    return { start, end };
  });
  const virtualItems = createMemo(() => {
    const { start, end } = range();
    return Array.from({ length: Math.max(0, end - start) }, (_, offset) => {
      const index = start + offset;
      const startOffset = index * estimateSize;
      return {
      index,
      offset: startOffset,
      start: startOffset,
      size: estimateSize,
      end: startOffset + estimateSize
    };
    });
  });
  const totalHeight = createMemo(() => resolveCount() * estimateSize);
  const virtualTopPad = createMemo(() => range().start * estimateSize);
  const virtualBottomPad = createMemo(() => {
    const count = resolveCount();
    return Math.max(0, (count - range().end) * estimateSize);
  });
  return { virtualItems, totalHeight, virtualTopPad, virtualBottomPad };
}
export {
  createVirtualList
};
