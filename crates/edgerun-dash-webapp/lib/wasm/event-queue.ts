export enum EventType {
  Network = 1,
  Disk = 2,
  Timer = 3,
}

export enum NetworkSubtype {
  Connected = 1,
  Disconnected = 2,
  Received = 3,
  Error = 4,
}

export enum DiskSubtype {
  ReadDone = 1,
  WriteDone = 2,
  Error = 3,
}

export class EventQueue {
  private queue: Uint8Array[] = []

  push(eventBytes: Uint8Array) {
    this.queue.push(new Uint8Array(eventBytes))
  }

  pushFront(eventBytes: Uint8Array) {
    this.queue.unshift(new Uint8Array(eventBytes))
  }

  pop(): Uint8Array | null {
    return this.queue.length > 0 ? this.queue.shift()! : null
  }

  peek(): Uint8Array | null {
    return this.queue.length > 0 ? this.queue[0] : null
  }

  hasEvents(): boolean {
    return this.queue.length > 0
  }
}

export function encodeNetworkEvent(sockId: number, subtype: NetworkSubtype, payload?: Uint8Array): Uint8Array {
  if (subtype === NetworkSubtype.Received) {
    const payloadBytes = payload ?? new Uint8Array(0)
    const size = 10 + payloadBytes.byteLength
    const buf = new Uint8Array(size)
    const view = new DataView(buf.buffer)

    buf[0] = EventType.Network
    view.setUint32(1, sockId, true)
    buf[5] = subtype
    view.setUint32(6, payloadBytes.byteLength, true)
    if (payloadBytes.byteLength > 0) {
      buf.set(payloadBytes, 10)
    }
    return buf
  } else {
    const buf = new Uint8Array(6)
    const view = new DataView(buf.buffer)
    buf[0] = EventType.Network
    view.setUint32(1, sockId, true)
    buf[5] = subtype
    return buf
  }
}

export function encodeDiskEvent(opId: number, subtype: DiskSubtype, hash?: Uint8Array, data?: Uint8Array): Uint8Array {
  const hasHash = hash && hash.byteLength === 32
  const dataBytes = data ?? new Uint8Array(0)
  const size = 6 + (hasHash ? 32 : 0) + dataBytes.byteLength
  const buf = new Uint8Array(size)
  const view = new DataView(buf.buffer)

  buf[0] = EventType.Disk
  view.setUint32(1, opId, true)
  buf[5] = subtype
  let offset = 6
  if (hasHash) {
    buf.set(hash!, offset)
    offset += 32
  }
  if (dataBytes.byteLength > 0) {
    buf.set(dataBytes, offset)
  }
  return buf
}

export function encodeTimerEvent(timerId: number): Uint8Array {
  const buf = new Uint8Array(10)
  buf[0] = EventType.Timer
  buf[1] = 1
  const view = new DataView(buf.buffer)
  view.setBigUint64(2, BigInt(timerId), true)
  return buf
}
