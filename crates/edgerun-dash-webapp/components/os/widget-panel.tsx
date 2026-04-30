"use client"

import { useState, useRef, useEffect, useCallback } from "react"
import {
  Music2,
  SkipBack,
  SkipForward,
  Play,
  Pause,
  Volume2,
  Lightbulb,
  Thermometer,
  Wind,
  ChevronRight,
  Droplets,
} from "lucide-react"
import { cn } from "@/lib/utils"

// ─── Types ────────────────────────────────────────────────────────────────────

interface Track {
  title: string
  artist: string
  duration: number // seconds
}

interface Room {
  id: string
  name: string
  brightness: number // 0-100
  on: boolean
}

type AcMode = "cool" | "heat" | "fan" | "auto"

// ─── Shared slider primitive ──────────────────────────────────────────────────

function Slider({
  value,
  min = 0,
  max = 100,
  onChange,
  accentColor = "var(--primary)",
  className,
}: {
  value: number
  min?: number
  max?: number
  onChange: (v: number) => void
  accentColor?: string
  className?: string
}) {
  const pct = ((value - min) / (max - min)) * 100

  return (
    <div className={cn("relative h-1.5 w-full rounded-full bg-secondary", className)}>
      <div
        className="absolute left-0 top-0 h-full rounded-full transition-all duration-75"
        style={{ width: `${pct}%`, background: accentColor }}
      />
      <input
        type="range"
        min={min}
        max={max}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="absolute inset-0 h-full w-full cursor-pointer opacity-0"
      />
    </div>
  )
}

// ─── Music Widget ─────────────────────────────────────────────────────────────

const TRACKS: Track[] = [
  { title: "Dark Matter", artist: "Edge Protocol", duration: 214 },
  { title: "Node Sync", artist: "Binary Ghost", duration: 187 },
  { title: "Runtime", artist: "Null Pointer", duration: 243 },
  { title: "Signal Loss", artist: "The Interrupt", duration: 198 },
]

function MusicWidget() {
  const [trackIdx, setTrackIdx] = useState(0)
  const [playing, setPlaying] = useState(false)
  const [progress, setProgress] = useState(0)
  const [volume, setVolume] = useState(72)
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

  const track = TRACKS[trackIdx]

  const fmt = (s: number) => {
    const m = Math.floor(s / 60)
    const sec = Math.floor(s % 60)
    return `${m}:${sec.toString().padStart(2, "0")}`
  }

  useEffect(() => {
    if (playing) {
      intervalRef.current = setInterval(() => {
        setProgress((p) => {
          if (p >= track.duration) {
            next()
            return 0
          }
          return p + 1
        })
      }, 1000)
    } else {
      if (intervalRef.current) clearInterval(intervalRef.current)
    }
    return () => { if (intervalRef.current) clearInterval(intervalRef.current) }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [playing, trackIdx])

  const next = useCallback(() => {
    setTrackIdx((i) => (i + 1) % TRACKS.length)
    setProgress(0)
  }, [])

  const prev = useCallback(() => {
    setTrackIdx((i) => (i - 1 + TRACKS.length) % TRACKS.length)
    setProgress(0)
  }, [])

  const pct = (progress / track.duration) * 100

  // Waveform bars (static visual, animated when playing)
  const bars = [3, 6, 9, 5, 11, 8, 4, 10, 7, 5, 9, 6]

  return (
    <div className="widget-card">
      <div className="widget-header">
        <Music2 className="h-3.5 w-3.5 text-primary" />
        <span>Music</span>
      </div>

      {/* Waveform */}
      <div className="mb-3 flex h-8 items-end justify-center gap-px">
        {bars.map((h, i) => (
          <div
            key={i}
            className={cn(
              "w-1 rounded-sm transition-all",
              playing ? "bg-primary" : "bg-secondary"
            )}
            style={{
              height: playing ? `${h * 3}px` : "6px",
              animation: playing ? `widget-bar ${0.4 + i * 0.05}s ease-in-out infinite alternate` : "none",
              animationDelay: `${i * 40}ms`,
            }}
          />
        ))}
      </div>

      {/* Track info */}
      <div className="mb-2 text-center">
        <p className="truncate text-sm font-semibold text-foreground">{track.title}</p>
        <p className="text-[11px] text-muted-foreground">{track.artist}</p>
      </div>

      {/* Progress */}
      <div className="mb-1 flex items-center gap-2">
        <span className="w-8 text-right font-mono text-[10px] text-muted-foreground">{fmt(progress)}</span>
        <Slider value={pct} onChange={(v) => setProgress((v / 100) * track.duration)} />
        <span className="w-8 font-mono text-[10px] text-muted-foreground">{fmt(track.duration)}</span>
      </div>

      {/* Controls */}
      <div className="mb-3 flex items-center justify-center gap-4">
        <button onClick={prev} className="widget-icon-btn"><SkipBack className="h-4 w-4" /></button>
        <button
          onClick={() => setPlaying((p) => !p)}
          className="flex h-9 w-9 items-center justify-center rounded-full bg-primary text-primary-foreground transition-transform hover:scale-105 active:scale-95"
        >
          {playing ? <Pause className="h-4 w-4" /> : <Play className="h-4 w-4 translate-x-px" />}
        </button>
        <button onClick={next} className="widget-icon-btn"><SkipForward className="h-4 w-4" /></button>
      </div>

      {/* Volume */}
      <div className="flex items-center gap-2">
        <Volume2 className="h-3 w-3 flex-shrink-0 text-muted-foreground" />
        <Slider value={volume} onChange={setVolume} />
        <span className="w-7 text-right font-mono text-[10px] text-muted-foreground">{volume}</span>
      </div>
    </div>
  )
}

// ─── Lights Widget ────────────────────────────────────────────────────────────

const SCENES = ["Ambient", "Focus", "Movie", "Night", "Off"]
const INITIAL_ROOMS: Room[] = [
  { id: "r1", name: "Office", brightness: 80, on: true },
  { id: "r2", name: "Living Rm", brightness: 60, on: true },
  { id: "r3", name: "Bedroom", brightness: 20, on: false },
]

function LightsWidget() {
  const [rooms, setRooms] = useState<Room[]>(INITIAL_ROOMS)
  const [activeScene, setActiveScene] = useState<string | null>("Focus")

  const setRoomBrightness = (id: string, brightness: number) => {
    setRooms((prev) =>
      prev.map((r) => (r.id === id ? { ...r, brightness, on: brightness > 0 } : r))
    )
    setActiveScene(null)
  }

  const toggleRoom = (id: string) => {
    setRooms((prev) =>
      prev.map((r) => (r.id === id ? { ...r, on: !r.on } : r))
    )
    setActiveScene(null)
  }

  const applyScene = (scene: string) => {
    setActiveScene(scene)
    const presets: Record<string, number[]> = {
      Ambient: [55, 55, 40],
      Focus:   [100, 80, 0],
      Movie:   [15, 20, 0],
      Night:   [10, 0, 15],
      Off:     [0, 0, 0],
    }
    const vals = presets[scene] ?? [50, 50, 50]
    setRooms((prev) =>
      prev.map((r, i) => ({ ...r, brightness: vals[i], on: vals[i] > 0 }))
    )
  }

  // Amber tint for lights color
  const LIGHT_COLOR = "oklch(0.78 0.15 75)"

  return (
    <div className="widget-card">
      <div className="widget-header">
        <Lightbulb className="h-3.5 w-3.5 text-[oklch(0.78_0.15_75)]" />
        <span>Lights</span>
      </div>

      {/* Rooms */}
      <div className="mb-3 flex flex-col gap-2.5">
        {rooms.map((room) => (
          <div key={room.id} className="flex flex-col gap-1">
            <div className="flex items-center justify-between">
              <button
                onClick={() => toggleRoom(room.id)}
                className={cn(
                  "text-left text-xs font-medium transition-colors",
                  room.on ? "text-foreground" : "text-muted-foreground"
                )}
              >
                {room.name}
              </button>
              <span className="font-mono text-[10px] text-muted-foreground">
                {room.on ? `${room.brightness}%` : "off"}
              </span>
            </div>
            <Slider
              value={room.on ? room.brightness : 0}
              onChange={(v) => setRoomBrightness(room.id, v)}
              accentColor={room.on ? LIGHT_COLOR : "var(--secondary)"}
            />
          </div>
        ))}
      </div>

      {/* Scene presets */}
      <div className="flex flex-wrap gap-1">
        {SCENES.map((s) => (
          <button
            key={s}
            onClick={() => applyScene(s)}
            className={cn(
              "rounded-md border px-2 py-0.5 text-[10px] font-medium transition-all",
              activeScene === s
                ? "border-[oklch(0.78_0.15_75)] bg-[oklch(0.78_0.15_75)]/15 text-[oklch(0.78_0.15_75)]"
                : "border-border text-muted-foreground hover:border-[oklch(0.78_0.15_75)]/50 hover:text-foreground"
            )}
          >
            {s}
          </button>
        ))}
      </div>
    </div>
  )
}

// ─── AC Widget ────────────────────────────────────────────────────────────────

const AC_MODES: { id: AcMode; label: string; icon: React.ReactNode }[] = [
  { id: "cool", label: "Cool", icon: <Droplets className="h-3 w-3" /> },
  { id: "heat", label: "Heat", icon: <Thermometer className="h-3 w-3" /> },
  { id: "fan",  label: "Fan",  icon: <Wind className="h-3 w-3" /> },
  { id: "auto", label: "Auto", icon: <Wind className="h-3 w-3" /> },
]

const MODE_COLORS: Record<AcMode, string> = {
  cool: "oklch(0.6 0.18 220)",
  heat: "oklch(0.65 0.2 35)",
  fan:  "var(--primary)",
  auto: "oklch(0.6 0.15 280)",
}

function AcWidget() {
  const [on, setOn] = useState(true)
  const [temp, setTemp] = useState(22)
  const [mode, setMode] = useState<AcMode>("cool")
  const [fanSpeed, setFanSpeed] = useState(2) // 1-4
  const fanLabels = ["Low", "Med", "High", "Max"]

  const modeColor = MODE_COLORS[mode]

  // Arc path for temperature dial
  const ARC_R = 36
  const ARC_CX = 52
  const ARC_CY = 52
  const minTemp = 16
  const maxTemp = 30
  const pct = (temp - minTemp) / (maxTemp - minTemp)
  const startAngle = -220 * (Math.PI / 180)
  const endAngle = 40 * (Math.PI / 180)
  const angle = startAngle + pct * (endAngle - startAngle)
  const arcSweep = endAngle - startAngle
  // Full track arc
  const tx1 = ARC_CX + ARC_R * Math.cos(startAngle)
  const ty1 = ARC_CY + ARC_R * Math.sin(startAngle)
  const tx2 = ARC_CX + ARC_R * Math.cos(endAngle)
  const ty2 = ARC_CY + ARC_R * Math.sin(endAngle)
  // Filled arc
  const fx1 = ARC_CX + ARC_R * Math.cos(startAngle)
  const fy1 = ARC_CY + ARC_R * Math.sin(startAngle)
  const fx2 = ARC_CX + ARC_R * Math.cos(angle)
  const fy2 = ARC_CY + ARC_R * Math.sin(angle)
  const largeArc = pct > 0.5 ? 1 : 0

  return (
    <div className="widget-card">
      <div className="widget-header">
        <Thermometer className="h-3.5 w-3.5" style={{ color: modeColor }} />
        <span>Climate</span>
        <button
          onClick={() => setOn((o) => !o)}
          className={cn(
            "ml-auto rounded-full px-2 py-0.5 text-[10px] font-medium transition-all",
            on
              ? "bg-primary/15 text-primary"
              : "bg-secondary text-muted-foreground"
          )}
        >
          {on ? "ON" : "OFF"}
        </button>
      </div>

      {/* Dial */}
      <div className={cn("relative flex justify-center transition-opacity", !on && "opacity-30 pointer-events-none")}>
        <div className="relative">
          <svg width="104" height="80" viewBox="0 0 104 80">
            {/* Track */}
            <path
              d={`M ${tx1} ${ty1} A ${ARC_R} ${ARC_R} 0 1 1 ${tx2} ${ty2}`}
              fill="none"
              stroke="var(--secondary)"
              strokeWidth="5"
              strokeLinecap="round"
            />
            {/* Filled */}
            {pct > 0 && (
              <path
                d={`M ${fx1} ${fy1} A ${ARC_R} ${ARC_R} 0 ${largeArc} 1 ${fx2} ${fy2}`}
                fill="none"
                stroke={modeColor}
                strokeWidth="5"
                strokeLinecap="round"
                style={{ transition: "d 0.15s ease" }}
              />
            )}
            {/* Thumb */}
            <circle cx={fx2} cy={fy2} r="6" fill={modeColor} />
          </svg>
          {/* Center temp display */}
          <div className="absolute inset-0 flex flex-col items-center justify-center pb-2">
            <button
              onClick={() => setTemp((t) => Math.max(minTemp, t - 1))}
              className="text-[16px] leading-none text-muted-foreground hover:text-foreground"
            >−</button>
            <span className="font-mono text-xl font-bold leading-tight" style={{ color: modeColor }}>
              {temp}°
            </span>
            <button
              onClick={() => setTemp((t) => Math.min(maxTemp, t + 1))}
              className="text-[16px] leading-none text-muted-foreground hover:text-foreground"
            >+</button>
          </div>
        </div>
      </div>

      {/* Mode pills */}
      <div className={cn("mb-3 flex gap-1", !on && "opacity-30 pointer-events-none")}>
        {AC_MODES.map((m) => (
          <button
            key={m.id}
            onClick={() => setMode(m.id)}
            className={cn(
              "flex flex-1 items-center justify-center gap-1 rounded-md border py-1 text-[10px] font-medium transition-all",
              mode === m.id
                ? "border-[var(--mode-c)] bg-[var(--mode-c)]/15 text-[var(--mode-c)]"
                : "border-border text-muted-foreground hover:border-border/80 hover:text-foreground"
            )}
            style={{ "--mode-c": modeColor } as React.CSSProperties}
          >
            {m.icon}
            {m.label}
          </button>
        ))}
      </div>

      {/* Fan speed */}
      <div className={cn("flex items-center gap-2", !on && "opacity-30 pointer-events-none")}>
        <Wind className="h-3 w-3 flex-shrink-0 text-muted-foreground" />
        <div className="flex flex-1 gap-1">
          {[1, 2, 3, 4].map((s) => (
            <button
              key={s}
              onClick={() => setFanSpeed(s)}
              className={cn(
                "h-1.5 flex-1 rounded-full transition-all",
                s <= fanSpeed ? "opacity-100" : "opacity-25"
              )}
              style={{ background: s <= fanSpeed ? modeColor : "var(--secondary)" }}
              title={fanLabels[s - 1]}
            />
          ))}
        </div>
        <span className="w-8 text-right font-mono text-[10px] text-muted-foreground">
          {fanLabels[fanSpeed - 1]}
        </span>
      </div>
    </div>
  )
}

// ─── Widget Panel ─────────────────────────────────────────────────────────────

interface WidgetPanelProps {
  visible: boolean
  onToggle: () => void
}

export function WidgetPanel({ visible, onToggle }: WidgetPanelProps) {
  return (
    <>
      {/* Toggle tab */}
      <button
        onClick={onToggle}
        className={cn(
          "absolute right-0 top-1/2 z-40 flex h-16 w-5 -translate-y-1/2 flex-col items-center justify-center rounded-l-md border border-r-0 border-border bg-[var(--window-bg)]/80 backdrop-blur-md transition-all hover:bg-secondary",
          visible && "right-[220px]"
        )}
        title={visible ? "Hide widgets" : "Show widgets"}
      >
        <ChevronRight
          className={cn(
            "h-3 w-3 text-muted-foreground transition-transform duration-300",
            visible ? "rotate-0" : "rotate-180"
          )}
        />
      </button>

      {/* Panel */}
      <div
        className={cn(
          "absolute right-0 top-10 bottom-16 z-30 flex w-[220px] flex-col gap-2 overflow-y-auto overflow-x-hidden p-2 transition-transform duration-300 ease-out",
          visible ? "translate-x-0" : "translate-x-full"
        )}
        style={{ scrollbarWidth: "none" }}
      >
        <MusicWidget />
        <LightsWidget />
        <AcWidget />
      </div>
    </>
  )
}
