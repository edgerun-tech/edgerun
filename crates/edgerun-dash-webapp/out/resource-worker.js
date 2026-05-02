// Resource monitoring web worker
// Runs in background to collect system stats without blocking UI

let intervalId = null
let isRunning = false

// Simulate system resource data collection
function collectResourceData() {
  const now = Date.now()

  // Simulate CPU usage (0-100%)
  const cpuUsage = Math.min(100, Math.max(0, 45 + Math.random() * 30 - 15))

  // Simulate RAM usage
  const ramTotal = 8 // GB
  const ramUsed = Math.min(ramTotal, Math.max(0.5, 4.2 + Math.random() * 1.5 - 0.75))
  const ramPercentage = (ramUsed / ramTotal) * 100

  // Simulate network I/O
  const networkIn = Math.max(0, 120 + Math.random() * 80 - 40)
  const networkOut = Math.max(0, 80 + Math.random() * 60 - 30)

  // Simulate disk I/O
  const diskRead = Math.max(0, 50 + Math.random() * 40 - 20)
  const diskWrite = Math.max(0, 30 + Math.random() * 30 - 15)

  // Simulate active connections
  const activeConnections = Math.max(1, Math.floor(12 + Math.random() * 6 - 3))

  // Generate historical data point
  return {
    timestamp: now,
    cpu: {
      usage: parseFloat(cpuUsage.toFixed(1)),
      cores: navigator.hardwareConcurrency || 4,
    },
    ram: {
      used: parseFloat(ramUsed.toFixed(1)),
      total: ramTotal,
      percentage: parseFloat(ramPercentage.toFixed(1)),
    },
    network: {
      in: parseFloat(networkIn.toFixed(1)),
      out: parseFloat(networkOut.toFixed(1)),
    },
    disk: {
      read: parseFloat(diskRead.toFixed(1)),
      write: parseFloat(diskWrite.toFixed(1)),
    },
    connections: activeConnections,
  }
}

self.onmessage = function (e) {
  const data = e.data

  switch (data.type) {
    case "START":
      if (!isRunning) {
        isRunning = true
        const interval = data.interval || 1000

        // Send initial data
        self.postMessage({ type: "DATA", payload: collectResourceData() })

        // Start periodic collection
        intervalId = setInterval(() => {
          if (isRunning) {
            self.postMessage({ type: "DATA", payload: collectResourceData() })
          }
        }, interval)
      }
      break

    case "STOP":
      isRunning = false
      if (intervalId) {
        clearInterval(intervalId)
        intervalId = null
      }
      break

    case "GET_SNAPSHOT":
      self.postMessage({ type: "DATA", payload: collectResourceData() })
      break

    default:
      console.warn("Unknown message type:", data.type)
  }
}

self.onerror = function (error) {
  self.postMessage({ type: "ERROR", payload: { message: error.message } })
}
