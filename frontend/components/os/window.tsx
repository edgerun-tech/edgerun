"use client"

import { useState, useRef, useEffect, type ReactNode, useCallback } from "react"
import { X, Minus, Maximize2, Minimize2 } from "lucide-react"
import { cn } from "@/lib/utils"
import { saveWindowLayout, getWindowLayout } from "@/stores/desktop-store"

interface WindowProps {
  id: string
  appId?: string
  title: string
  icon?: ReactNode
  children: ReactNode
  defaultPosition?: { x: number; y: number }
  defaultSize?: { width: number; height: number }
  minSize?: { width: number; height: number }
  onClose: () => void
  onFocus: () => void
  isFocused: boolean
  zIndex: number
  /** Stage Manager: hide this window (it's shown in the strip instead) */
  stageHidden?: boolean
  /** Stage Manager: constrain focused window to stage area */
  stageMode?: boolean
}

export function Window({
  id,
  appId,
  title,
  icon,
  children,
  defaultPosition = { x: 100, y: 100 },
  defaultSize = { width: 600, height: 400 },
  minSize = { width: 300, height: 200 },
  onClose,
  onFocus,
  isFocused,
  zIndex,
  stageHidden = false,
  stageMode = false,
}: WindowProps) {
  const savedLayout = appId ? getWindowLayout(appId) : null
  const [position, setPosition] = useState(savedLayout?.position ?? defaultPosition)
  const [size, setSize] = useState(savedLayout?.size ?? defaultSize)
  const [isMaximized, setIsMaximized] = useState(savedLayout?.maximized ?? false)
  const [isMinimized, setIsMinimized] = useState(false)
  const [isDragging, setIsDragging] = useState(false)
  const [isResizing, setIsResizing] = useState(false)
  const [isClosing, setIsClosing] = useState(false)
  const windowRef = useRef<HTMLDivElement>(null)
  const dragOffset = useRef({ x: 0, y: 0 })
  const resizeStart = useRef({ x: 0, y: 0, width: 0, height: 0 })
  const preMaximizeState = useRef({ position, size })
  const frameRef = useRef<number | null>(null)
  const pendingMouseRef = useRef<{ clientX: number; clientY: number } | null>(null)

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest("[data-window-controls]")) return
    onFocus()
    if (isMaximized) return

    setIsDragging(true)
    dragOffset.current = {
      x: e.clientX - position.x,
      y: e.clientY - position.y,
    }
  }, [onFocus, isMaximized, position.x, position.y])

  const handleResizeStart = useCallback((e: React.MouseEvent) => {
    e.stopPropagation()
    onFocus()
    if (isMaximized) return

    setIsResizing(true)
    resizeStart.current = {
      x: e.clientX,
      y: e.clientY,
      width: size.width,
      height: size.height,
    }
  }, [onFocus, isMaximized, size.width, size.height])

  useEffect(() => {
    const applyPointerFrame = () => {
      frameRef.current = null
      const next = pendingMouseRef.current
      if (!next) return

      if (isDragging) {
        const newX = Math.max(0, Math.min(window.innerWidth - 100, next.clientX - dragOffset.current.x))
        const newY = Math.max(0, Math.min(window.innerHeight - 50, next.clientY - dragOffset.current.y))
        setPosition({ x: newX, y: newY })
      }

      if (isResizing) {
        const deltaX = next.clientX - resizeStart.current.x
        const deltaY = next.clientY - resizeStart.current.y
        const newWidth = Math.max(minSize.width, resizeStart.current.width + deltaX)
        const newHeight = Math.max(minSize.height, resizeStart.current.height + deltaY)
        setSize({ width: newWidth, height: newHeight })
      }
    }

    const handleMouseMove = (e: MouseEvent) => {
      pendingMouseRef.current = { clientX: e.clientX, clientY: e.clientY }
      if (frameRef.current === null) {
        frameRef.current = requestAnimationFrame(applyPointerFrame)
      }
    }

    const handleMouseUp = () => {
      if (frameRef.current !== null) {
        cancelAnimationFrame(frameRef.current)
        frameRef.current = null
      }
      pendingMouseRef.current = null
      setIsDragging(false)
      setIsResizing(false)
      if (appId && !isMaximized) {
        saveWindowLayout(appId, { position: position, size, maximized: false })
      }
    }

    if (isDragging || isResizing) {
      document.addEventListener("mousemove", handleMouseMove)
      document.addEventListener("mouseup", handleMouseUp)
    }

    return () => {
      if (frameRef.current !== null) {
        cancelAnimationFrame(frameRef.current)
        frameRef.current = null
      }
      document.removeEventListener("mousemove", handleMouseMove)
      document.removeEventListener("mouseup", handleMouseUp)
    }
  }, [isDragging, isResizing, isMaximized, minSize.width, minSize.height, appId, position, size])

  const handleMaximize = () => {
    if (isMaximized) {
      setPosition(preMaximizeState.current.position)
      setSize(preMaximizeState.current.size)
      setIsMaximized(false)
      if (appId) saveWindowLayout(appId, { position: preMaximizeState.current.position, size: preMaximizeState.current.size, maximized: false })
    } else {
      preMaximizeState.current = { position, size }
      setPosition({ x: 0, y: 40 })
      setSize({ width: window.innerWidth, height: window.innerHeight - 40 })
      setIsMaximized(true)
      if (appId) saveWindowLayout(appId, { position: { x: 0, y: 40 }, size: { width: window.innerWidth, height: window.innerHeight - 40 }, maximized: true })
    }
  }

  const handleMinimize = () => {
    setIsMinimized(!isMinimized)
  }

  const handleClose = () => {
    setIsClosing(true)
    setTimeout(onClose, 150)
  }

  if (isMinimized || stageHidden) return null

  // In stage mode, clamp the focused window to stay inside the stage area (left of strip)
  const stageLeft = stageMode && isFocused ? 100 : undefined

  return (
    <div
      ref={windowRef}
      data-window-id={id}
      onClick={onFocus}
      className={cn(
        "absolute flex flex-col overflow-hidden rounded-lg border border-[var(--window-border)] bg-[var(--window-bg)] shadow-2xl",
        isFocused ? "window-focused" : "shadow-xl",
        isClosing ? "animate-window-close" : "animate-window-open",
        isDragging && "cursor-grabbing select-none",
        isResizing && "select-none"
      )}
      style={{
        left: stageLeft ?? position.x,
        top: position.y,
        width: size.width,
        height: size.height,
        zIndex,
        transition: isDragging || isResizing ? "none" : "box-shadow 0.2s ease, opacity 0.25s ease",
      }}
    >
      {/* Window Header */}
      <div
        onMouseDown={handleMouseDown}
        className={cn(
          "flex h-10 flex-shrink-0 cursor-grab items-center justify-between border-b border-[var(--window-border)] bg-[var(--window-header)] px-3",
          isDragging && "cursor-grabbing"
        )}
      >
        <div className="flex items-center gap-2">
          {icon && <span className="text-muted-foreground">{icon}</span>}
          <span className="text-sm font-medium text-foreground">{title}</span>
        </div>
        <div className="flex items-center gap-1" data-window-controls>
          <button
            onClick={handleMinimize}
            aria-label={`Minimize ${title}`}
            className="flex h-6 w-6 items-center justify-center rounded transition-colors hover:bg-secondary"
          >
            <Minus className="h-3.5 w-3.5 text-muted-foreground" />
          </button>
          <button
            onClick={handleMaximize}
            aria-label={isMaximized ? `Restore ${title}` : `Maximize ${title}`}
            className="flex h-6 w-6 items-center justify-center rounded transition-colors hover:bg-secondary"
          >
            {isMaximized ? (
              <Minimize2 className="h-3.5 w-3.5 text-muted-foreground" />
            ) : (
              <Maximize2 className="h-3.5 w-3.5 text-muted-foreground" />
            )}
          </button>
          <button
            onClick={handleClose}
            aria-label={`Close ${title}`}
            className="flex h-6 w-6 items-center justify-center rounded transition-colors hover:bg-destructive hover:text-destructive-foreground"
          >
            <X className="h-3.5 w-3.5 text-muted-foreground hover:text-destructive-foreground" />
          </button>
        </div>
      </div>

      {/* Window Content */}
      <div className="flex-1 overflow-auto">{children}</div>

      {/* Resize Handle */}
      {!isMaximized && (
        <div
          onMouseDown={handleResizeStart}
          className="absolute bottom-0 right-0 h-4 w-4 cursor-se-resize"
          style={{
            background: "linear-gradient(135deg, transparent 50%, var(--border) 50%)",
          }}
        />
      )}
    </div>
  )
}
