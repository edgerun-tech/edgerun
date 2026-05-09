"use client"

import * as React from "react"

import { cn } from "@/lib/utils"

export type GridViewProps = React.ComponentProps<"div"> & {
  itemMinSize?: number
  minItemSize?: number
  maxItemSize?: number
  zoomStep?: number
  overscanRows?: number
  virtualized?: boolean
}

export function GridView({
  children,
  className,
  itemMinSize = 128,
  minItemSize = 72,
  maxItemSize = 1200,
  zoomStep = 12,
  overscanRows = 3,
  virtualized = true,
  onWheel,
  style,
  ...props
}: GridViewProps) {
  const [itemSize, setItemSize] = React.useState(itemMinSize)
  const [isHovered, setIsHovered] = React.useState(false)
  const [viewport, setViewport] = React.useState({ width: 0, height: 0, scrollTop: 0 })
  const rootRef = React.useRef<HTMLDivElement>(null)
  const contentRef = React.useRef<HTMLDivElement>(null)
  const items = React.Children.toArray(children)
  const gap = 16

  const zoomBy = React.useCallback(
    (direction: 1 | -1) => {
      setItemSize((current) =>
        Math.min(maxItemSize, Math.max(minItemSize, current + direction * zoomStep))
      )
    },
    [maxItemSize, minItemSize, zoomStep]
  )

  React.useEffect(() => {
    const node = rootRef.current
    if (!node) return

    const handleNativeWheel = (event: WheelEvent) => {
      if (!isHovered || (!event.ctrlKey && !event.altKey)) return
      event.preventDefault()
      event.stopPropagation()
      zoomBy(event.deltaY < 0 ? 1 : -1)
    }

    node.addEventListener("wheel", handleNativeWheel, {
      capture: true,
      passive: false,
    })

    return () => {
      node.removeEventListener("wheel", handleNativeWheel, { capture: true })
    }
  }, [isHovered, zoomBy])

  React.useEffect(() => {
    const root = rootRef.current
    if (!root) return

    const updateViewport = () => {
      const computedStyle = window.getComputedStyle(root)
      const horizontalPadding =
        Number.parseFloat(computedStyle.paddingLeft || "0") +
        Number.parseFloat(computedStyle.paddingRight || "0")
      setViewport({
        width: Math.max(0, root.clientWidth - horizontalPadding),
        height: root.clientHeight,
        scrollTop: root.scrollTop,
      })
    }

    updateViewport()
    const resizeObserver = new ResizeObserver(updateViewport)
    resizeObserver.observe(root)
    if (contentRef.current) resizeObserver.observe(contentRef.current)
    root.addEventListener("scroll", updateViewport, { passive: true })

    return () => {
      resizeObserver.disconnect()
      root.removeEventListener("scroll", updateViewport)
    }
  }, [])

  const handleWheel = React.useCallback(
    (event: React.WheelEvent<HTMLDivElement>) => {
      onWheel?.(event)
      if (event.defaultPrevented || !isHovered || !event.altKey || event.ctrlKey) return

      event.preventDefault()
      zoomBy(event.deltaY < 0 ? 1 : -1)
    },
    [isHovered, onWheel, zoomBy]
  )

  const handleKeyDown = React.useCallback(
    (event: React.KeyboardEvent<HTMLDivElement>) => {
      if (!event.ctrlKey && !event.metaKey) return
      if (event.key !== "=" && event.key !== "+" && event.key !== "-") return

      event.preventDefault()
      zoomBy(event.key === "-" ? -1 : 1)
    },
    [zoomBy]
  )

  const columns = Math.max(1, Math.floor((viewport.width + gap) / (itemSize + gap)))
  const columnWidth = viewport.width > 0
    ? (viewport.width - gap * (columns - 1)) / columns
    : itemSize
  const rowStride = columnWidth + gap
  const rowCount = Math.ceil(items.length / columns)
  const startRow = virtualized ? Math.max(0, Math.floor(viewport.scrollTop / rowStride) - overscanRows) : 0
  const endRow = virtualized
    ? Math.min(rowCount, Math.ceil((viewport.scrollTop + viewport.height) / rowStride) + overscanRows)
    : rowCount
  const startIndex = startRow * columns
  const endIndex = Math.min(items.length, endRow * columns)
  const visibleItems = items.slice(startIndex, endIndex)

  return (
    <div
      ref={rootRef}
      className={cn("overflow-auto", className)}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      onWheel={handleWheel}
      onKeyDown={handleKeyDown}
      tabIndex={0}
      style={
        {
          "--grid-view-item-size": `${itemSize}px`,
          ...style,
        } as React.CSSProperties
      }
      {...props}
    >
      {virtualized ? (
        <div
          ref={contentRef}
          className="relative w-full min-w-0"
          style={{ height: rowCount > 0 ? rowCount * rowStride - gap : 0 }}
        >
          {visibleItems.map((child, offset) => {
            const index = startIndex + offset
            const row = Math.floor(index / columns)
            const column = index % columns

            return (
              <div
                key={(child as React.ReactElement).key ?? index}
                className="absolute"
                style={{
                  top: row * rowStride,
                  left: column * (columnWidth + gap),
                  width: columnWidth,
                }}
              >
                {child}
              </div>
            )
          })}
        </div>
      ) : (
        <div ref={contentRef} className="grid grid-cols-[repeat(auto-fill,minmax(var(--grid-view-item-size),1fr))] gap-4">
          {children}
        </div>
      )}
    </div>
  )
}
