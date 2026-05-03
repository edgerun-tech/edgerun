import { systemStatsStore } from "./desktop-store"

export { systemStatsStore }

let statsInterval: ReturnType<typeof setInterval> | null = null

export function startSystemStatsSimulation() {
  if (statsInterval) return

  statsInterval = setInterval(() => {
    const stats = systemStatsStore.get()
    systemStatsStore.set({
      ...stats,
      nodeCount: Math.max(5, Math.min(24, stats.nodeCount + Math.floor(Math.random() * 3) - 1)),
    })
  }, 5000)
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
    { id: `log-${Date.now()}-2`, timestamp: new Date(), type: "success" as const, message: "Connected to Edgerun network — 12 peers online" },
  ]
}
