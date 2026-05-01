import { BrowserAdapter } from './adapter.js';
import { readFileSync } from 'fs';

const adapter = new BrowserAdapter({ verbose: true });

const wasmPath = new URL('app.wasm', import.meta.url);
const wasmBytes = readFileSync(wasmPath);

const imports = adapter.getImportObject();
const { instance } = await WebAssembly.instantiate(wasmBytes, imports);

adapter.setMemory(instance.exports.memory);
const run = instance.exports.run;

console.log('Running WASM module...');
const result = run(0, 0);
console.log(`run() returned: ${result}`);

const output = adapter.getOutput();
if (output.length > 0) {
  const data = output[0];
  if (data.length >= 10) {
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
    const status = view.getUint16(0, true);
    const ctLen = view.getUint32(2, true);
    const ct = new TextDecoder().decode(data.slice(6, 6 + ctLen));
    const bodyOffset = 6 + ctLen;
    const bodyLen = view.getUint32(bodyOffset, true);
    const body = new TextDecoder().decode(data.slice(bodyOffset + 4, bodyOffset + 4 + bodyLen));
    console.log(`Response: [${status}] ${ct}`);
    console.log(body);
  }
}

adapter.pushTimerFired(42);
const pollResult = run(0, 0);
console.log(`Second run returned: ${pollResult}`);
