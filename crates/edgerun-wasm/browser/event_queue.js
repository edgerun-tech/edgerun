class EventQueue {
  constructor() {
    this.queue = [];
  }

  push(eventBytes) {
    this.queue.push(new Uint8Array(eventBytes));
  }

  pop() {
    return this.queue.length > 0 ? this.queue.shift() : null;
  }

  hasEvents() {
    return this.queue.length > 0;
  }
}

const EventType = {
  Network: 1,
  Disk: 2,
  Timer: 3,
};

const NetworkSubtype = {
  Connected: 1,
  Disconnected: 2,
  Received: 3,
  Error: 4,
};

const DiskSubtype = {
  ReadDone: 1,
  WriteDone: 2,
  Error: 3,
};

function encodeNetworkEvent(sockId, subtype, payload) {
  if (subtype === NetworkSubtype.Received) {
    const hasPayload = payload && (payload.byteLength || payload.length) > 0;
    const payloadBytes = hasPayload
      ? payload instanceof ArrayBuffer
        ? new Uint8Array(payload)
        : new Uint8Array(payload)
      : new Uint8Array(0);

    const size = 10 + payloadBytes.byteLength;
    const buf = new Uint8Array(size);
    const view = new DataView(buf.buffer);

    buf[0] = EventType.Network;
    view.setUint32(1, sockId, true);
    buf[5] = subtype;
    view.setUint32(6, payloadBytes.byteLength, true);
    if (hasPayload) {
      buf.set(payloadBytes, 10);
    }
    return buf;
  } else {
    const buf = new Uint8Array(6);
    const view = new DataView(buf.buffer);
    buf[0] = EventType.Network;
    view.setUint32(1, sockId, true);
    buf[5] = subtype;
    return buf;
  }
}

function encodeDiskEvent(opId, subtype, hash, data) {
  const hasHash = hash && hash.byteLength === 32;
  const hasData = data && (data.byteLength || data.length) > 0;
  const dataBytes = hasData
    ? data instanceof ArrayBuffer
      ? new Uint8Array(data)
      : new Uint8Array(data)
    : new Uint8Array(0);

  const size = 6 + (hasHash ? 32 : 0) + dataBytes.byteLength;
  const buf = new Uint8Array(size);
  const view = new DataView(buf.buffer);

  buf[0] = EventType.Disk;
  view.setUint32(1, opId, true);
  buf[5] = subtype;
  let offset = 6;
  if (hasHash) {
    buf.set(new Uint8Array(hash), offset);
    offset += 32;
  }
  if (hasData) {
    buf.set(dataBytes, offset);
  }
  return buf;
}

function encodeTimerEvent(timerId) {
  const buf = new Uint8Array(10);
  buf[0] = EventType.Timer;
  buf[1] = 1;
  const view = new DataView(buf.buffer);
  view.setBigUint64(2, BigInt(timerId), true);
  return buf;
}

export { EventQueue, encodeNetworkEvent, encodeDiskEvent, encodeTimerEvent, EventType, NetworkSubtype, DiskSubtype };
