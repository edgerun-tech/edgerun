import { useCallback, useState } from "react"
import { cn } from "@/lib/utils"
import type { UINode } from "./protobuf-ui"

interface UIRendererProps {
  node: UINode | null
  onAction: (action: string) => void
  className?: string
}

export function UIRenderer({ node, onAction, className }: UIRendererProps) {
  const [activeTab, setActiveTab] = useState<string>(node?.props?.active ?? "")

  const renderNode = useCallback(
    (n: UINode | null, keyHint: string) => {
      if (!n) return null
      const ctx: RenderCtx = { onAction, renderNode, activeTab, setActiveTab }
      switch (n.type) {
        case "column":
          return renderColumn(n, ctx, keyHint)
        case "row":
          return renderRow(n, ctx, keyHint)
        case "grid":
          return renderGrid(n, ctx, keyHint)
        case "scroll":
          return renderScroll(n, ctx, keyHint)
        case "spacer":
          return renderSpacer(n, keyHint)
        case "divider":
          return renderDivider(n, keyHint)
        case "text":
          return renderText(n, keyHint)
        case "heading":
          return renderHeading(n, keyHint)
        case "paragraph":
          return renderParagraph(n, keyHint)
        case "button":
          return renderButton(n, ctx, keyHint)
        case "input":
          return renderInput(n, ctx, keyHint)
        case "link":
          return renderLink(n, ctx, keyHint)
        case "card":
          return renderCard(n, ctx, keyHint)
        case "avatar":
          return renderAvatar(n, keyHint)
        case "badge":
          return renderBadge(n, keyHint)
        case "tabs":
          return renderTabs(n, ctx, keyHint)
        case "tab":
          return renderTab(n, ctx, keyHint)
        case "tab_content":
          return renderTabContent(n, ctx, keyHint)
        case "list":
          return renderList(n, ctx, keyHint)
        case "list_item":
          return renderListItem(n, ctx, keyHint)
        case "image":
          return renderImage(n, keyHint)
        case "icon":
          return renderIcon(n, keyHint)
        default:
          return renderUnknown(n, keyHint)
      }
    },
    [onAction, activeTab]
  )

  if (!node) return null

  return (
    <div className={cn("h-full w-full", className)}>
      {renderNode(node, "root")}
    </div>
  )
}

interface RenderCtx {
  onAction: (action: string) => void
  renderNode: (n: UINode | null, keyHint: string) => React.ReactNode
  activeTab: string
  setActiveTab: (id: string) => void
}

// ── Layout ──────────────────────────────────────────────

function renderColumn(node: UINode, ctx: RenderCtx, keyHint: string) {
  return (
    <div
      key={keyHint}
      className="flex h-full flex-col"
    >
      {node.children?.map((child, i) => ctx.renderNode(child, `${keyHint}-${i}`))}
    </div>
  )
}

function renderRow(node: UINode, ctx: RenderCtx, keyHint: string) {
  return (
    <div
      key={keyHint}
      className="flex flex-row gap-px bg-[var(--window-border)]"
    >
      {node.children?.map((child, i) => ctx.renderNode(child, `${keyHint}-${i}`))}
    </div>
  )
}

function renderGrid(node: UINode, ctx: RenderCtx, keyHint: string) {
  const cols = node.props?.cols ?? "2"
  const gap = node.props?.gap ?? "3"
  const padding = node.props?.padding
  const bgOnGap = node.props?.gap === "px"
  return (
    <div
      key={keyHint}
      className={cn(
        "grid",
        `grid-cols-${cols}`,
        `gap-${gap}`,
        bgOnGap && "bg-[var(--window-border)]",
        padding && `p-${padding}`,
        node.props?.className
      )}
    >
      {node.children?.map((child, i) => ctx.renderNode(child, `${keyHint}-${i}`))}
    </div>
  )
}

function renderScroll(node: UINode, ctx: RenderCtx, keyHint: string) {
  return (
    <div
      key={keyHint}
      className={cn("flex-1 overflow-auto", node.props?.className)}
    >
      {node.children?.map((child, i) => ctx.renderNode(child, `${keyHint}-${i}`))}
    </div>
  )
}

function renderSpacer(node: UINode, keyHint: string) {
  const height = parseInt(node.props?.height || "8", 10)
  return <div key={keyHint} style={{ height: `${height}px` }} />
}

function renderDivider(node: UINode, keyHint: string) {
  return (
    <div
      key={keyHint}
      className={cn("h-px w-full bg-[var(--window-border)]", node.props?.className)}
    />
  )
}

// ── Content ─────────────────────────────────────────────

function renderText(node: UINode, keyHint: string) {
  const value = node.props?.value || ""
  const variant = node.props?.variant ?? "sm"
  const color = node.props?.color ?? "foreground"

  const variantClass: Record<string, string> = {
    xs: "text-[10px]",
    sm: "text-xs",
    base: "text-sm",
    lg: "text-base",
    mono: "font-mono text-xs",
  }

  const colorClass: Record<string, string> = {
    foreground: "text-foreground",
    muted: "text-muted-foreground",
    primary: "text-primary",
    error: "text-[var(--status-error)]",
    warning: "text-[var(--status-warning)]",
    online: "text-[var(--status-online)]",
  }

  const opacity = node.props?.opacity
  const style = opacity ? { opacity: parseFloat(opacity) } : undefined

  return (
    <span
      key={keyHint}
      className={cn(
        "truncate",
        variantClass[variant] ?? "text-sm",
        colorClass[color] ?? "text-foreground",
      )}
      style={style}
    >
      {value}
    </span>
  )
}

function renderHeading(node: UINode, keyHint: string) {
  const value = node.props?.value || ""
  const level = parseInt(node.props?.level || "2", 10)
  const clamped = Math.min(Math.max(level, 1), 6)
  const tags = ["h1", "h2", "h3", "h4", "h5", "h6"] as const
  const Tag = tags[clamped - 1]

  const sizeMap: Record<number, string> = {
    1: "text-xl font-bold",
    2: "text-lg font-semibold",
    3: "text-base font-semibold",
    4: "text-sm font-semibold",
    5: "text-xs font-semibold uppercase tracking-wider",
    6: "text-[10px] font-semibold uppercase tracking-widest",
  }

  // Override size if explicit size prop provided (for calc display)
  const sizeOverride = node.props?.size
  const sizeClass = sizeOverride
    ? sizeOverride === "2"
      ? "text-2xl"
      : sizeOverride === "3"
        ? "text-3xl"
        : sizeOverride === "4"
          ? "text-4xl"
          : sizeMap[clamped] ?? "text-lg font-semibold"
    : sizeMap[clamped] ?? "text-lg font-semibold"

  const isMono = node.props?.variant === "mono"

  return (
    <Tag
      key={keyHint}
      className={cn(
        "text-right font-light tabular-nums text-foreground leading-none truncate",
        sizeClass,
        isMono && "font-mono",
      )}
    >
      {value}
    </Tag>
  )
}

function renderParagraph(node: UINode, keyHint: string) {
  const value = node.props?.value || ""
  return (
    <p
      key={keyHint}
      className={cn("text-sm leading-relaxed text-muted-foreground", node.props?.className)}
    >
      {value}
    </p>
  )
}

// ── Interactive ─────────────────────────────────────────

function renderButton(node: UINode, ctx: RenderCtx, keyHint: string) {
  const label = node.props?.label || "button"
  const variant = node.props?.variant ?? "primary"
  const size = node.props?.size ?? "md"

  const variantClass: Record<string, string> = {
    primary: "bg-primary text-primary-foreground hover:bg-primary/90 font-semibold",
    secondary: "bg-primary/20 text-primary hover:bg-primary/30 font-medium",
    ghost: "bg-secondary text-muted-foreground hover:bg-secondary/70 hover:text-foreground",
    destructive: "bg-[var(--status-error)]/15 text-[var(--status-error)] hover:bg-[var(--status-error)]/25",
    outline: "bg-[oklch(0.16_0.005_280)] text-foreground hover:bg-[oklch(0.2_0.005_280)]",
    success: "bg-[var(--status-online)]/15 text-[var(--status-online)] hover:bg-[var(--status-online)]/25",
  }

  const sizeClass: Record<string, string> = {
    xs: "h-6 rounded-md px-2 text-[10px]",
    sm: "h-7 rounded-md px-2.5 text-xs",
    md: "h-8 rounded-md px-3 text-sm",
    lg: "h-14 rounded-none text-lg",
  }

  return (
    <button
      key={keyHint}
      className={cn(
        "inline-flex items-center justify-center gap-1.5 font-medium transition-all active:scale-95",
        variantClass[variant] ?? variantClass.primary,
        sizeClass[size] ?? sizeClass.md,
      )}
      onClick={() => node.action && ctx.onAction(node.action)}
    >
      {label}
    </button>
  )
}

function renderInput(node: UINode, ctx: RenderCtx, keyHint: string) {
  const placeholder = node.props?.placeholder ?? ""
  const label = node.props?.label
  const type = node.props?.type ?? "text"
  const multiline = node.props?.multiline === "true"

  return (
    <div key={keyHint} className={cn("flex flex-col gap-1.5", node.props?.className)}>
      {label && (
        <label className="text-xs font-medium text-foreground">{label}</label>
      )}
      {multiline ? (
        <textarea
          placeholder={placeholder}
          rows={3}
          className={cn(
            "w-full rounded-md border border-border bg-secondary px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground outline-none transition-colors focus:border-primary/50 focus:ring-1 focus:ring-primary/50",
          )}
          onKeyDown={(e) => {
            if (e.key === "Enter" && e.ctrlKey && node.action) {
              const ta = e.target as HTMLTextAreaElement
              ctx.onAction(`input:${ta.value}`)
              ctx.onAction(node.action!)
            }
          }}
        />
      ) : (
        <input
          type={type}
          placeholder={placeholder}
          className={cn(
            "h-8 w-full rounded-md border border-border bg-secondary px-3 text-sm text-foreground placeholder:text-muted-foreground outline-none transition-colors focus:border-primary/50 focus:ring-1 focus:ring-primary/50",
          )}
          onKeyDown={(e) => {
            if (e.key === "Enter" && node.action) {
              const input = e.target as HTMLInputElement
              ctx.onAction(`input:${input.value}`)
              ctx.onAction(node.action!)
            }
          }}
        />
      )}
    </div>
  )
}

function renderLink(node: UINode, ctx: RenderCtx, keyHint: string) {
  const label = node.props?.label || "link"
  return (
    <button
      key={keyHint}
      className={cn(
        "inline-flex items-center text-sm text-primary underline-offset-4 transition-colors hover:underline",
        node.props?.className
      )}
      onClick={() => node.action && ctx.onAction(node.action)}
    >
      {label}
    </button>
  )
}

// ── Components ──────────────────────────────────────────

function renderCard(node: UINode, ctx: RenderCtx, keyHint: string) {
  const title = node.props?.title
  return (
    <div
      key={keyHint}
      className="flex-1 flex flex-col justify-end bg-[oklch(0.07_0.005_280)] px-5 py-4 min-h-0"
    >
      {title && (
        <div className="mb-2 border-b border-[var(--window-border)] pb-2">
          <span className="text-xs font-semibold text-foreground">{title}</span>
        </div>
      )}
      {node.children?.map((child, i) => ctx.renderNode(child, `${keyHint}-${i}`))}
    </div>
  )
}

function renderAvatar(node: UINode, keyHint: string) {
  const name = node.props?.name || "?"
  const size = node.props?.size ?? "sm"
  const initials = name
    .split(" ")
    .map((n) => n[0])
    .join("")
    .slice(0, 2)
    .toUpperCase()
  const hue = name.split("").reduce((acc, c) => acc + c.charCodeAt(0), 0) % 360

  const sizeClass: Record<string, string> = {
    xs: "h-6 w-6 text-[9px]",
    sm: "h-8 w-8 text-xs",
    md: "h-10 w-10 text-sm",
    lg: "h-12 w-12 text-base",
  }

  return (
    <div
      key={keyHint}
      className={cn(
        "flex flex-shrink-0 items-center justify-center rounded-full font-mono font-bold",
        sizeClass[size] ?? sizeClass.sm,
        node.props?.className
      )}
      style={{
        background: `oklch(0.3 0.1 ${hue})`,
        color: `oklch(0.85 0.1 ${hue})`,
      }}
    >
      {initials}
    </div>
  )
}

function renderBadge(node: UINode, keyHint: string) {
  const label = node.props?.label || ""
  const variant = node.props?.variant ?? "default"

  const variantClass: Record<string, string> = {
    default: "bg-primary/20 text-primary",
    success: "bg-[var(--status-online)]/20 text-[var(--status-online)]",
    warning: "bg-[var(--status-warning)]/20 text-[var(--status-warning)]",
    error: "bg-[var(--status-error)]/20 text-[var(--status-error)]",
    info: "bg-muted/30 text-muted-foreground",
  }

  return (
    <span
      key={keyHint}
      className={cn(
        "inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-medium",
        variantClass[variant] ?? variantClass.default,
        node.props?.className
      )}
    >
      {label}
    </span>
  )
}

function renderTabs(node: UINode, ctx: RenderCtx, keyHint: string) {
  const tabs = node.children?.filter((c) => c.type === "tab") ?? []
  const contents = node.children?.filter((c) => c.type === "tab_content") ?? []
  const active = ctx.activeTab || tabs[0]?.props?.id || ""

  if (!ctx.activeTab && tabs[0]?.props?.id) {
    ctx.setActiveTab(tabs[0].props.id)
  }

  return (
    <div key={keyHint} className={cn("flex flex-col", node.props?.className)}>
      <div className="flex gap-1 border-b border-[var(--window-border)]">
        {tabs.map((tab, i) => {
          const isActive = tab.props?.id === active
          return (
            <button
              key={`${keyHint}-tab-${i}`}
              className={cn(
                "rounded-t-md px-3 py-1.5 text-xs font-medium transition-colors",
                isActive
                  ? "border-b-2 border-primary text-primary"
                  : "text-muted-foreground hover:text-foreground"
              )}
              onClick={() => {
                ctx.setActiveTab(tab.props?.id ?? "")
                if (tab.action) ctx.onAction(tab.action)
              }}
            >
              {tab.props?.label}
            </button>
          )
        })}
      </div>
      <div className="flex-1 overflow-auto">
        {contents
          .filter((c) => c.props?.id === active)
          .map((c, i) => ctx.renderNode(c, `${keyHint}-content-${i}`))}
      </div>
    </div>
  )
}

function renderTab(node: UINode, _ctx: RenderCtx, _keyHint: string) {
  return null
}

function renderTabContent(node: UINode, ctx: RenderCtx, keyHint: string) {
  return (
    <div key={keyHint} className={cn("p-3", node.props?.className)}>
      {node.children?.map((child, i) => ctx.renderNode(child, `${keyHint}-${i}`))}
    </div>
  )
}

function renderList(node: UINode, ctx: RenderCtx, keyHint: string) {
  return (
    <div key={keyHint} className={cn("flex flex-col", node.props?.className)}>
      {node.children?.map((child, i) => ctx.renderNode(child, `${keyHint}-${i}`))}
    </div>
  )
}

function renderListItem(node: UINode, ctx: RenderCtx, keyHint: string) {
  const label = node.props?.label || ""
  const subtitle = node.props?.subtitle
  const leading = node.props?.leading
  const trailing = node.props?.trailing

  return (
    <button
      key={keyHint}
      className={cn(
        "flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left transition-colors hover:bg-secondary",
        node.action && "cursor-pointer",
        node.props?.className
      )}
      onClick={() => node.action && ctx.onAction(node.action)}
    >
      {leading && (
        <span className="flex h-5 w-5 flex-shrink-0 items-center justify-center text-muted-foreground">
          {leading}
        </span>
      )}
      <div className="flex min-w-0 flex-1 flex-col">
        <span className="truncate text-sm font-medium text-foreground">{label}</span>
        {subtitle && (
          <span className="truncate text-xs text-muted-foreground">{subtitle}</span>
        )}
      </div>
      {trailing && (
        <span className="flex flex-shrink-0 items-center text-xs text-muted-foreground">
          {trailing}
        </span>
      )}
    </button>
  )
}

function renderImage(node: UINode, keyHint: string) {
  const src = node.props?.src ?? ""
  const alt = node.props?.alt ?? ""
  const width = node.props?.width
  const height = node.props?.height
  return (
    <img
      key={keyHint}
      src={src}
      alt={alt}
      width={width ? parseInt(width, 10) : undefined}
      height={height ? parseInt(height, 10) : undefined}
      className={cn("rounded-md object-cover", node.props?.className)}
    />
  )
}

function renderIcon(node: UINode, keyHint: string) {
  const name = node.props?.name ?? ""
  const size = node.props?.size ?? "4"
  return (
    <span
      key={keyHint}
      className={cn(`h-${size} w-${size} text-muted-foreground`, node.props?.className)}
      title={name}
    >
      ●
    </span>
  )
}

function renderUnknown(node: UINode, keyHint: string) {
  return (
    <div
      key={keyHint}
      className="rounded border border-dashed border-border/50 p-2 text-xs italic text-muted-foreground"
    >
      [unknown: {node.type || "none"}]
    </div>
  )
}
