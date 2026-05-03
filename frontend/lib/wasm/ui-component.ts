// Safe JSON-based component tree protocol for WASM apps.
// WASM sends a declarative tree of component descriptions.
// Host renders with actual React components, injects event handlers.
// No JavaScript execution in the UI tree.

export interface UIComponent {
  component: string
  props?: Record<string, SafeValue>
  children?: UIComponent[]
  on?: Record<string, string>
}

export type SafeValue = string | number | boolean | null | SafeValue[]

export function isValidComponentTree(obj: unknown): obj is UIComponent {
  if (typeof obj !== "object" || obj === null) return false
  const tree = obj as Record<string, unknown>
  if (typeof tree.component !== "string") return false
  if (tree.props !== undefined) {
    if (typeof tree.props !== "object" || Array.isArray(tree.props)) return false
    for (const [, value] of Object.entries(tree.props as Record<string, unknown>)) {
      if (!isSafeValue(value)) return false
    }
  }
  if (tree.children !== undefined) {
    if (!Array.isArray(tree.children)) return false
    for (const child of tree.children) {
      if (!isValidComponentTree(child)) return false
    }
  }
  if (tree.on !== undefined) {
    if (typeof tree.on !== "object" || Array.isArray(tree.on)) return false
    for (const [, value] of Object.entries(tree.on as Record<string, unknown>)) {
      if (typeof value !== "string") return false
    }
  }
  return true
}

function isSafeValue(value: unknown): boolean {
  if (value === null || typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
    return true
  }
  if (Array.isArray(value)) {
    return value.every(isSafeValue)
  }
  return false
}

// Allowed component registry — maps component names to implementation metadata
export interface ComponentDef {
  tag: "native" | "custom"
  allowedProps: string[]
  defaultTag?: string
}

export const COMPONENT_REGISTRY: Record<string, ComponentDef> = {
  // HTML primitives
  div: { tag: "native", allowedProps: ["className", "style", "id"], defaultTag: "div" },
  span: { tag: "native", allowedProps: ["className", "style", "id"], defaultTag: "span" },
  section: { tag: "native", allowedProps: ["className", "id"], defaultTag: "section" },
  header: { tag: "native", allowedProps: ["className", "id"], defaultTag: "header" },
  footer: { tag: "native", allowedProps: ["className", "id"], defaultTag: "footer" },
  main: { tag: "native", allowedProps: ["className", "id"], defaultTag: "main" },
  p: { tag: "native", allowedProps: ["className", "style"], defaultTag: "p" },
  h1: { tag: "native", allowedProps: ["className", "style"], defaultTag: "h1" },
  h2: { tag: "native", allowedProps: ["className", "style"], defaultTag: "h2" },
  h3: { tag: "native", allowedProps: ["className", "style"], defaultTag: "h3" },

  // Edgerun UI components
  Card: { tag: "custom", allowedProps: ["variant", "title", "className", "header", "footer"] },
  Button: { tag: "custom", allowedProps: ["label", "variant", "size", "className", "disabled", "icon"] },
  Input: { tag: "custom", allowedProps: ["placeholder", "label", "type", "value", "className", "multiline"] },
  Badge: { tag: "custom", allowedProps: ["label", "variant", "className"] },
  Avatar: { tag: "custom", allowedProps: ["name", "size", "className"] },
  Tabs: { tag: "custom", allowedProps: ["defaultTab", "className"] },
  List: { tag: "custom", allowedProps: ["className"] },
  ListItem: { tag: "custom", allowedProps: ["label", "subtitle", "leading", "trailing", "className"] },
  Divider: { tag: "custom", allowedProps: ["className"] },
  Spacer: { tag: "custom", allowedProps: ["height"] },
  Grid: { tag: "custom", allowedProps: ["cols", "gap", "className"] },
  Scroll: { tag: "custom", allowedProps: ["className"] },
  Image: { tag: "custom", allowedProps: ["src", "alt", "width", "height", "className"] },
  Text: { tag: "custom", allowedProps: ["value", "variant", "color", "className", "mono"] },
  Heading: { tag: "custom", allowedProps: ["value", "level", "className"] },
}

export const ALLOWED_COMPONENTS = Object.keys(COMPONENT_REGISTRY)

export function sanitizeTree(tree: unknown): UIComponent | null {
  if (!isValidComponentTree(tree)) return null
  return validateComponentNames(tree as UIComponent)
}

function validateComponentNames(tree: UIComponent): UIComponent | null {
  if (!ALLOWED_COMPONENTS.includes(tree.component)) {
    return null
  }
  const def = COMPONENT_REGISTRY[tree.component]

  const safeProps: Record<string, SafeValue> = {}
  if (tree.props) {
    for (const [key, value] of Object.entries(tree.props)) {
      if (def.allowedProps.includes(key)) {
        safeProps[key] = value
      }
    }
  }

  const safeChildren: UIComponent[] = []
  if (tree.children) {
    for (const child of tree.children) {
      const validated = validateComponentNames(child)
      if (validated) safeChildren.push(validated)
    }
  }

  const safeOn: Record<string, string> = {}
  if (tree.on) {
    for (const [event, action] of Object.entries(tree.on)) {
      if (typeof event === "string" && typeof action === "string") {
        safeOn[event] = action
      }
    }
  }

  return {
    component: tree.component,
    props: Object.keys(safeProps).length > 0 ? safeProps : undefined,
    children: safeChildren.length > 0 ? safeChildren : undefined,
    on: Object.keys(safeOn).length > 0 ? safeOn : undefined,
  }
}

// Parse JSON UI tree from WASM output
export function parseUITree(text: string): UIComponent | null {
  try {
    const parsed = JSON.parse(text)
    return sanitizeTree(parsed)
  } catch {
    return null
  }
}

// Encode component tree to JSON for WASM consumption
export function encodeUITree(tree: UIComponent): string {
  return JSON.stringify(tree)
}
