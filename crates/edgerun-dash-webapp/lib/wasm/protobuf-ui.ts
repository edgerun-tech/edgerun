export interface UINode {
  type: string
  props: Record<string, string>
  children: UINode[]
  action: string | null
}

// Supported UINode types and their expected props:
//
// Layout:
//   column       - flex-col container. props: gap?, padding?, className?
//   row          - flex-row container. props: gap?, padding?, align?, justify?, className?
//   grid         - CSS grid container. props: cols?, gap?, padding?, className?
//   scroll       - overflow-auto container. props: className?
//   spacer       - vertical spacing. props: height? (default "8")
//   divider      - horizontal rule. props: className?
//
// Content:
//   text         - inline text. props: value, variant? (xs|sm|base|lg|mono), color? (fg|muted|primary|error|warning), className?
//   heading      - h1-h6 heading. props: value, level? (1-6), className?
//   paragraph    - block text with spacing. props: value, className?
//
// Interactive:
//   button       - clickable button. props: label, variant? (primary|secondary|ghost|destructive|outline), size? (sm|md|lg), icon?, action, className?
//   input        - text input. props: placeholder, label?, type? (text|password|email|number), multiline?, action, className?
//   link         - clickable text. props: label, href?, action, className?
//   list_item    - row with leading/trailing. props: label, subtitle?, leading?, trailing?, action, className?
//
// Components:
//   card         - bordered container. props: title?, padding?, className?
//   avatar       - circular initials. props: name, size? (xs|sm|md|lg), className?
//   badge        - status pill. props: label, variant? (default|success|warning|error|info), className?
//   tabs         - tab bar. props: active?, className?. Children: tab + tab_content
//   tab          - tab button. props: label, id?, action, className?
//   tab_content  - tab panel. props: id?, className?
//   list         - vertical list. props: className?
//   image        - image display. props: src?, alt?, width?, height?, className?
//   icon         - icon placeholder. props: name?, size?, className?

export function decodeUINode(bytes: Uint8Array): UINode {
  return decodeMessage(bytes, 0)
}

function decodeMessage(bytes: Uint8Array, end: number): UINode {
  const node: UINode = {
    type: '',
    props: {},
    children: [],
    action: null,
  }

  let pos = 0
  const limit = end > 0 ? end : bytes.length

  while (pos < limit) {
    const { tag, wireType, newPos } = decodeTag(bytes, pos)
    pos = newPos

    switch (tag) {
      case 1:
        node.type = decodeString(bytes, pos)
        pos = skipLengthDelimited(bytes, pos)
        break

      case 2: {
        const propBytes = extractLengthDelimited(bytes, pos)
        pos = skipLengthDelimited(bytes, pos)
        decodeMapEntry(propBytes, node.props)
        break
      }

      case 3: {
        const childBytes = extractLengthDelimited(bytes, pos)
        pos = skipLengthDelimited(bytes, pos)
        node.children.push(decodeMessage(childBytes, childBytes.length))
        break
      }

      case 4:
        node.action = decodeString(bytes, pos)
        pos = skipLengthDelimited(bytes, pos)
        break

      default:
        pos = skipField(bytes, pos, wireType)
        break
    }
  }

  return node
}

function decodeTag(bytes: Uint8Array, pos: number) {
  const { value, newPos } = decodeVarint(bytes, pos)
  const wireType = value & 0x07
  const tag = value >>> 3
  return { tag, wireType, newPos }
}

function decodeVarint(bytes: Uint8Array, pos: number) {
  let result = 0
  let shift = 0
  let b: number
  let newPos = pos

  do {
    b = bytes[newPos++]
    result |= (b & 0x7F) << shift
    shift += 7
  } while (b & 0x80)

  return { value: result, newPos }
}

function decodeString(bytes: Uint8Array, pos: number): string {
  const { length, newPos } = decodeLength(bytes, pos)
  const end = newPos + length
  const decoder = new TextDecoder()
  return decoder.decode(bytes.slice(newPos, end))
}

function extractLengthDelimited(bytes: Uint8Array, pos: number): Uint8Array {
  const { length, newPos } = decodeLength(bytes, pos)
  return bytes.slice(newPos, newPos + length)
}

function skipLengthDelimited(bytes: Uint8Array, pos: number): number {
  const { length, newPos } = decodeLength(bytes, pos)
  return newPos + length
}

function decodeLength(bytes: Uint8Array, pos: number) {
  const { value: length, newPos } = decodeVarint(bytes, pos)
  return { length, newPos }
}

function skipField(bytes: Uint8Array, pos: number, wireType: number): number {
  switch (wireType) {
    case 0:
      return decodeVarint(bytes, pos).newPos
    case 1:
      return pos + 8
    case 2:
      return skipLengthDelimited(bytes, pos)
    case 5:
      return pos + 4
    default:
      return pos
  }
}

function decodeMapEntry(bytes: Uint8Array, map: Record<string, string>) {
  let pos = 0
  const limit = bytes.length
  let key = ""
  let value = ""
  while (pos < limit) {
    const { tag, wireType, newPos } = decodeTag(bytes, pos)
    pos = newPos
    if (tag === 1 && wireType === 2) {
      key = decodeString(bytes, pos)
      pos = skipLengthDelimited(bytes, pos)
    } else if (tag === 2 && wireType === 2) {
      value = decodeString(bytes, pos)
      pos = skipLengthDelimited(bytes, pos)
    } else {
      pos = skipField(bytes, pos, wireType)
    }
  }
  if (key) {
    map[key] = value
  }
}
