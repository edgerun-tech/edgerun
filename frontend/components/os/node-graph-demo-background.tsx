"use client"

import { useMemo, useRef, useState, type PointerEvent } from "react"
import { Bot, Cloud, Cpu, Database, GitBranch, ShieldCheck, UserRound } from "lucide-react"
import { cn } from "@/lib/utils"

type DemoNode = {
  id: string
  label: string
  sublabel: string
  x: number
  y: number
  tone: "cyan" | "green" | "amber" | "rose" | "violet" | "blue" | "slate"
}

type DemoEdge = {
  id: string
  from: string
  to: string
  label: string
}

const CANVAS_WIDTH = 1000
const CANVAS_HEIGHT = 560

const INITIAL_NODES: DemoNode[] = [
  { id: "identity", label: "Identity", sublabel: "sealed keys", x: 170, y: 260, tone: "cyan" },
  { id: "policy", label: "Policy", sublabel: "capability gate", x: 355, y: 140, tone: "green" },
  { id: "codex", label: "Codex", sublabel: "agent loop", x: 522, y: 285, tone: "violet" },
  { id: "runtime", label: "Runtime", sublabel: "WASI host", x: 700, y: 170, tone: "blue" },
  { id: "storage", label: "Storage", sublabel: "private index", x: 755, y: 380, tone: "amber" },
  { id: "apps", label: "Apps", sublabel: "launch surface", x: 470, y: 430, tone: "rose" },
  { id: "node", label: "Edge Node", sublabel: "local bridge", x: 870, y: 280, tone: "slate" },
]

const EDGES: DemoEdge[] = [
  { id: "identity-policy", from: "identity", to: "policy", label: "grant" },
  { id: "policy-codex", from: "policy", to: "codex", label: "approve" },
  { id: "codex-runtime", from: "codex", to: "runtime", label: "plan" },
  { id: "runtime-node", from: "runtime", to: "node", label: "execute" },
  { id: "node-storage", from: "node", to: "storage", label: "sync" },
  { id: "storage-apps", from: "storage", to: "apps", label: "hydrate" },
  { id: "apps-identity", from: "apps", to: "identity", label: "request" },
  { id: "codex-apps", from: "codex", to: "apps", label: "open" },
]

const TONE_CLASSES: Record<DemoNode["tone"], string> = {
  amber: "border-amber-200/20 bg-amber-300/10 text-amber-100",
  blue: "border-sky-200/20 bg-sky-300/10 text-sky-100",
  cyan: "border-cyan-200/20 bg-cyan-300/10 text-cyan-100",
  green: "border-emerald-200/20 bg-emerald-300/10 text-emerald-100",
  rose: "border-rose-200/20 bg-rose-300/10 text-rose-100",
  slate: "border-zinc-200/20 bg-zinc-300/10 text-zinc-100",
  violet: "border-violet-200/20 bg-violet-300/10 text-violet-100",
}

const TONE_ICON_CLASSES: Record<DemoNode["tone"], string> = {
  amber: "text-amber-200",
  blue: "text-sky-200",
  cyan: "text-cyan-200",
  green: "text-emerald-200",
  rose: "text-rose-200",
  slate: "text-zinc-200",
  violet: "text-violet-200",
}

function iconFor(id: string) {
  if (id === "identity") return UserRound
  if (id === "policy") return ShieldCheck
  if (id === "codex") return Bot
  if (id === "runtime") return Cpu
  if (id === "storage") return Database
  if (id === "apps") return GitBranch
  return Cloud
}

function edgePath(from: DemoNode, to: DemoNode) {
  const dx = Math.abs(to.x - from.x)
  const curve = Math.max(80, dx * 0.42)
  return `M ${from.x} ${from.y} C ${from.x + curve} ${from.y}, ${to.x - curve} ${to.y}, ${to.x} ${to.y}`
}

export function NodeGraphDemoBackground() {
  const containerRef = useRef<HTMLDivElement>(null)
  const [nodes, setNodes] = useState(INITIAL_NODES)
  const [draggedId, setDraggedId] = useState<string | null>(null)
  const nodeMap = useMemo(() => new Map(nodes.map((node) => [node.id, node])), [nodes])

  function pointerToCanvas(event: PointerEvent) {
    const rect = containerRef.current?.getBoundingClientRect()
    if (!rect) return null
    return {
      x: ((event.clientX - rect.left) / rect.width) * CANVAS_WIDTH,
      y: ((event.clientY - rect.top) / rect.height) * CANVAS_HEIGHT,
    }
  }

  function moveDraggedNode(event: PointerEvent) {
    if (!draggedId) return
    const point = pointerToCanvas(event)
    if (!point) return
    setNodes((current) => current.map((node) => (
      node.id === draggedId
        ? {
          ...node,
          x: Math.min(CANVAS_WIDTH - 80, Math.max(80, point.x)),
          y: Math.min(CANVAS_HEIGHT - 70, Math.max(70, point.y)),
        }
        : node
    )))
  }

  return (
    <div
      ref={containerRef}
      className="absolute inset-0 overflow-hidden bg-zinc-950 text-white"
      onPointerMove={moveDraggedNode}
      onPointerUp={() => setDraggedId(null)}
      onPointerCancel={() => setDraggedId(null)}
    >
      <svg className="absolute inset-0 h-full w-full" viewBox={`0 0 ${CANVAS_WIDTH} ${CANVAS_HEIGHT}`} aria-hidden="true">
        {EDGES.map((edge, index) => {
          const from = nodeMap.get(edge.from)
          const to = nodeMap.get(edge.to)
          if (!from || !to) return null
          const path = edgePath(from, to)

          return (
            <g key={edge.id}>
              <path d={path} fill="none" stroke="rgba(148,163,184,0.28)" strokeWidth="1.2" />
              <circle r="3" fill={index % 3 === 0 ? "#67e8f9" : index % 3 === 1 ? "#a7f3d0" : "#fcd34d"} opacity="0.72">
                <animateMotion dur={`${4.4 + index * 0.22}s`} repeatCount="indefinite" path={path} />
              </circle>
              <text x={(from.x + to.x) / 2} y={(from.y + to.y) / 2 - 7} className="fill-zinc-500 text-[10px]">
                {edge.label}
              </text>
            </g>
          )
        })}
      </svg>

      {nodes.map((node) => {
        const Icon = iconFor(node.id)
        return (
          <button
            key={node.id}
            type="button"
            onPointerDown={(event) => {
              event.currentTarget.setPointerCapture(event.pointerId)
              setDraggedId(node.id)
            }}
            className={cn(
              "absolute flex min-w-32 cursor-grab items-center gap-2 rounded-lg border px-3 py-2 text-left shadow-lg backdrop-blur-sm transition-transform active:cursor-grabbing",
              draggedId === node.id ? "scale-105" : "hover:scale-[1.02]",
              TONE_CLASSES[node.tone],
            )}
            style={{
              left: `${(node.x / CANVAS_WIDTH) * 100}%`,
              top: `${(node.y / CANVAS_HEIGHT) * 100}%`,
              transform: "translate(-50%, -50%)",
            }}
          >
            <Icon className={cn("h-4 w-4 shrink-0", TONE_ICON_CLASSES[node.tone])} />
            <span className="min-w-0">
              <span className="block truncate text-xs font-semibold">{node.label}</span>
              <span className="block truncate font-mono text-[10px] opacity-65">{node.sublabel}</span>
            </span>
          </button>
        )
      })}

      <div className="pointer-events-none absolute inset-0 bg-transparent" />
    </div>
  )
}
