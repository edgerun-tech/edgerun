#!/usr/bin/env node
const assert = require("assert");
const { execFileSync } = require("child_process");
const { readFileSync, mkdtempSync, rmSync } = require("fs");
const { tmpdir } = require("os");
const { join } = require("path");

const root = process.cwd();
const wat = join(root, "standards/build/wasm/codec-primitives/http-client-core.wat");
const dir = mkdtempSync(join(tmpdir(), "edgerun-http-client-core-"));
const wasm = join(dir, "http-client-core.wasm");

try {
  execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });
  const mod = new WebAssembly.Module(readFileSync(wasm));
  const { exports: e } = new WebAssembly.Instance(mod);

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300135);
  assert.strictEqual(e.http_client_default_connect_timeout_secs(), 10);
  assert.strictEqual(e.http_client_default_read_timeout_secs(), 30);
  assert.strictEqual(e.http_client_default_redirect_limit(), 10);
  assert.strictEqual(e.http1_pool_default_max_per_host(), 6);
  assert.strictEqual(e.http1_pool_default_idle_timeout_secs(), 30);
  assert.strictEqual(e.http1_pool_default_dns_timeout_secs(), 5);
  assert.strictEqual(e.http2_max_body_size(), 100 * 1024 * 1024);
  assert.strictEqual(e.http2_idle_ping_secs(), 30);
  assert.strictEqual(e.http2_ping_timeout_secs(), 10);
  assert.strictEqual(e.tls_session_cache_default_per_server(), 4);

  assert.strictEqual(e.http_client_route(1, 1, 0, 0), 1);
  assert.strictEqual(e.http_client_route(2, 1, 0, 0), 2);
  assert.strictEqual(e.http_client_route(3, 1, 0, 0), 3);
  assert.strictEqual(e.http_client_route(5, 1, 0, 0), 5);
  assert.strictEqual(e.http_client_route(5, 1, 1, 0), 1);
  assert.strictEqual(e.http_client_route(4, 1, 0, 0), 4);
  assert.strictEqual(e.http_client_route(4, 1, 0, 1), 5);
  assert.strictEqual(e.http_client_route(4, 0, 0, 0), 5);
  assert.strictEqual(e.http_client_route(4, 0, 1, 0), 1);

  assert.strictEqual(e.http_client_best_winner(1, 1, 0), 3);
  assert.strictEqual(e.http_client_best_winner(1, 1, 1), 2);
  assert.strictEqual(e.http_client_best_winner(0, 1, 0), 2);

  assert.strictEqual(e.http_redirect_action(1, 2, 302, 1, 2), 4);
  assert.strictEqual(e.http_redirect_action(1, 2, 303, 1, 2), 6);
  assert.strictEqual(e.http_redirect_action(1, 0, 302, 1, 1), 1);
  assert.strictEqual(e.http_redirect_action(0, 2, 302, 1, 1), 1);

  assert.strictEqual(e.http1_pool_action(2, 0, 1, 0, 6), 1);
  assert.strictEqual(e.http1_pool_action(2, 1, 1, 0, 6), 2);
  assert.strictEqual(e.http1_pool_action(0, 0, 1, 6, 6), 4);
  assert.strictEqual(e.http1_pool_action(0, 0, 0, 0, 6), 3);
  assert.strictEqual(e.http1_response_allows_reuse(0, 1), 1);
  assert.strictEqual(e.http1_response_allows_reuse(1, 1), 0);
  assert.strictEqual(e.http1_default_port(1, 0), 443);
  assert.strictEqual(e.http1_default_port(0, 0), 80);
  assert.strictEqual(e.http1_default_port(1, 8443), 8443);

  assert.strictEqual(e.http_request_builder_header_mask(0, 0, 0, 1), 15);
  assert.strictEqual(e.http_request_builder_header_mask(1, 1, 1, 0), 0);
  assert.strictEqual(e.http_decompress_gate(1, 1, 0), 1);
  assert.strictEqual(e.http_decompress_gate(1, 2, 0), 1);
  assert.strictEqual(e.http_decompress_gate(1, 1, 1), 0);
  assert.strictEqual(e.http_decompress_gate(0, 1, 0), 0);

  assert.strictEqual(e.http2_next_client_stream_id(1), 3);
  assert.strictEqual(e.http2_request_end_stream(0, 0), 1);
  assert.strictEqual(e.http2_request_end_stream(1, 0), 1);
  assert.strictEqual(e.http2_request_end_stream(1, 9), 0);
  assert.strictEqual(e.http2_data_frame_count(0, 16384), 0);
  assert.strictEqual(e.http2_data_frame_count(16385, 16384), 2);
  assert.strictEqual(e.http2_keepalive_action(1, 0, 0, 0), 1);
  assert.strictEqual(e.http2_keepalive_action(0, 1, 0, 1), 2);
  assert.strictEqual(e.http2_keepalive_action(0, 1, 1, 0), 5);

  assert.strictEqual(e.http3_initial_bidi_stream(0), 0);
  assert.strictEqual(e.http3_initial_bidi_stream(1), 1);
  assert.strictEqual(e.http3_initial_uni_stream(0), 2);
  assert.strictEqual(e.http3_initial_uni_stream(1), 3);
  assert.strictEqual(e.http3_preface_stream_count(), 3);
  assert.strictEqual(e.http3_uni_stream_role(0), 0);
  assert.strictEqual(e.http3_uni_stream_role(2), 2);
  assert.strictEqual(e.http3_uni_stream_role(3), 3);
  assert.strictEqual(e.http3_uni_stream_role(1), 1);
  assert.strictEqual(e.http3_uni_stream_role(99), 9);
  assert.strictEqual(e.http3_control_frame_allowed(4), 1);
  assert.strictEqual(e.http3_control_frame_allowed(0), 0);
  assert.strictEqual(e.http3_goaway_accept_new_stream(11n, 9n), 1);
  assert.strictEqual(e.http3_goaway_accept_new_stream(11n, 13n), 0);

  assert.strictEqual(e.middleware_chain_index(3, 0), 0);
  assert.strictEqual(e.middleware_chain_index(3, 2), 2);
  assert.strictEqual(e.middleware_chain_index(3, 3), -1);
  assert.strictEqual(e.tls_session_ticket_action(1, 0, 0, 4), 0);
  assert.strictEqual(e.tls_session_ticket_action(0, 1, 0, 4), 0);
  assert.strictEqual(e.tls_session_ticket_action(0, 0, 4, 4), 4);
  assert.strictEqual(e.tls_session_ticket_action(0, 0, 2, 4), 1);
  assert.strictEqual(e.tls_obfuscated_ticket_age(250, 0xfffffff0), 234);
} finally {
  rmSync(dir, { recursive: true, force: true });
}

console.log("http-client-core smoke ok");
