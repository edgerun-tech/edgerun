declare module "/wasm/edgerun-runtime-web/edgerun_runtime_web.js" {
  export function validate_wasm_module(wasm: Uint8Array): void;
  export function execute_bundle_payload_bytes_strict(
    bundlePayload: Uint8Array,
  ): unknown;
  export function execute_bundle_payload_bytes_for_runtime_and_abi_digest_strict(
    bundlePayload: Uint8Array,
    expectedRuntimeIdHex: string,
    expectedAbiVersion: number,
  ): unknown;
}
