export type MetricFocus = "all" | "throughput" | "latency" | "memory" | "cpu"

export interface BenchmarkSeries {
  id: string
  suite: string
  workload: string
  branch: string
  environment: string
  commit: string
  date: string
  throughputOpsPerSec: number
  latencyP99Ms: number
  memoryPeakGiB: number
  cpuAvgPercent: number
}

export interface FairnessRule {
  metric: string
  weight: number
  normalizedScore: number
  confidence: "high" | "medium" | "low"
  notes: string
}

export interface BenchmarkRunResult {
  run: BenchmarkSeries
  fairness: FairnessRule[]
}

export const benchmarkResults: BenchmarkRunResult[] = [
  {
    run: {
      id: "run-2026-04-core",
      suite: "Core Router Dispatch",
      workload: "mixed",
      branch: "main",
      environment: "16 vCPU / 32GB / NVMe",
      commit: "a8b91f4",
      date: "2026-04-28",
      throughputOpsPerSec: 18400,
      latencyP99Ms: 38,
      memoryPeakGiB: 4.7,
      cpuAvgPercent: 52,
    },
    fairness: [
      {
        metric: "Throughput",
        weight: 40,
        normalizedScore: 86,
        confidence: "high",
        notes: "3 runs, CI template template-b04",
      },
      {
        metric: "P99 Latency",
        weight: 30,
        normalizedScore: 91,
        confidence: "high",
        notes: "Lower is better, stable under load",
      },
      {
        metric: "Memory Peak",
        weight: 20,
        normalizedScore: 88,
        confidence: "medium",
        notes: "Large cache warmup phase excluded",
      },
      {
        metric: "CPU",
        weight: 10,
        normalizedScore: 74,
        confidence: "high",
        notes: "Measured as average over 10m run",
      },
    ],
  },
  {
    run: {
      id: "run-2026-04-edge",
      suite: "Edge Admission Path",
      workload: "burst",
      branch: "release",
      environment: "8 vCPU / 16GB / c5a.large",
      commit: "4d7f1ca",
      date: "2026-04-29",
      throughputOpsPerSec: 11250,
      latencyP99Ms: 47,
      memoryPeakGiB: 3.9,
      cpuAvgPercent: 61,
    },
    fairness: [
      {
        metric: "Throughput",
        weight: 40,
        normalizedScore: 79,
        confidence: "high",
        notes: "Same seed set as run-2026-04-core",
      },
      {
        metric: "P99 Latency",
        weight: 30,
        normalizedScore: 83,
        confidence: "high",
        notes: "Tail latency improved after queue warmup",
      },
      {
        metric: "Memory Peak",
        weight: 20,
        normalizedScore: 92,
        confidence: "high",
        notes: "Very stable peak envelope",
      },
      {
        metric: "CPU",
        weight: 10,
        normalizedScore: 70,
        confidence: "medium",
        notes: "Burst amplification visible on edge gateways",
      },
    ],
  },
  {
    run: {
      id: "run-2026-05-01-workshop",
      suite: "Workflow Execution",
      workload: "stateful",
      branch: "main",
      environment: "12 vCPU / 64GB / NVMe",
      commit: "c1e3d90",
      date: "2026-05-01",
      throughputOpsPerSec: 9600,
      latencyP99Ms: 64,
      memoryPeakGiB: 6.2,
      cpuAvgPercent: 68,
    },
    fairness: [
      {
        metric: "Throughput",
        weight: 40,
        normalizedScore: 68,
        confidence: "medium",
        notes: "State-heavy tests produce variance",
      },
      {
        metric: "P99 Latency",
        weight: 30,
        normalizedScore: 65,
        confidence: "high",
        notes: "Noisy neighbors controlled with CPU pinning",
      },
      {
        metric: "Memory Peak",
        weight: 20,
        normalizedScore: 74,
        confidence: "medium",
        notes: "Includes cache prefill on first run",
      },
      {
        metric: "CPU",
        weight: 10,
        normalizedScore: 62,
        confidence: "low",
        notes: "Need one extra warmup cycle for final confidence",
      },
    ],
  },
]
