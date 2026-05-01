import { EventQueue, encodeNetworkEvent, encodeDiskEvent, encodeTimerEvent, EventType, NetworkSubtype, DiskSubtype } from './event_queue.js';
import { DOMRenderer } from './dom_renderer.js';
import { decodeUINode } from './protobuf_ui.js';

export class BrowserAdapter {
  constructor(options = {}) {
    this.eventQueue = new EventQueue();
    this.outputBuffer = [];
    this.messages = [];
    this.verbose = options.verbose || false;
    this.onOutput = options.onOutput || null;
    this.wasmMemory = null;
    this.pendingRequests = new Map();
    this.pendingBlobs = new Map();
    this.nextSockId = 1;
    this.nextOpId = 1;
    this.nextTimerId = 1;
    this.timers = new Map();
    this.sockets = new Map();

    this.uiRenderer = null;
    this.uiContainer = options.uiContainer || null;
    this.onUIAction = options.onUIAction || null;
    this.currentUI = null;
  }

  getMemory() {
    return this.wasmMemory;
  }

  setMemory(memory) {
    this.wasmMemory = memory;
  }

  writeOutput(ptr, len) {
    const mem = new Uint8Array(this.wasmMemory.buffer);
    const data = mem.slice(ptr, ptr + len);
    this.outputBuffer.push(data);

    if (this.verbose) {
      console.log(`write_output(${len}): ${new TextDecoder().decode(data)}`);
    }

    if (this.onOutput) {
      this.onOutput(data);
    }

    this.tryRenderUI(data);
  }

  sendMessage(targetPtr, targetLen, payloadPtr, payloadLen) {
    const mem = new Uint8Array(this.wasmMemory.buffer);
    const target = new TextDecoder().decode(mem.slice(targetPtr, targetPtr + targetLen));
    const payload = mem.slice(payloadPtr, payloadPtr + payloadLen);
    this.messages.push({ target, payload });
    if (this.verbose) {
      console.log(`send_message(target=${target}, payload_size=${payloadLen})`);
    }
    return 0;
  }

  readBlob(hashPtr, hashLen, dstPtr) {
    const mem = new Uint8Array(this.wasmMemory.buffer);
    const hashHex = Array.from(mem.slice(hashPtr, hashPtr + hashLen))
      .map(b => b.toString(16).padStart(2, '0'))
      .join('');

    if (this.pendingBlobs.has(hashHex)) {
      const data = this.pendingBlobs.get(hashHex);
      if (dstPtr >= 0 && data.length <= mem.length - dstPtr) {
        mem.set(data, dstPtr);
        return data.length;
      }
    }
    return -1;
  }

  writeBlob(ptr, len, hashOutPtr) {
    const mem = new Uint8Array(this.wasmMemory.buffer);
    const data = mem.slice(ptr, ptr + len);
    const hash = new Uint8Array(32);
    hash.fill(0xab);
    if (hashOutPtr >= 0 && hashOutPtr + 32 <= mem.length) {
      mem.set(hash, hashOutPtr);
    }
    this.pendingBlobs.set(Array.from(hash).map(b => b.toString(16).padStart(2, '0')).join(''), data);
    return 0;
  }

  pollEvent(bufPtr, bufLen) {
    const event = this.eventQueue.pop();
    if (!event) {
      return -1;
    }
    if (event.length > bufLen) {
      this.eventQueue.queue.unshift(event);
      return -2;
    }
    const mem = new Uint8Array(this.wasmMemory.buffer);
    mem.set(event, bufPtr);
    if (this.verbose) {
      console.log(`poll_event: type=${event[0]}, subtype=${event.length > 1 ? event[1] : 0}`);
    }
    return event.length;
  }

  getImportObject() {
    const adapter = this;
    return {
      env: {
        write_output(ptr, len) {
          adapter.writeOutput(ptr, len);
        },
        send_message(targetPtr, targetLen, payloadPtr, payloadLen) {
          return adapter.sendMessage(targetPtr, targetLen, payloadPtr, payloadLen);
        },
        read_blob(hashPtr, hashLen, dstPtr) {
          return adapter.readBlob(hashPtr, hashLen, dstPtr);
        },
        write_blob(ptr, len, hashOutPtr) {
          return adapter.writeBlob(ptr, len, hashOutPtr);
        },
        poll_event(bufPtr, bufLen) {
          return adapter.pollEvent(bufPtr, bufLen);
        },
      },
    };
  }

  pushNetworkConnected(sockId) {
    this.eventQueue.push(encodeNetworkEvent(sockId || this.nextSockId++, NetworkSubtype.Connected, null));
  }

  pushNetworkDisconnected(sockId) {
    this.eventQueue.push(encodeNetworkEvent(sockId, NetworkSubtype.Disconnected, null));
  }

  pushNetworkReceived(sockId, data) {
    this.eventQueue.push(encodeNetworkEvent(sockId, NetworkSubtype.Received, data));
  }

  pushNetworkError(sockId) {
    this.eventQueue.push(encodeNetworkEvent(sockId, NetworkSubtype.Error, null));
  }

  pushDiskReadDone(opId, hash, data) {
    this.eventQueue.push(encodeDiskEvent(opId || this.nextOpId++, DiskSubtype.ReadDone, hash, data));
  }

  pushDiskWriteDone(opId, hash) {
    this.eventQueue.push(encodeDiskEvent(opId || this.nextOpId++, DiskSubtype.WriteDone, hash, null));
  }

  pushDiskError(opId) {
    this.eventQueue.push(encodeDiskEvent(opId || this.nextOpId++, DiskSubtype.Error, null, null));
  }

  pushTimerFired(timerId) {
    this.eventQueue.push(encodeTimerEvent(timerId || this.nextTimerId++));
  }

  fetchRequest(url, options) {
    const sockId = this.nextSockId++;
    this.pushNetworkConnected(sockId);

    fetch(url, options)
      .then(async (resp) => {
        const headers = [];
        resp.headers.forEach((v, k) => headers.push(`${k}: ${v}`));
        const body = await resp.arrayBuffer();

        const head = `HTTP/1.1 ${resp.status} ${resp.statusText}\r\n${headers.join('\r\n')}\r\n\r\n`;
        const headBytes = new TextEncoder().encode(head);
        const total = new Uint8Array(headBytes.length + body.byteLength);
        total.set(headBytes);
        total.set(new Uint8Array(body), headBytes.length);
        this.pushNetworkReceived(sockId, total);
        this.pushNetworkDisconnected(sockId);
      })
      .catch(() => {
        this.pushNetworkError(sockId);
        this.pushNetworkDisconnected(sockId);
      });

    return sockId;
  }

  openWebSocket(url, protocols) {
    const sockId = this.nextSockId++;
    const ws = protocols ? new WebSocket(url, protocols) : new WebSocket(url);
    this.sockets.set(sockId, ws);

    ws.onopen = () => {
      this.pushNetworkConnected(sockId);
    };

    ws.onmessage = (msg) => {
      let data;
      if (msg.data instanceof ArrayBuffer) {
        data = new Uint8Array(msg.data);
      } else if (msg.data instanceof Blob) {
        msg.data.arrayBuffer().then(buf => {
          this.pushNetworkReceived(sockId, buf);
        });
        return;
      } else {
        data = new TextEncoder().encode(msg.data);
      }
      this.pushNetworkReceived(sockId, data);
    };

    ws.onclose = () => {
      this.pushNetworkDisconnected(sockId);
      this.sockets.delete(sockId);
    };

    ws.onerror = () => {
      this.pushNetworkError(sockId);
    };

    return sockId;
  }

  sendWebSocket(sockId, data) {
    const ws = this.sockets.get(sockId);
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(data);
      return 0;
    }
    return -1;
  }

  closeWebSocket(sockId) {
    const ws = this.sockets.get(sockId);
    if (ws) {
      ws.close();
      this.sockets.delete(sockId);
    }
  }

  setTimer(intervalMs) {
    const timerId = this.nextTimerId++;
    const id = setTimeout(() => {
      this.pushTimerFired(timerId);
    }, intervalMs);
    this.timers.set(timerId, id);
    return timerId;
  }

  setIntervalTimer(intervalMs) {
    const timerId = this.nextTimerId++;
    const id = setInterval(() => {
      this.pushTimerFired(timerId);
    }, intervalMs);
    this.timers.set(timerId, id);
    return timerId;
  }

  cancelTimer(timerId) {
    const id = this.timers.get(timerId);
    if (id !== undefined) {
      clearTimeout(id);
      this.timers.delete(timerId);
    }
  }

  listenStorageEvent() {
    window.addEventListener('storage', (e) => {
      const data = JSON.stringify({ key: e.key, newValue: e.newValue, oldValue: e.oldValue, url: e.url });
      const encoded = new TextEncoder().encode(data);
      this.pushDiskReadDone(0, null, encoded);
    });
  }

  async loadModule(wasmUrl) {
    const imports = this.getImportObject();
    const { instance } = await WebAssembly.instantiateStreaming(fetch(wasmUrl), imports);

    this.setMemory(instance.exports.memory);

    const runFunc = instance.exports.run;
    if (!runFunc) {
      throw new Error('WASM module missing required export: run');
    }

    return instance;
  }

  getOutput() {
    return this.outputBuffer;
  }

  getMessages() {
    return this.messages;
  }

  // -----------------------------------------------------------------------
  // UI rendering — detects UI content type and renders via DOMRenderer
  // -----------------------------------------------------------------------

  tryRenderUI(data) {
    if (!this.uiContainer) return;

    if (!this.uiRenderer) {
      this.uiRenderer = new DOMRenderer(this.uiContainer, {
        onAction: (action) => this.sendUIAction(action),
        verbose: this.verbose,
      });
    }

    if (data.length >= 10) {
      const ctLen = new DataView(data.buffer, data.byteOffset, data.byteLength)
        .getUint32(2, true);
      const ctStart = 6;
      const ctEnd = ctStart + ctLen;

      if (ctEnd <= data.length) {
        const contentType = new TextDecoder().decode(data.slice(ctStart, ctEnd));
        if (contentType.includes('edgerun-ui')) {
          const bodyOffset = 6 + ctLen;
          if (bodyOffset + 4 <= data.length) {
            const bodyLen = new DataView(data.buffer, data.byteOffset, data.byteLength)
              .getUint32(bodyOffset, true);
            const bodyStart = bodyOffset + 4;
            const uiBytes = data.slice(bodyStart, bodyStart + bodyLen);

            try {
              const node = decodeUINode(uiBytes);
              this.currentUI = node;
              this.uiRenderer.render(node);
            } catch (e) {
              if (this.verbose) {
                console.error('Failed to decode UINode:', e);
              }
            }
          }
        }
      }
    }
  }

  sendUIAction(action) {
    if (this.verbose) {
      console.log(`BrowserAdapter sending action: ${action}`);
    }

    const actionBytes = new TextEncoder().encode(JSON.stringify({
      type: 'ui_action',
      action: action,
    }));

    this.pushNetworkReceived(999, actionBytes);

    if (this.onUIAction) {
      this.onUIAction(action);
    }
  }
}
