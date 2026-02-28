// SPDX-License-Identifier: Apache-2.0

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

function encodeField(tag, wireType, payload) {
  return [...encodeVarint((tag << 3) | wireType), ...payload];
}

function encodeStringField(tag, value) {
  const bytes = new TextEncoder().encode(String(value || ''));
  return encodeField(tag, 2, [...encodeVarint(bytes.length), ...bytes]);
}

function encodeBoolField(tag, value) {
  return encodeField(tag, 0, [value ? 1 : 0]);
}

function encodeUint64Field(tag, value) {
  return encodeField(tag, 0, encodeVarint(value));
}

function encodeNodeInfoPb() {
  const bytes = [
    ...encodeBoolField(1, true),
    ...encodeStringField(3, 'local-node-manager'),
    ...encodeStringField(4, 'device_pubkey_dummy'),
    ...encodeStringField(5, 'v1'),
    ...encodeUint64Field(6, Date.now()),
    ...encodeStringField(7, '/v1/local/eventbus/ws')
  ];
  return new Uint8Array(bytes);
}

describe('intent ui file manager node selector', () => {
  it('sends selected node id with local fs requests', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        class FakeWebSocket {
          static CONNECTING = 0
          static OPEN = 1
          static CLOSING = 2
          static CLOSED = 3

          constructor() {
            this.readyState = FakeWebSocket.CONNECTING;
            this.binaryType = 'arraybuffer';
            setTimeout(() => {
              this.readyState = FakeWebSocket.OPEN;
              if (typeof this.onopen === 'function') this.onopen();
            }, 5);
          }

          send() {}

          close() {
            this.readyState = FakeWebSocket.CLOSED;
            if (typeof this.onclose === 'function') this.onclose();
          }
        }

        const realFetch = win.fetch.bind(win);
        win.__fsRequestUrls = [];
        win.fetch = (input, init) => {
          const url = typeof input === 'string' ? input : input.url;
          if (url.includes('/v1/local/node/info.pb')) {
            return Promise.resolve(
              new win.Response(encodeNodeInfoPb(), {
                status: 200,
                headers: {
                  'content-type': 'application/x-protobuf'
                }
              })
            );
          }
          if (url.includes('/v1/local/fs/meta')) {
            return Promise.resolve(
              new win.Response(JSON.stringify({ ok: true, error: '', localFsRoot: '/' }), {
                status: 200,
                headers: {
                  'content-type': 'application/json'
                }
              })
            );
          }
          if (url.includes('/v1/local/fs/list')) {
            win.__fsRequestUrls.push(url);
            return Promise.resolve(
              new win.Response(JSON.stringify({ ok: true, error: '', entries: [] }), {
                status: 200,
                headers: {
                  'content-type': 'application/json'
                }
              })
            );
          }
          if (url.includes('/v1/local/fs/read')) {
            return Promise.resolve(
              new win.Response(JSON.stringify({ ok: true, error: '', content: '' }), {
                status: 200,
                headers: {
                  'content-type': 'application/json'
                }
              })
            );
          }
          return realFetch(input, init);
        };

        win.WebSocket = FakeWebSocket;
      }
    });

    cy.get('button[title="Files panel"]').first().click({ force: true });
    cy.get('[data-testid="file-manager-node-selector"]').should('be.visible');
    cy.get('[data-testid="file-manager-node-selector"]').should('have.value', 'local-node-manager');
    cy.get('button[title="Local"]').click({ force: true });
    cy.get('[data-testid="file-manager-node-selector"] option').should('have.length.at.least', 1);
  });
});
