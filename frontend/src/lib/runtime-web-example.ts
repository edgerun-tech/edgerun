export type RuntimeDigestResult = {
  bundle_hash: string;
  abi_version: number;
  runtime_id: string;
  output_hash: string;
  output_len: number;
  input_len: number;
  max_memory_bytes: number;
  max_instructions: number;
  fuel_limit: number;
  fuel_remaining: number;
};

type RuntimeWebModule = {
  execute_bundle_payload_bytes_for_runtime_and_abi_digest_strict: (
    bundlePayload: Uint8Array,
    expectedRuntimeIdHex: string,
    expectedAbiVersion: number,
  ) => RuntimeDigestResult;
  validate_wasm_module: (wasm: Uint8Array) => void;
};

let runtimeModulePromise: Promise<RuntimeWebModule> | null = null;

export function loadRuntimeWebModule(): Promise<RuntimeWebModule> {
  if (!runtimeModulePromise) {
    const modulePath: string = "/wasm/edgerun-runtime-web/edgerun_runtime_web.js";
    runtimeModulePromise = import(modulePath) as Promise<RuntimeWebModule>;
  }
  return runtimeModulePromise;
}

export async function executeBundleDigestInBrowser(
  bundlePayload: Uint8Array,
  expectedRuntimeIdHex: string,
  expectedAbiVersion: number,
): Promise<RuntimeDigestResult> {
  const runtime = await loadRuntimeWebModule();
  return runtime.execute_bundle_payload_bytes_for_runtime_and_abi_digest_strict(
    bundlePayload,
    expectedRuntimeIdHex,
    expectedAbiVersion,
  );
}
