// SPDX-License-Identifier: Apache-2.0

export function installLocalBridgeSimulator(win) {
  class FakeBridgeWebSocket {
    constructor(url) {
      this.url = url
      this.readyState = 0
      this.binaryType = 'arraybuffer'
      setTimeout(() => {
        this.readyState = 1
        if (typeof this.onopen === 'function') this.onopen()
      }, 0)
    }

    send(payload) {
      const data = payload instanceof Uint8Array
        ? payload.buffer.slice(payload.byteOffset, payload.byteOffset + payload.byteLength)
        : payload
      setTimeout(() => {
        if (typeof this.onmessage === 'function') this.onmessage({ data })
      }, 0)
    }

    close() {
      this.readyState = 3
      if (typeof this.onclose === 'function') this.onclose()
    }
  }

  const originalFetch = win.fetch.bind(win)
  win.fetch = (input, init) => {
    const url = String(typeof input === 'string' ? input : input?.url || '')
    if (url.includes('/v1/local/node/info.pb')) {
      return Promise.resolve(
        new win.Response(new Uint8Array([8, 1]), {
          status: 200,
          headers: { 'content-type': 'application/octet-stream' }
        })
      )
    }
    return originalFetch(input, init)
  }

  win.WebSocket = FakeBridgeWebSocket
}
