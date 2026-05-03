"use client"

import { useState, useEffect, useRef, useCallback } from "react"
import { cn } from "@/lib/utils"
import { Cpu, MemoryStick, Network, HardDrive, Activity, Pause, Play } from "lucide-react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Progress } from "@/components/ui/progress"

interface ResourceData {
  timestamp: number
  cpu: {
    usage: number
    cores: number
  }
  ram: {
    used: number
    total: number
    percentage: number
  }
  network: {
    in: number
    out: number
  }
  disk: {
    read: number
    write: number
  }
  connections: number
}

interface ResourceMonitorProps {
  onLog?: (message: string) => void
}

export function ResourceMonitor({ onLog }: ResourceMonitorProps) {
  const [isMonitoring, setIsMonitoring] = useState(true)
  const [resourceData, setResourceData] = useState<ResourceData | null>(null)
  const [history, setHistory] = useState<ResourceData[]>([])
  const workerRef = useRef<Worker | null>(null)

  const startWorker = useCallback(() => {
    if (workerRef.current) return

    const worker = new Worker("/resource-worker.js")
    workerRef.current = worker

    worker.onmessage = (e) => {
      if (e.data.type === "DATA") {
        const data: ResourceData = e.data.payload
        setResourceData(data)
        setHistory((prev) => {
          const updated = [...prev, data]
          // Keep last 60 data points
          return updated.length > 60 ? updated.slice(-60) : updated
        })
      } else if (e.data.type === "ERROR") {
        onLog?.(`Worker error: ${e.data.payload.message}`)
      }
    }

    worker.onerror = (error) => {
      onLog?.(`Worker error: ${error.message}`)
    }

    worker.postMessage({ type: "START", interval: 1000 })
    onLog?.("Resource monitor started")
  }, [onLog])

  const stopWorker = useCallback(() => {
    if (workerRef.current) {
      workerRef.current.postMessage({ type: "STOP" })
      workerRef.current.terminate()
      workerRef.current = null
      onLog?.("Resource monitor stopped")
    }
  }, [onLog])

  useEffect(() => {
    if (isMonitoring) {
      startWorker()
    } else {
      stopWorker()
    }

    return () => {
      stopWorker()
    }
  }, [isMonitoring, startWorker, stopWorker])

  const toggleMonitoring = () => {
    setIsMonitoring((prev) => !prev)
  }

  const cpuHistory = history.map((d) => d.cpu.usage)
  const ramHistory = history.map((d) => d.ram.percentage)
  const maxCpu = Math.max(...cpuHistory, 1)
  const maxRam = Math.max(...ramHistory, 1)

  return (
    <div className="flex h-full flex-col gap-3 p-4 overflow-auto">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-foreground">Resource Monitor</h2>
        <Button
          size="sm"
          variant={isMonitoring ? "destructive" : "default"}
          onClick={toggleMonitoring}
          className="gap-1.5"
        >
          {isMonitoring ? (
            <>
              <Pause className="h-3.5 w-3.5" /> Pause
            </>
          ) : (
            <>
              <Play className="h-3.5 w-3.5" /> Resume
            </>
          )}
        </Button>
      </div>

      {resourceData && (
        <div className="grid grid-cols-2 gap-3">
          {/* CPU Card */}
          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">CPU</CardTitle>
              <Cpu className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{resourceData.cpu.usage}%</div>
              <p className="text-xs text-muted-foreground">
                {resourceData.cpu.cores} cores
              </p>
              <div className="mt-3 h-12 flex items-end gap-0.5">
                {cpuHistory.slice(-20).map((val, i) => (
                  <div
                    key={i}
                    className="flex-1 bg-primary/60 rounded-sm"
                    style={{ height: `${(val / maxCpu) * 100}%` }}
                  />
                ))}
              </div>
            </CardContent>
          </Card>

          {/* RAM Card */}
          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">Memory</CardTitle>
              <MemoryStick className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">
                {resourceData.ram.used} GB
              </div>
              <p className="text-xs text-muted-foreground">
                of {resourceData.ram.total} GB ({resourceData.ram.percentage}%)
              </p>
              <Progress value={resourceData.ram.percentage} className="mt-3" />
              <div className="mt-2 h-12 flex items-end gap-0.5">
                {ramHistory.slice(-20).map((val, i) => (
                  <div
                    key={i}
                    className="flex-1 bg-blue-500/60 rounded-sm"
                    style={{ height: `${(val / maxRam) * 100}%` }}
                  />
                ))}
              </div>
            </CardContent>
          </Card>

          {/* Network Card */}
          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">Network</CardTitle>
              <Network className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="space-y-1">
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">↓ In</span>
                  <span className="font-medium">{resourceData.network.in.toFixed(1)} KB/s</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">↑ Out</span>
                  <span className="font-medium">{resourceData.network.out.toFixed(1)} KB/s</span>
                </div>
              </div>
              <p className="mt-2 text-xs text-muted-foreground">
                {resourceData.connections} active connections
              </p>
            </CardContent>
          </Card>

          {/* Disk Card */}
          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">Disk I/O</CardTitle>
              <HardDrive className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="space-y-1">
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Read</span>
                  <span className="font-medium">{resourceData.disk.read.toFixed(1)} KB/s</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Write</span>
                  <span className="font-medium">{resourceData.disk.write.toFixed(1)} KB/s</span>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      {/* Historical Chart */}
      {history.length > 0 && (
        <Card className="flex-1">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <Activity className="h-4 w-4" />
              CPU & Memory History
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="h-32 flex items-end gap-0.5 border-b border-l relative">
              {history.slice(-60).map((data, i) => (
                <div
                  key={i}
                  className="flex-1 flex flex-col items-end gap-px"
                  style={{ height: "100%" }}
                >
                  <div
                    className="w-full bg-primary/70 rounded-t-sm"
                    style={{
                      height: `${(data.cpu.usage / 100) * 100}%`,
                    }}
                  />
                  <div
                    className="w-full bg-blue-500/50 rounded-b-sm"
                    style={{
                      height: `${(data.ram.percentage / 100) * 100}%`,
                    }}
                  />
                </div>
              ))}
              <div className="absolute bottom-0 left-0 text-[10px] text-muted-foreground">0%</div>
              <div className="absolute top-0 left-0 text-[10px] text-muted-foreground">100%</div>
            </div>
            <div className="mt-2 flex items-center gap-4 text-[10px]">
              <span className="flex items-center gap-1">
                <span className="h-2 w-2 rounded-sm bg-primary/70" />
                CPU
              </span>
              <span className="flex items-center gap-1">
                <span className="h-2 w-2 rounded-sm bg-blue-500/50" />
                RAM
              </span>
            </div>
          </CardContent>
        </Card>
      )}

      {!resourceData && (
        <div className="flex flex-1 items-center justify-center">
          <p className="text-sm text-muted-foreground">
            {isMonitoring ? "Initializing monitor..." : "Monitoring paused"}
          </p>
        </div>
      )}
    </div>
  )
}
