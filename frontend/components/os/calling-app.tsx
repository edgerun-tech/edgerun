"use client"

import { useState, useEffect, useRef } from "react"
import { Phone, PhoneOff, Mic, MicOff, Volume2, VolumeX, Video, VideoOff, Signal } from "lucide-react"
import { cn } from "@/lib/utils"

type CallState = "idle" | "dialing" | "ringing" | "connected" | "ended"

interface Peer {
  id: string
  name: string
  handle: string
  nodeId: string
}

const DEMO_PEERS: Peer[] = [
  { id: "1", name: "Ara Nakamura", handle: "@ara.run", nodeId: "ed3f:a1b2" },
  { id: "2", name: "Elias Voss", handle: "@voss.edge", nodeId: "ed3f:c3d4" },
  { id: "3", name: "Priya Mehta", handle: "@priya.m", nodeId: "ed3f:i9j0" },
  { id: "4", name: "Mila Dube", handle: "@mila", nodeId: "ed3f:m3n4" },
]

function Avatar({ name, pulse }: { name: string; pulse?: boolean }) {
  const initials = name.split(" ").map((n) => n[0]).join("").slice(0, 2).toUpperCase()
  const hue = name.split("").reduce((acc, c) => acc + c.charCodeAt(0), 0) % 360
  return (
    <div className="relative flex items-center justify-center">
      {pulse && (
        <>
          <div
            className="absolute h-28 w-28 animate-ping rounded-full opacity-20"
            style={{ background: `oklch(0.65 0.2 145)` }}
          />
          <div
            className="absolute h-24 w-24 animate-ping rounded-full opacity-10"
            style={{ animationDelay: "0.3s", background: `oklch(0.65 0.2 145)` }}
          />
        </>
      )}
      <div
        className="relative flex h-20 w-20 items-center justify-center rounded-full text-2xl font-bold"
        style={{ background: `oklch(0.25 0.1 ${hue})`, color: `oklch(0.85 0.1 ${hue})` }}
      >
        {initials}
      </div>
    </div>
  )
}

function SignalBars({ quality }: { quality: number }) {
  return (
    <div className="flex items-end gap-0.5">
      {[1, 2, 3, 4].map((bar) => (
        <div
          key={bar}
          className={cn(
            "w-1 rounded-sm transition-colors",
            bar <= quality ? "bg-[var(--status-online)]" : "bg-muted-foreground/30"
          )}
          style={{ height: `${bar * 4 + 2}px` }}
        />
      ))}
    </div>
  )
}

interface CallingAppProps {
  initialPeer?: Peer
}

export function CallingApp({ initialPeer }: CallingAppProps) {
  const [callState, setCallState] = useState<CallState>("idle")
  const [activePeer, setActivePeer] = useState<Peer | null>(initialPeer || null)
  const [elapsed, setElapsed] = useState(0)
  const [isMuted, setIsMuted] = useState(false)
  const [isSpeakerOff, setIsSpeakerOff] = useState(false)
  const [isVideoOn, setIsVideoOn] = useState(false)
  const [quality, setQuality] = useState(4)
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null)

  // Auto-connect the initial peer if passed in
  useEffect(() => {
    if (initialPeer) {
      handleCall(initialPeer)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const handleCall = (peer: Peer) => {
    setActivePeer(peer)
    setCallState("dialing")
    setElapsed(0)

    // Simulate ringing then connecting
    setTimeout(() => setCallState("ringing"), 800)
    setTimeout(() => {
      setCallState("connected")
      timerRef.current = setInterval(() => setElapsed((e) => e + 1), 1000)
      setQuality(Math.floor(Math.random() * 2) + 3) // 3 or 4 bars
    }, 2800)
  }

  const handleHangUp = () => {
    if (timerRef.current) clearInterval(timerRef.current)
    setCallState("ended")
    setTimeout(() => {
      setCallState("idle")
      setActivePeer(null)
      setElapsed(0)
      setIsMuted(false)
      setIsSpeakerOff(false)
      setIsVideoOn(false)
    }, 1200)
  }

  useEffect(() => () => { if (timerRef.current) clearInterval(timerRef.current) }, [])

  const formatTime = (s: number) => {
    const m = Math.floor(s / 60)
    return `${String(m).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`
  }

  const isInCall = callState === "connected" || callState === "dialing" || callState === "ringing"

  return (
    <div className="flex h-full flex-col">
      {/* Active call / idle panel */}
      <div className={cn(
        "flex flex-col items-center justify-center gap-5 transition-all",
        isInCall || callState === "ended" ? "flex-1 py-10" : "py-8"
      )}>
        {isInCall || callState === "ended" ? (
          <>
            <Avatar name={activePeer!.name} pulse={callState === "ringing"} />
            <div className="text-center">
              <h2 className="text-xl font-semibold text-foreground">{activePeer!.name}</h2>
              <p className="text-sm text-muted-foreground">{activePeer!.handle}</p>
              <p className="mt-1 font-mono text-[10px] text-muted-foreground/60">{activePeer!.nodeId}</p>
            </div>

            {/* Status */}
            <div className="flex items-center gap-2">
              {callState === "dialing" && (
                <p className="text-sm text-muted-foreground">Establishing P2P channel...</p>
              )}
              {callState === "ringing" && (
                <p className="text-sm text-[var(--status-warning)]">Ringing...</p>
              )}
              {callState === "connected" && (
                <div className="flex items-center gap-2">
                  <SignalBars quality={quality} />
                  <p className="font-mono text-sm text-[var(--status-online)]">{formatTime(elapsed)}</p>
                </div>
              )}
              {callState === "ended" && (
                <p className="text-sm text-muted-foreground">Call ended</p>
              )}
            </div>

            {/* In-call controls */}
            {callState === "connected" && (
              <div className="flex items-center gap-3">
                <button
                  onClick={() => setIsMuted((v) => !v)}
                  className={cn(
                    "flex h-11 w-11 items-center justify-center rounded-full transition-colors",
                    isMuted ? "bg-[var(--status-error)]/20 text-[var(--status-error)]" : "bg-secondary text-foreground hover:bg-secondary/70"
                  )}
                >
                  {isMuted ? <MicOff className="h-4 w-4" /> : <Mic className="h-4 w-4" />}
                </button>
                <button
                  onClick={() => setIsSpeakerOff((v) => !v)}
                  className={cn(
                    "flex h-11 w-11 items-center justify-center rounded-full transition-colors",
                    isSpeakerOff ? "bg-[var(--status-error)]/20 text-[var(--status-error)]" : "bg-secondary text-foreground hover:bg-secondary/70"
                  )}
                >
                  {isSpeakerOff ? <VolumeX className="h-4 w-4" /> : <Volume2 className="h-4 w-4" />}
                </button>
                <button
                  onClick={() => setIsVideoOn((v) => !v)}
                  className={cn(
                    "flex h-11 w-11 items-center justify-center rounded-full transition-colors",
                    isVideoOn ? "bg-primary/20 text-primary" : "bg-secondary text-foreground hover:bg-secondary/70"
                  )}
                >
                  {isVideoOn ? <Video className="h-4 w-4" /> : <VideoOff className="h-4 w-4" />}
                </button>
              </div>
            )}

            {/* Hang up */}
            {isInCall && (
              <button
                onClick={handleHangUp}
                className="flex h-14 w-14 items-center justify-center rounded-full bg-[var(--status-error)] text-white shadow-lg transition-transform hover:scale-105 active:scale-95"
              >
                <PhoneOff className="h-6 w-6" />
              </button>
            )}
          </>
        ) : (
          <div className="text-center">
            <div className="mx-auto mb-3 flex h-14 w-14 items-center justify-center rounded-full bg-secondary">
              <Phone className="h-6 w-6 text-muted-foreground" />
            </div>
            <p className="text-sm font-medium text-foreground">P2P Voice</p>
            <p className="mt-1 text-xs text-muted-foreground">End-to-end encrypted via Edgerun</p>
          </div>
        )}
      </div>

      {/* Contact list — only shown when idle */}
      {!isInCall && callState !== "ended" && (
        <div className="flex-1 overflow-y-auto border-t border-[var(--window-border)]">
          <p className="px-4 pb-2 pt-3 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">Online peers</p>
          {DEMO_PEERS.map((peer) => (
            <button
              key={peer.id}
              onClick={() => handleCall(peer)}
              className="flex w-full items-center gap-3 px-4 py-2.5 text-left transition-colors hover:bg-secondary/50"
            >
              <div
                className="flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-full text-xs font-bold"
                style={{
                  background: `oklch(0.25 0.1 ${peer.name.split("").reduce((a, c) => a + c.charCodeAt(0), 0) % 360})`,
                  color: `oklch(0.85 0.1 ${peer.name.split("").reduce((a, c) => a + c.charCodeAt(0), 0) % 360})`,
                }}
              >
                {peer.name.split(" ").map((n) => n[0]).join("").slice(0, 2).toUpperCase()}
              </div>
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium text-foreground">{peer.name}</p>
                <p className="truncate text-xs text-muted-foreground">{peer.handle}</p>
              </div>
              <div className="flex items-center gap-2">
                <Signal className="h-3.5 w-3.5 text-[var(--status-online)]" />
                <Phone className="h-4 w-4 text-muted-foreground transition-colors hover:text-[var(--status-online)]" />
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
