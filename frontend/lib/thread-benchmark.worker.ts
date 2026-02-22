// SPDX-License-Identifier: Apache-2.0
type CpuBenchmarkRequest = {
  id: string
  type: 'cpu-benchmark'
  iterations: number
  workScale: number
}

type PayloadProbeRequest = {
  id: string
  type: 'payload-probe'
  payload: ArrayBuffer
}

type BenchmarkRequest = CpuBenchmarkRequest | PayloadProbeRequest

type CpuBenchmarkResponse = {
  id: string
  type: 'cpu-benchmark'
  totalMs: number
  durationsMs: number[]
  checksum: number
}

type PayloadProbeResponse = {
  id: string
  type: 'payload-probe'
  bytes: number
  checksum: number
}

self.onmessage = (event: MessageEvent<BenchmarkRequest>) => {
  void handleMessage(event.data)
}

async function handleMessage(message: BenchmarkRequest): Promise<void> {
  if (!message || typeof message.id !== 'string') return
  if (message.type === 'cpu-benchmark') {
    const result = runCpuBenchmark(message.iterations, message.workScale)
    const response: CpuBenchmarkResponse = {
      id: message.id,
      type: 'cpu-benchmark',
      totalMs: result.totalMs,
      durationsMs: result.durationsMs,
      checksum: result.checksum
    }
    self.postMessage(response)
    return
  }
  if (message.type === 'payload-probe') {
    const bytes = message.payload.byteLength
    const view = new Uint8Array(message.payload)
    const checksum = sampleChecksum(view)
    const response: PayloadProbeResponse = {
      id: message.id,
      type: 'payload-probe',
      bytes,
      checksum
    }
    self.postMessage(response)
  }
}

function runCpuBenchmark(iterations: number, workScale: number): {
  totalMs: number
  durationsMs: number[]
  checksum: number
} {
  const runs = Math.max(1, Math.floor(iterations))
  const scale = Math.max(1000, Math.floor(workScale))
  const durationsMs: number[] = []
  let checksum = 0
  const start = performance.now()
  for (let i = 0; i < runs; i += 1) {
    const runStart = performance.now()
    checksum ^= computeWork(scale + i * 17)
    durationsMs.push(performance.now() - runStart)
  }
  const totalMs = performance.now() - start
  return { totalMs, durationsMs, checksum: checksum >>> 0 }
}

function computeWork(size: number): number {
  let value = 0x811c9dc5
  for (let i = 0; i < size; i += 1) {
    value ^= (i + 0x9e3779b9) >>> 0
    value = Math.imul(value, 0x01000193) >>> 0
    value = ((value << 7) | (value >>> 25)) >>> 0
  }
  return value >>> 0
}

function sampleChecksum(bytes: Uint8Array): number {
  let checksum = bytes.byteLength >>> 0
  const stride = 4096
  for (let i = 0; i < bytes.byteLength; i += stride) {
    checksum = (checksum + bytes[i] + ((i / stride) & 0xff)) >>> 0
  }
  return checksum >>> 0
}
