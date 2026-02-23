/**
 * Simple virtual list hook for GmailPanel
 * Lightweight alternative to @tanstack/solid-virtual
 */

import { createSignal, createMemo, onMount, onCleanup, Accessor } from 'solid-js';

interface VirtualListOptions {
  count: number | Accessor<number>;
  estimateSize: number;
  overscan?: number;
  containerRef: () => HTMLDivElement | undefined;
}

export function createVirtualList(options: VirtualListOptions) {
  const { estimateSize, overscan = 5, containerRef } = options;
  const countAccessor: Accessor<number> = typeof options.count === 'function' 
    ? options.count as Accessor<number>
    : () => options.count as number;

  const [scrollTop, setScrollTop] = createSignal(0);
  const [containerHeight, setContainerHeight] = createSignal(0);

  const totalHeight = createMemo(() => countAccessor() * estimateSize);

  const startIndex = createMemo(() => {
    const index = Math.floor(scrollTop() / estimateSize);
    return Math.max(0, index - overscan);
  });

  const endIndex = createMemo(() => {
    const index = Math.floor((scrollTop() + containerHeight()) / estimateSize);
    return Math.min(countAccessor(), index + overscan);
  });

  const virtualItems = createMemo(() => {
    const items: { index: number; offset: number; size: number }[] = [];
    for (let i = startIndex(); i < endIndex(); i++) {
      items.push({
        index: i,
        offset: i * estimateSize,
        size: estimateSize,
      });
    }
    return items;
  });

  const handleScroll = (e: Event) => {
    const target = e.target as HTMLDivElement;
    setScrollTop(target.scrollTop);
  };

  const updateContainerHeight = () => {
    const container = containerRef();
    if (container) {
      setContainerHeight(container.clientHeight);
    }
  };

  onMount(() => {
    updateContainerHeight();
    const container = containerRef();
    if (container) {
      container.addEventListener('scroll', handleScroll, { passive: true });
      window.addEventListener('resize', updateContainerHeight);
    }
  });

  onCleanup(() => {
    const container = containerRef();
    if (container) {
      container.removeEventListener('scroll', handleScroll);
    }
    window.removeEventListener('resize', updateContainerHeight);
  });

  return {
    virtualItems,
    totalHeight,
  };
}
