const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

function encodeVarint(value) {
  let n = Number(value || 0);
  const out = [];
  while (n >= 0x80) {
    out.push((n & 0x7f) | 0x80);
    n = Math.floor(n / 128);
  }
  out.push(n & 0x7f);
  return out;
}

function decodeVarint(bytes, offset) {
  let value = 0;
  let shift = 0;
  let cursor = offset;
  while (cursor < bytes.length) {
    const byte = bytes[cursor];
    value += (byte & 0x7f) * (2 ** shift);
    cursor += 1;
    if ((byte & 0x80) === 0) {
      return { value, offset: cursor };
    }
    shift += 7;
  }
  throw new Error("invalid protobuf varint");
}

function concatChunks(chunks) {
  const total = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
  const out = new Uint8Array(total);
  let cursor = 0;
  for (const chunk of chunks) {
    out.set(chunk, cursor);
    cursor += chunk.length;
  }
  return out;
}

function encodeField(tag, wireType, payload) {
  const key = encodeVarint((tag << 3) | wireType);
  return concatChunks([Uint8Array.from(key), payload]);
}

function encodeStringField(tag, value) {
  const bytes = textEncoder.encode(String(value || ""));
  return encodeField(tag, 2, concatChunks([Uint8Array.from(encodeVarint(bytes.length)), bytes]));
}

function encodeBytesField(tag, bytes) {
  const payload = bytes instanceof Uint8Array ? bytes : new Uint8Array();
  return encodeField(tag, 2, concatChunks([Uint8Array.from(encodeVarint(payload.length)), payload]));
}

function encodeUint64Field(tag, value) {
  return encodeField(tag, 0, Uint8Array.from(encodeVarint(value)));
}

function encodeBoolField(tag, value) {
  return encodeField(tag, 0, Uint8Array.from([value ? 1 : 0]));
}

function decodeLengthDelimited(bytes, offset) {
  const { value: length, offset: nextOffset } = decodeVarint(bytes, offset);
  const end = nextOffset + length;
  if (end > bytes.length) throw new Error("length-delimited field overrun");
  return { bytes: bytes.slice(nextOffset, end), offset: end };
}

function decodeLocalNodeInfoResponse(buffer) {
  const bytes = buffer instanceof Uint8Array ? buffer : new Uint8Array(buffer || 0);
  const out = {
    ok: false,
    error: "",
    nodeId: "",
    devicePubkeyB64url: "",
    bridgeVersion: "",
    startedUnixMs: 0,
    eventbusWsPath: ""
  };
  let cursor = 0;
  while (cursor < bytes.length) {
    const key = decodeVarint(bytes, cursor);
    cursor = key.offset;
    const tag = key.value >> 3;
    const wireType = key.value & 0x07;
    if (wireType === 2) {
      const field = decodeLengthDelimited(bytes, cursor);
      cursor = field.offset;
      const value = textDecoder.decode(field.bytes);
      if (tag === 2) out.error = value;
      if (tag === 3) out.nodeId = value;
      if (tag === 4) out.devicePubkeyB64url = value;
      if (tag === 5) out.bridgeVersion = value;
      if (tag === 7) out.eventbusWsPath = value;
      continue;
    }
    if (wireType === 0) {
      const field = decodeVarint(bytes, cursor);
      cursor = field.offset;
      if (tag === 1) out.ok = field.value !== 0;
      if (tag === 6) out.startedUnixMs = field.value;
      continue;
    }
    break;
  }
  return out;
}

function encodeLocalEventEnvelope(input = {}) {
  const chunks = [];
  chunks.push(encodeStringField(1, input.eventId || ""));
  chunks.push(encodeStringField(2, input.topic || "event.unknown"));
  chunks.push(encodeBytesField(3, input.payloadBytes || new Uint8Array()));
  chunks.push(encodeStringField(4, input.source || "browser"));
  chunks.push(encodeUint64Field(5, Number(input.tsUnixMs || Date.now())));
  return concatChunks(chunks);
}

function decodeLocalEventEnvelope(buffer) {
  const bytes = buffer instanceof Uint8Array ? buffer : new Uint8Array(buffer || 0);
  const out = {
    eventId: "",
    topic: "",
    payloadBytes: new Uint8Array(),
    source: "",
    tsUnixMs: 0
  };
  let cursor = 0;
  while (cursor < bytes.length) {
    const key = decodeVarint(bytes, cursor);
    cursor = key.offset;
    const tag = key.value >> 3;
    const wireType = key.value & 0x07;
    if (wireType === 2) {
      const field = decodeLengthDelimited(bytes, cursor);
      cursor = field.offset;
      if (tag === 3) {
        out.payloadBytes = field.bytes;
      } else {
        const value = textDecoder.decode(field.bytes);
        if (tag === 1) out.eventId = value;
        if (tag === 2) out.topic = value;
        if (tag === 4) out.source = value;
      }
      continue;
    }
    if (wireType === 0) {
      const field = decodeVarint(bytes, cursor);
      cursor = field.offset;
      if (tag === 5) out.tsUnixMs = field.value;
      continue;
    }
    break;
  }
  return out;
}

export {
  decodeLocalEventEnvelope,
  decodeLocalNodeInfoResponse,
  encodeBoolField,
  encodeLocalEventEnvelope
};
