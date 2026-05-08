import { systemStatsStore } from "./desktop-store"

export { systemStatsStore }

let statsInterval: ReturnType<typeof setInterval> | null = null

export function startSystemStatsSimulation() {
  statsInterval = null
}

export function stopSystemStatsSimulation() {
  if (statsInterval) {
    clearInterval(statsInterval)
    statsInterval = null
  }
}

export function generateInitialLogs() {
  return [
    { id: `log-${Date.now()}-1`, timestamp: new Date(), type: "system" as const, message: "System initialized" },
    { id: `log-${Date.now()}-2`, timestamp: new Date(), type: "warning" as const, message: "No node connection detected" },
  ]
}
