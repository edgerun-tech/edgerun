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
  const bytes = new TextEncoder().encode(String(value || ""));
  return encodeField(tag, 2, [...encodeVarint(bytes.length), ...bytes]);
}

function encodeBytesField(tag, bytes) {
  const arr = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes || []);
  return encodeField(tag, 2, [...encodeVarint(arr.length), ...arr]);
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
    ...encodeStringField(3, "local-node-manager"),
    ...encodeStringField(4, "device_pubkey_dummy"),
    ...encodeStringField(5, "v1"),
    ...encodeUint64Field(6, Date.now()),
    ...encodeStringField(7, "/v1/local/eventbus/ws")
  ];
  return new Uint8Array(bytes);
}

function encodeLocalEventEnvelope(topic, payload) {
  const payloadBytes = new TextEncoder().encode(
    JSON.stringify({
      payload: payload || {},
      meta: {}
    })
  );
  const bytes = [
    ...encodeStringField(1, `evt-${Date.now()}`),
    ...encodeStringField(2, topic),
    ...encodeBytesField(3, payloadBytes),
    ...encodeStringField(4, "node-manager"),
    ...encodeUint64Field(5, Date.now())
  ];
  return new Uint8Array(bytes);
}

describe("intent ui docker logs panel", () => {
  it("renders docker logs from local bridge event stream", () => {
    cy.viewport(1400, 900);
    cy.visit("/intent-ui/", {
      onBeforeLoad(win) {
        class FakeWebSocket {
          static CONNECTING = 0;
          static OPEN = 1;
          static CLOSING = 2;
          static CLOSED = 3;

          constructor() {
            this.readyState = FakeWebSocket.CONNECTING;
            this.binaryType = "arraybuffer";
            setTimeout(() => {
              this.readyState = FakeWebSocket.OPEN;
              if (typeof this.onopen === "function") this.onopen();
              setTimeout(() => {
                if (typeof this.onmessage === "function") {
                  const envelope = encodeLocalEventEnvelope("local.docker.events", {
                    container_name: "caddy",
                    message: "reverse proxy ready"
                  });
                  this.onmessage({ data: envelope.buffer });
                }
              }, 10);
            }, 5);
          }

          send() {}

          close() {
            this.readyState = FakeWebSocket.CLOSED;
            if (typeof this.onclose === "function") this.onclose();
          }
        }

        const realFetch = win.fetch.bind(win);
        win.fetch = (input, init) => {
          const url = typeof input === "string" ? input : input.url;
          if (url.includes("/v1/local/node/info.pb")) {
            return Promise.resolve(
              new win.Response(encodeNodeInfoPb(), {
                status: 200,
                headers: { "content-type": "application/x-protobuf" }
              })
            );
          }
          return realFetch(input, init);
        };

        win.WebSocket = FakeWebSocket;
      }
    });

    cy.get("[data-testid='floating-feed-panel-docker-logs']").should("be.visible");
    cy.get("[data-testid='floating-feed-panel-scroll-docker-logs']")
      .contains("[caddy] reverse proxy ready")
      .should("be.visible");
  });
});
