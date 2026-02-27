import { createMemo } from "solid-js";

/**
 * @typedef {object} VirtualListOptions
 * @property {number | (() => number)} count
 * @property {number} estimateSize
 * @property {number=} overscan
 */

/** @param {VirtualListOptions} options */
function createVirtualList(options) {
  const resolveCount = () => {
    const value = typeof options.count === "function" ? options.count() : options.count;
    return Number.isFinite(value) && value > 0 ? Math.floor(value) : 0;
  };
  const virtualItems = createMemo(() => {
    const count = resolveCount();
    return Array.from({ length: count }, (_, index) => ({
      index,
      start: index * options.estimateSize,
      size: options.estimateSize,
      end: (index + 1) * options.estimateSize
    }));
  });
  const totalHeight = createMemo(() => resolveCount() * options.estimateSize);
  return { virtualItems, totalHeight };
}
export {
  createVirtualList
};
