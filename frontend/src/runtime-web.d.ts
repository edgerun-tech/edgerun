declare module '/wasm/edgerun-runtime-web/edgerun_runtime_web.js' {
  export default function initRuntimeWeb(moduleOrPath?: string | URL | Request): Promise<unknown>

  export function execute_bundle_payload_bytes_strict(
    bundlePayload: Uint8Array
  ): {
    bundle_hash: string
    abi_version: number
    runtime_id: string
    output_hash: string
    output: Uint8Array
    output_len: number
    input_len: number
    max_memory_bytes: number
    max_instructions: number
    fuel_limit: number
    fuel_remaining: number
  }

  export function execute_bundle_payload_bytes_for_runtime_and_abi_digest_strict(
    bundlePayload: Uint8Array,
    expectedRuntimeIdHex: string,
    expectedAbiVersion: number
  ): {
    bundle_hash: string
    abi_version: number
    runtime_id: string
    output_hash: string
    output_len: number
    input_len: number
    max_memory_bytes: number
    max_instructions: number
    fuel_limit: number
    fuel_remaining: number
  }

  export function validate_wasm_module(wasm: Uint8Array): void
}
