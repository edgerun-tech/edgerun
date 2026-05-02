import type { UINode } from "./protobuf-ui"

// ── Node builders ───────────────────────────────────────

function sanitize(props?: Props): Record<string, string> {
  if (!props) return {}
  const result: Record<string, string> = {}
  for (const [k, v] of Object.entries(props)) {
    if (v !== undefined) result[k] = v
  }
  return result
}

export function col(children: UINode[], props?: Props): UINode {
  return { type: "column", props: sanitize(props), children, action: null }
}

export function row(children: UINode[], props?: Props): UINode {
  return { type: "row", props: sanitize(props), children, action: null }
}

export function grid(children: UINode[], props?: Props): UINode {
  return { type: "grid", props: sanitize(props), children, action: null }
}

export function scroll(children: UINode[], props?: Props): UINode {
  return { type: "scroll", props: sanitize(props), children, action: null }
}

export function spacer(height = 8): UINode {
  return { type: "spacer", props: { height: String(height) }, children: [], action: null }
}

export function divider(): UINode {
  return { type: "divider", props: {}, children: [], action: null }
}

export function text(value: string, props?: TextProps): UINode {
  return { type: "text", props: { value, ...(props ?? {}) }, children: [], action: null }
}

export function heading(value: string, level = 2): UINode {
  return { type: "heading", props: { value, level: String(level) }, children: [], action: null }
}

export function paragraph(value: string): UINode {
  return { type: "paragraph", props: { value }, children: [], action: null }
}

export function button(label: string, props?: ButtonProps): UINode {
  return {
    type: "button",
    props: { label, ...(props ?? {}) },
    children: [],
    action: props?.action ?? null,
  }
}

export function input(props?: InputProps): UINode {
  return { type: "input", props: sanitize(props), children: [], action: props?.action ?? null }
}

export function link(label: string, action?: string): UINode {
  return { type: "link", props: { label }, children: [], action: action ?? null }
}

export function card(children: UINode[], props?: CardProps): UINode {
  return {
    type: "card",
    props: sanitize(props),
    children,
    action: null,
  }
}

export function avatar(name: string, size = "sm"): UINode {
  return { type: "avatar", props: { name, size }, children: [], action: null }
}

export function badge(label: string, variant = "default"): UINode {
  return { type: "badge", props: { label, variant }, children: [], action: null }
}

export function tabs(
  items: { id: string; label: string; content: UINode[]; action?: string }[],
  activeId?: string
): UINode {
  const children: UINode[] = []
  for (const item of items) {
    children.push({
      type: "tab",
      props: { id: item.id, label: item.label },
      children: [],
      action: item.action ?? null,
    })
    children.push({
      type: "tab_content",
      props: { id: item.id },
      children: item.content,
      action: null,
    })
  }
  return { type: "tabs", props: activeId ? { active: activeId } : {}, children, action: null }
}

export function list(items: UINode[]): UINode {
  return { type: "list", props: {}, children: items, action: null }
}

export function listItem(label: string, props?: ListItemProps): UINode {
  return {
    type: "list_item",
    props: { label, ...(props ?? {}) },
    children: [],
    action: props?.action ?? null,
  }
}

export function image(src: string, props?: Props): UINode {
  return { type: "image", props: { src, ...(props ?? {}) }, children: [], action: null }
}

// ── Prop types ──────────────────────────────────────────

export interface Props {
  [key: string]: string | undefined
}

export interface TextProps extends Props {
  variant?: "xs" | "sm" | "base" | "lg" | "mono"
  color?: "foreground" | "muted" | "primary" | "error" | "warning" | "online"
}

export interface ButtonProps extends Props {
  variant?: "primary" | "secondary" | "ghost" | "destructive" | "outline" | "success"
  size?: "xs" | "sm" | "md" | "lg"
  action?: string
}

export interface InputProps extends Props {
  placeholder?: string
  label?: string
  type?: "text" | "password" | "email" | "number"
  multiline?: "true" | "false"
  action?: string
}

export interface CardProps extends Props {
  title?: string
  padding?: string
}

export interface ListItemProps extends Props {
  subtitle?: string
  leading?: string
  trailing?: string
  action?: string
}

// ── Helpers ─────────────────────────────────────────────

export function app(children: UINode[], props?: Props): UINode {
  return col(children, { padding: "4", ...props })
}

export function section(title: string, children: UINode[]): UINode {
  return col([heading(title, 5), spacer(4), ...children], { gap: "2" })
}

export function cardSection(title: string, children: UINode[]): UINode {
  return card(children, { title, padding: "3" })
}

// ── Binary encoding ─────────────────────────────────────

export function encodeUINode(node: UINode): Uint8Array {
  const parts: Uint8Array[] = []

  if (node.type) {
    parts.push(encodeString(1, node.type))
  }

  const propsEntries = Object.entries(node.props)
  for (const [key, value] of propsEntries) {
    const entryBytes = encodeMapEntry(key, value)
    parts.push(encodeLengthDelimited(2, entryBytes))
  }

  for (const child of node.children) {
    const childBytes = encodeUINode(child)
    parts.push(encodeLengthDelimited(3, childBytes))
  }

  if (node.action) {
    parts.push(encodeString(4, node.action))
  }

  const total = parts.reduce((sum, p) => sum + p.length, 0)
  const result = new Uint8Array(total)
  let offset = 0
  for (const part of parts) {
    result.set(part, offset)
    offset += part.length
  }
  return result
}

function encodeString(field: number, value: string): Uint8Array {
  const tag = encodeVarint((field << 3) | 2)
  const bytes = new TextEncoder().encode(value)
  const len = encodeVarint(bytes.length)
  const result = new Uint8Array(tag.length + len.length + bytes.length)
  result.set(tag, 0)
  result.set(len, tag.length)
  result.set(bytes, tag.length + len.length)
  return result
}

function encodeLengthDelimited(field: number, value: Uint8Array): Uint8Array {
  const tag = encodeVarint((field << 3) | 2)
  const len = encodeVarint(value.length)
  const result = new Uint8Array(tag.length + len.length + value.length)
  result.set(tag, 0)
  result.set(len, tag.length)
  result.set(value, tag.length + len.length)
  return result
}

function encodeMapEntry(key: string, value: string): Uint8Array {
  const keyBytes = encodeString(1, key)
  const valBytes = encodeString(2, value)
  const result = new Uint8Array(keyBytes.length + valBytes.length)
  result.set(keyBytes, 0)
  result.set(valBytes, keyBytes.length)
  return result
}

function encodeVarint(value: number): Uint8Array {
  const parts: number[] = []
  while (value > 0x7f) {
    parts.push((value & 0x7f) | 0x80)
    value >>>= 7
  }
  parts.push(value)
  return new Uint8Array(parts)
}

// ── Output wrapper ──────────────────────────────────────

export function wrapUIOutput(uiBytes: Uint8Array): Uint8Array {
  const contentType = "edgerun-ui"
  const ctBytes = new TextEncoder().encode(contentType)

  const header = new Uint8Array(2 + 4 + ctBytes.length + 4)
  const view = new DataView(header.buffer)
  view.setUint16(0, 0, true)
  view.setUint32(2, ctBytes.length, true)
  for (let i = 0; i < ctBytes.length; i++) {
    header[6 + i] = ctBytes[i]
  }
  view.setUint32(6 + ctBytes.length, uiBytes.length, true)

  const result = new Uint8Array(header.length + uiBytes.length)
  result.set(header, 0)
  result.set(uiBytes, header.length)
  return result
}
