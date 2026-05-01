/**
 * Minimal protobuf decoder for UINode (edgerun.v0.ui.UINode).
 *
 * Wire format: prost (standard protobuf wire format v1).
 * Fields:
 *   1: type     — string (wire type 2)
 *   2: props    — map<string,string> (wire type 2, nested key-value pairs)
 *   3: children — repeated message (wire type 2)
 *   4: action   — string (wire type 2, optional)
 */

export function decodeUINode(bytes) {
  return decodeMessage(bytes, 0);
}

function decodeMessage(bytes, end) {
  const node = {
    type: '',
    props: {},
    children: [],
    action: null,
  };

  let pos = 0;
  const limit = end > 0 ? end : bytes.length;

  while (pos < limit) {
    const { tag, wireType, newPos } = decodeTag(bytes, pos);
    pos = newPos;

    switch (tag) {
      case 1: // type — string
        node.type = decodeString(bytes, pos);
        pos = skipLengthDelimited(bytes, pos);
        break;

      case 2: // props — map (key=1 string, value=2 string)
        const propBytes = extractLengthDelimited(bytes, pos);
        pos = skipLengthDelimited(bytes, pos);
        decodeMapEntry(propBytes, node.props);
        break;

      case 3: // children — repeated message
        const childBytes = extractLengthDelimited(bytes, pos);
        pos = skipLengthDelimited(bytes, pos);
        node.children.push(decodeMessage(childBytes, childBytes.length));
        break;

      case 4: // action — optional string
        node.action = decodeString(bytes, pos);
        pos = skipLengthDelimited(bytes, pos);
        break;

      default:
        pos = skipField(bytes, pos, wireType);
        break;
    }
  }

  return node;
}

function decodeTag(bytes, pos) {
  const { value, newPos } = decodeVarint(bytes, pos);
  const wireType = value & 0x07;
  const tag = value >>> 3;
  return { tag, wireType, newPos };
}

function decodeVarint(bytes, pos) {
  let result = 0;
  let shift = 0;
  let b;
  let newPos = pos;

  do {
    b = bytes[newPos++];
    result |= (b & 0x7F) << shift;
    shift += 7;
  } while (b & 0x80);

  return { value: result, newPos };
}

function decodeString(bytes, pos) {
  const { length, newPos } = decodeLength(bytes, pos);
  const end = newPos + length;
  const decoder = new TextDecoder();
  const str = decoder.decode(bytes.slice(newPos, end));
  return str;
}

function extractLengthDelimited(bytes, pos) {
  const { length, newPos } = decodeLength(bytes, pos);
  return bytes.slice(newPos, newPos + length);
}

function skipLengthDelimited(bytes, pos) {
  const { length, newPos } = decodeLength(bytes, pos);
  return newPos + length;
}

function decodeLength(bytes, pos) {
  const { value: length, newPos } = decodeVarint(bytes, pos);
  return { length, newPos };
}

function skipField(bytes, pos, wireType) {
  switch (wireType) {
    case 0: // varint
      return decodeVarint(bytes, pos).newPos;
    case 1: // fixed64
      return pos + 8;
    case 2: // length-delimited
      return skipLengthDelimited(bytes, pos);
    case 5: // fixed32
      return pos + 4;
    default:
      return pos;
  }
}

function decodeMapEntry(bytes, map) {
  let pos = 0;
  const limit = bytes.length;

  while (pos < limit) {
    const { tag, wireType, newPos } = decodeTag(bytes, pos);
    pos = newPos;

    if (tag === 1 && wireType === 2) {
      const key = decodeString(bytes, pos);
      pos = skipLengthDelimited(bytes, pos);
    } else if (tag === 2 && wireType === 2) {
      const value = decodeString(bytes, pos);
      pos = skipLengthDelimited(bytes, pos);
    } else {
      pos = skipField(bytes, pos, wireType);
    }
  }

  if (Object.keys(map).length === 0) {
    let key = '';
    let value = '';
    pos = 0;
    while (pos < limit) {
      const { tag, wireType: wt, newPos } = decodeTag(bytes, pos);
      pos = newPos;
      if (tag === 1 && wt === 2) {
        key = decodeString(bytes, pos);
        pos = skipLengthDelimited(bytes, pos);
      } else if (tag === 2 && wt === 2) {
        value = decodeString(bytes, pos);
        pos = skipLengthDelimited(bytes, pos);
      } else {
        pos = skipField(bytes, pos, wt);
      }
    }
    map[key] = value;
  }
}
