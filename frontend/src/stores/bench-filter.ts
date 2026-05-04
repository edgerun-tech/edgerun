import { atom } from "nanostores"
import type { MetricFocus } from "@/data/benchmarks"

export const $metricFocus = atom<MetricFocus>("all")
