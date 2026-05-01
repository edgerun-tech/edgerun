import { useCallback, useState, useMemo } from "react"
import { cn } from "@/lib/utils"
import type { UIComponent, SafeValue } from "./ui-component"

interface ComponentRendererProps {
  tree: UIComponent
  onAction: (action: string) => void
  className?: string
}

export function ComponentRenderer({ tree, onAction, className }: ComponentRendererProps) {
  const [activeTab, setActiveTab] = useState<string>(
    tree.props?.defaultTab as string ?? ""
  )

  const render = useCallback(
    (node: UIComponent, keyHint: string): React.ReactNode => {
      const ctx: RenderCtx = { onAction, render, activeTab, setActiveTab }
      return renderNode(node, ctx, keyHint)
    },
    [onAction, activeTab]
  )

  return (
    <div className={cn("h-full w-full", className)}>
      {render(tree, "root")}
    </div>
  )
}

interface RenderCtx {
  onAction: (action: string) => void
  render: (node: UIComponent, keyHint: string) => React.ReactNode
  activeTab: string
  setActiveTab: (id: string) => void
}

function renderNode(node: UIComponent, ctx: RenderCtx, keyHint: string): React.ReactNode {
  switch (node.component) {
    case "div":
    case "span":
    case "section":
    case "header":
    case "footer":
    case "main":
    case "p":
    case "h1":
    case "h2":
    case "h3":
      return renderNative(node, ctx, keyHint)
    case "Card":
      return renderCard(node, ctx, keyHint)
    case "Button":
      return renderButton(node, ctx, keyHint)
    case "Input":
      return renderInput(node, ctx, keyHint)
    case "Badge":
      return renderBadge(node, keyHint)
    case "Avatar":
      return renderAvatar(node, keyHint)
    case "Tabs":
      return renderTabs(node, ctx, keyHint)
    case "List":
      return renderList(node, ctx, keyHint)
    case "ListItem":
      return renderListItem(node, ctx, keyHint)
    case "Divider":
      return renderDivider(keyHint)
    case "Spacer":
      return renderSpacer(node, keyHint)
    case "Grid":
      return renderGrid(node, ctx, keyHint)
    case "Scroll":
      return renderScroll(node, ctx, keyHint)
    case "Image":
      return renderImage(node, keyHint)
    case "Text":
      return renderText(node, keyHint)
    case "Heading":
      return renderHeading(node, keyHint)
    default:
      return null
  }
}

// ── HTML primitives ─────────────────────────────────────

function renderNative(node: UIComponent, ctx: RenderCtx, keyHint: string): React.ReactNode {
  const Tag = node.component
  const props = buildProps(node.props)
  const handlers = buildHandlers(node.on, ctx.onAction)
  const children = node.children?.map((c, i) => ctx.render(c, `${keyHint}-${i}`))

  return (
    <Tag key={keyHint} {...props} {...handlers}>
      {children}
    </Tag>
  )
}

// ── Card ────────────────────────────────────────────────

function renderCard(node: UIComponent, ctx: RenderCtx, keyHint: string): React.ReactNode {
  const variant = node.props?.variant as string ?? "default"
  const title = node.props?.title as string | undefined
  const children = node.children?.map((c, i) => ctx.render(c, `${keyHint}-${i}`))

  return (
    <div
      key={keyHint}
      className={cn(
        "rounded-lg border bg-[var(--window-bg)]",
        variant === "default" && "border-[var(--window-border)]",
        variant === "muted" && "border-border bg-secondary/50",
        variant === "glass" && "border-border/50 bg-[var(--window-bg)]/80 backdrop-blur-md",
        node.props?.className as string
      )}
    >
      {title && (
        <div className="border-b border-[var(--window-border)] px-3 py-2">
          <span className="text-xs font-semibold text-foreground">{title}</span>
        </div>
      )}
      <div className="p-3">{children}</div>
    </div>
  )
}

// ── Button ──────────────────────────────────────────────

function renderButton(node: UIComponent, ctx: RenderCtx, keyHint: string): React.ReactNode {
  const label = (node.props?.label as string) ?? "button"
  const variant = node.props?.variant as string ?? "default"
  const size = node.props?.size as string ?? "default"
  const disabled = node.props?.disabled === true

  const variantClass: Record<string, string> = {
    default: "bg-primary text-primary-foreground hover:bg-primary/90",
    secondary: "bg-secondary text-foreground hover:bg-secondary/70",
    destructive: "bg-destructive text-destructive-foreground hover:bg-destructive/90",
    outline: "border border-border bg-transparent hover:bg-secondary hover:text-foreground",
    ghost: "hover:bg-secondary hover:text-foreground",
    link: "text-primary underline-offset-4 hover:underline",
  }

  const sizeClass: Record<string, string> = {
    default: "h-9 px-4 py-2 text-sm",
    sm: "h-8 rounded-md px-3 text-xs",
    lg: "h-10 rounded-md px-8",
    icon: "h-9 w-9",
  }

  const handlers = buildHandlers(node.on, ctx.onAction)

  return (
    <button
      key={keyHint}
      disabled={disabled}
      className={cn(
        "inline-flex items-center justify-center rounded-md font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50",
        variantClass[variant] ?? variantClass.default,
        sizeClass[size] ?? sizeClass.default,
        node.props?.className as string
      )}
      {...handlers}
    >
      {label}
    </button>
  )
}

// ── Input ───────────────────────────────────────────────

function renderInput(node: UIComponent, ctx: RenderCtx, keyHint: string): React.ReactNode {
  const placeholder = node.props?.placeholder as string ?? ""
  const label = node.props?.label as string | undefined
  const type = node.props?.type as string ?? "text"
  const multiline = node.props?.multiline === true

  const handlers = buildHandlers(node.on, ctx.onAction)

  const baseClass = cn(
    "flex h-9 w-full rounded-md border border-border bg-transparent px-3 py-1 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50",
    node.props?.className as string
  )

  return (
    <div key={keyHint} className="flex flex-col gap-1.5">
      {label && (
        <label className="text-xs font-medium text-foreground">{label}</label>
      )}
      {multiline ? (
        <textarea
          placeholder={placeholder}
          rows={3}
          className={cn(baseClass, "min-h-[80px] py-2")}
          {...handlers}
        />
      ) : (
        <input type={type} placeholder={placeholder} className={baseClass} {...handlers} />
      )}
    </div>
  )
}

// ── Badge ───────────────────────────────────────────────

function renderBadge(node: UIComponent, keyHint: string): React.ReactNode {
  const label = node.props?.label as string ?? ""
  const variant = node.props?.variant as string ?? "default"

  const variantClass: Record<string, string> = {
    default: "bg-primary/20 text-primary",
    secondary: "bg-secondary text-secondary-foreground",
    destructive: "bg-destructive/20 text-destructive",
    success: "bg-[var(--status-online)]/20 text-[var(--status-online)]",
    warning: "bg-[var(--status-warning)]/20 text-[var(--status-warning)]",
  }

  return (
    <span
      key={keyHint}
      className={cn(
        "inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-semibold transition-colors",
        variantClass[variant] ?? variantClass.default,
        node.props?.className as string
      )}
    >
      {label}
    </span>
  )
}

// ── Avatar ──────────────────────────────────────────────

function renderAvatar(node: UIComponent, keyHint: string): React.ReactNode {
  const name = (node.props?.name as string) ?? "?"
  const size = node.props?.size as string ?? "sm"

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
        node.props?.className as string
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

// ── Tabs ────────────────────────────────────────────────

function renderTabs(node: UIComponent, ctx: RenderCtx, keyHint: string): React.ReactNode {
  const tabs = node.children?.filter((c) => c.component === "Tab") ?? []
  const panels = node.children?.filter((c) => c.component === "TabPanel") ?? []
  const active = ctx.activeTab || tabs[0]?.props?.id as string || ""

  if (!ctx.activeTab && tabs[0]?.props?.id) {
    ctx.setActiveTab(tabs[0].props.id as string)
  }

  return (
    <div key={keyHint} className="flex flex-col">
      <div className="flex gap-1 border-b border-[var(--window-border)]">
        {tabs.map((tab, i) => {
          const id = tab.props?.id as string
          const isActive = id === active
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
                if (id) ctx.setActiveTab(id)
                if (tab.on?.click) ctx.onAction(tab.on.click)
              }}
            >
              {tab.props?.label}
            </button>
          )
        })}
      </div>
      <div className="flex-1 overflow-auto">
        {panels
          .filter((p) => (p.props?.id as string) === active)
          .map((p, i) => ctx.render(p, `${keyHint}-panel-${i}`))}
      </div>
    </div>
  )
}

// ── List ────────────────────────────────────────────────

function renderList(node: UIComponent, ctx: RenderCtx, keyHint: string): React.ReactNode {
  return (
    <div key={keyHint} className="flex flex-col">
      {node.children?.map((c, i) => ctx.render(c, `${keyHint}-${i}`))}
    </div>
  )
}

// ── ListItem ────────────────────────────────────────────

function renderListItem(node: UIComponent, ctx: RenderCtx, keyHint: string): React.ReactNode {
  const label = (node.props?.label as string) ?? ""
  const subtitle = node.props?.subtitle as string | undefined
  const leading = node.props?.leading as string | undefined
  const trailing = node.props?.trailing as string | undefined
  const handlers = buildHandlers(node.on, ctx.onAction)

  return (
    <button
      key={keyHint}
      className={cn(
        "flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left transition-colors hover:bg-secondary",
        node.on && "cursor-pointer",
        node.props?.className as string
      )}
      {...handlers}
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

// ── Divider ─────────────────────────────────────────────

function renderDivider(keyHint: string): React.ReactNode {
  return <div key={keyHint} className="h-px w-full bg-[var(--window-border)]" />
}

// ── Spacer ──────────────────────────────────────────────

function renderSpacer(node: UIComponent, keyHint: string): React.ReactNode {
  const height = (node.props?.height as number) ?? 8
  return <div key={keyHint} style={{ height: `${height}px` }} />
}

// ── Grid ────────────────────────────────────────────────

function renderGrid(node: UIComponent, ctx: RenderCtx, keyHint: string): React.ReactNode {
  const cols = (node.props?.cols as number) ?? 2
  const gap = node.props?.gap as number ?? 3
  return (
    <div
      key={keyHint}
      className={cn(`grid grid-cols-${cols} gap-${gap}`, node.props?.className as string)}
    >
      {node.children?.map((c, i) => ctx.render(c, `${keyHint}-${i}`))}
    </div>
  )
}

// ── Scroll ──────────────────────────────────────────────

function renderScroll(node: UIComponent, ctx: RenderCtx, keyHint: string): React.ReactNode {
  return (
    <div
      key={keyHint}
      className={cn("flex-1 overflow-auto", node.props?.className as string)}
    >
      {node.children?.map((c, i) => ctx.render(c, `${keyHint}-${i}`))}
    </div>
  )
}

// ── Image ───────────────────────────────────────────────

function renderImage(node: UIComponent, keyHint: string): React.ReactNode {
  const src = (node.props?.src as string) ?? ""
  const alt = (node.props?.alt as string) ?? ""
  return (
    <img
      key={keyHint}
      src={src}
      alt={alt}
      className={cn("rounded-md object-cover", node.props?.className as string)}
    />
  )
}

// ── Text ────────────────────────────────────────────────

function renderText(node: UIComponent, keyHint: string): React.ReactNode {
  const value = (node.props?.value as string) ?? ""
  const variant = node.props?.variant as string ?? "sm"
  const color = node.props?.color as string ?? "foreground"
  const isMono = node.props?.mono === true

  const variantClass: Record<string, string> = {
    xs: "text-[10px]",
    sm: "text-xs",
    base: "text-sm",
    lg: "text-base",
  }

  const colorClass: Record<string, string> = {
    foreground: "text-foreground",
    muted: "text-muted-foreground",
    primary: "text-primary",
    error: "text-[var(--status-error)]",
    warning: "text-[var(--status-warning)]",
    online: "text-[var(--status-online)]",
  }

  return (
    <span
      key={keyHint}
      className={cn(
        variantClass[variant] ?? "text-sm",
        colorClass[color] ?? "text-foreground",
        isMono && "font-mono",
        node.props?.className as string
      )}
    >
      {value}
    </span>
  )
}

// ── Heading ─────────────────────────────────────────────

function renderHeading(node: UIComponent, keyHint: string): React.ReactNode {
  const value = (node.props?.value as string) ?? ""
  const level = Math.min(Math.max((node.props?.level as number) ?? 2, 1), 6) as 1 | 2 | 3 | 4 | 5 | 6
  const tags = ["h1", "h2", "h3", "h4", "h5", "h6"] as const
  const Tag = tags[level - 1]

  const sizeClass: Record<number, string> = {
    1: "text-xl font-bold",
    2: "text-lg font-semibold",
    3: "text-base font-semibold",
    4: "text-sm font-semibold",
    5: "text-xs font-semibold uppercase tracking-wider",
    6: "text-[10px] font-semibold uppercase tracking-widest",
  }

  return (
    <Tag
      key={keyHint}
      className={cn(
        "text-foreground",
        sizeClass[level] ?? "text-lg font-semibold",
        node.props?.className as string
      )}
    >
      {value}
    </Tag>
  )
}

// ── Helpers ─────────────────────────────────────────────

function buildProps(props?: Record<string, SafeValue>): Record<string, string | number | boolean> {
  if (!props) return {}
  const out: Record<string, string | number | boolean> = {}
  for (const [key, value] of Object.entries(props)) {
    if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
      out[key] = value
    }
  }
  return out
}

function buildHandlers(
  on?: Record<string, string>,
  onAction?: (action: string) => void
): Record<string, () => void> {
  if (!on || !onAction) return {}
  const handlers: Record<string, () => void> = {}
  for (const [event, action] of Object.entries(on)) {
    const reactEvent = `on${event.charAt(0).toUpperCase()}${event.slice(1)}`
    handlers[reactEvent] = () => onAction(action)
  }
  return handlers
}
