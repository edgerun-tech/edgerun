"use client"

import { useEffect, useRef, useCallback } from "react"

// Seeded pseudo-random for stable node positions
function seededRandom(seed: number) {
  const x = Math.sin(seed + 1) * 10000
  return x - Math.floor(x)
}

export interface GlobeNode {
  id: string
  lat: number  // degrees, -90 to 90
  lng: number  // degrees, -180 to 180
  isUser?: boolean
  isActive?: boolean
}

interface GlobeProps {
  nodeCount?: number
  className?: string
}

// Convert lat/lng to 3D unit sphere coords
function latLngToXYZ(lat: number, lng: number): [number, number, number] {
  const phi = (90 - lat) * (Math.PI / 180)
  const theta = (lng + 180) * (Math.PI / 180)
  return [
    -Math.sin(phi) * Math.cos(theta),
    Math.cos(phi),
    Math.sin(phi) * Math.sin(theta),
  ]
}

// Rotate a 3D point around Y axis
function rotateY(x: number, y: number, z: number, angle: number): [number, number, number] {
  const cos = Math.cos(angle)
  const sin = Math.sin(angle)
  return [x * cos + z * sin, y, -x * sin + z * cos]
}

// Rotate a 3D point around X axis (tilt)
function rotateX(x: number, y: number, z: number, angle: number): [number, number, number] {
  const cos = Math.cos(angle)
  const sin = Math.sin(angle)
  return [x, y * cos - z * sin, y * sin + z * cos]
}

// Orthographic project to 2D
function project(x: number, y: number, _z: number, cx: number, cy: number, r: number): [number, number] {
  return [cx + x * r, cy - y * r]
}

// Interpolate great-circle arc between two lat/lng points
function greatCirclePoints(
  lat1: number, lng1: number,
  lat2: number, lng2: number,
  steps: number
): Array<[number, number, number]> {
  const [x1, y1, z1] = latLngToXYZ(lat1, lng1)
  const [x2, y2, z2] = latLngToXYZ(lat2, lng2)
  const pts: Array<[number, number, number]> = []
  for (let i = 0; i <= steps; i++) {
    const t = i / steps
    // Slerp between the two vectors
    let bx = x1 + (x2 - x1) * t
    let by = y1 + (y2 - y1) * t
    let bz = z1 + (z2 - z1) * t
    const len = Math.sqrt(bx * bx + by * by + bz * bz)
    bx /= len; by /= len; bz /= len
    pts.push([bx, by, bz])
  }
  return pts
}

// Generate stable fake nodes
function generateNodes(count: number): GlobeNode[] {
  const nodes: GlobeNode[] = []
  for (let i = 0; i < count; i++) {
    nodes.push({
      id: `node-${i}`,
      lat: (seededRandom(i * 3) * 160) - 80,
      lng: (seededRandom(i * 3 + 1) * 360) - 180,
      isActive: seededRandom(i * 3 + 2) > 0.3,
    })
  }
  // User node at a reasonable location
  nodes.push({
    id: "user",
    lat: 37.7,
    lng: -122.4,
    isUser: true,
    isActive: true,
  })
  return nodes
}

// Connection pairs — user node connects to several, nodes connect amongst themselves
function generateConnections(nodes: GlobeNode[]): Array<[number, number]> {
  const pairs: Array<[number, number]> = []
  const userIdx = nodes.findIndex((n) => n.isUser)
  const activeIdxs = nodes.map((n, i) => ({ n, i })).filter(({ n }) => n.isActive && !n.isUser)

  // User → 5 nearest active nodes
  activeIdxs.slice(0, 5).forEach(({ i }) => pairs.push([userIdx, i]))

  // Inter-node connections
  for (let i = 0; i < activeIdxs.length - 1; i += 3) {
    if (activeIdxs[i + 1]) pairs.push([activeIdxs[i].i, activeIdxs[i + 1].i])
    if (activeIdxs[i + 2]) pairs.push([activeIdxs[i].i, activeIdxs[i + 2].i])
  }

  return pairs
}

export function Globe({ nodeCount = 24, className }: GlobeProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const animRef = useRef<number>(0)
  const rotationRef = useRef(0)
  const nodesRef = useRef(generateNodes(nodeCount))
  const connectionsRef = useRef(generateConnections(nodesRef.current))
  // Arc animation: each connection has an animated "t" progress 0→1→fade
  const arcProgressRef = useRef<number[]>(connectionsRef.current.map(() => Math.random()))

  const draw = useCallback(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const ctx = canvas.getContext("2d")
    if (!ctx) return

    const W = canvas.width
    const H = canvas.height
    const cx = W / 2
    const cy = H / 2
    const r = Math.min(W, H) * 0.38
    const tilt = 0.36  // ~20 deg X tilt

    ctx.clearRect(0, 0, W, H)

    const angle = rotationRef.current

    // Colors (matching CSS tokens)
    const primaryRgb = "101, 212, 141"   // approx oklch(0.65 0.2 145) → green
    const mutedRgb = "80, 80, 95"

    // ---- Draw lat/lng grid lines ----
    const latLines = [-60, -40, -20, 0, 20, 40, 60]
    const lngLines = Array.from({ length: 12 }, (_, i) => i * 30 - 180)
    const STEPS = 80

    // Latitude lines
    for (const lat of latLines) {
      ctx.beginPath()
      let first = true
      for (let li = 0; li <= STEPS; li++) {
        const lng = (li / STEPS) * 360 - 180
        let [x, y, z] = latLngToXYZ(lat, lng)
        ;[x, y, z] = rotateX(x, y, z, tilt)
        ;[x, y, z] = rotateY(x, y, z, angle)
        const visible = z > -0.05
        const [px, py] = project(x, y, z, cx, cy, r)
        const alpha = Math.max(0, Math.min(1, (z + 0.1) * 2))
        if (first || !visible) {
          ctx.moveTo(px, py)
          first = false
        } else {
          ctx.lineTo(px, py)
        }
        if (!visible) first = true
      }
      ctx.strokeStyle = `rgba(${mutedRgb}, 0.12)`
      ctx.lineWidth = 0.5
      ctx.stroke()
    }

    // Longitude lines
    for (const lng of lngLines) {
      ctx.beginPath()
      let first = true
      for (let li = 0; li <= STEPS; li++) {
        const lat = (li / STEPS) * 180 - 90
        let [x, y, z] = latLngToXYZ(lat, lng)
        ;[x, y, z] = rotateX(x, y, z, tilt)
        ;[x, y, z] = rotateY(x, y, z, angle)
        const visible = z > -0.05
        const [px, py] = project(x, y, z, cx, cy, r)
        if (first || !visible) {
          ctx.moveTo(px, py)
          first = false
        } else {
          ctx.lineTo(px, py)
        }
        if (!visible) first = true
      }
      ctx.strokeStyle = `rgba(${mutedRgb}, 0.1)`
      ctx.lineWidth = 0.5
      ctx.stroke()
    }

    // ---- Draw arc connections ----
    const connections = connectionsRef.current
    const arcProgress = arcProgressRef.current

    connections.forEach((pair, ci) => {
      const [ai, bi] = pair
      const nodeA = nodesRef.current[ai]
      const nodeB = nodesRef.current[bi]
      if (!nodeA || !nodeB) return

      const progress = arcProgress[ci]
      const arcSteps = 40
      const drawUpTo = Math.floor(progress * arcSteps)

      const pts = greatCirclePoints(nodeA.lat, nodeA.lng, nodeB.lat, nodeB.lng, arcSteps)

      let prevVisible = false
      let prevPx = 0, prevPy = 0
      const isUserConn = nodeA.isUser || nodeB.isUser

      for (let pi = 0; pi <= drawUpTo && pi < pts.length; pi++) {
        let [x, y, z] = pts[pi]
        ;[x, y, z] = rotateX(x, y, z, tilt)
        ;[x, y, z] = rotateY(x, y, z, angle)
        const visible = z > 0
        const [px, py] = project(x, y, z, cx, cy, r)
        const frac = pi / drawUpTo
        const alpha = Math.max(0, z) * (isUserConn ? 0.7 : 0.3) * (1 - Math.pow(Math.abs(frac - 0.5) * 2, 2) * 0.4)

        if (pi > 0 && visible && prevVisible) {
          ctx.beginPath()
          ctx.moveTo(prevPx, prevPy)
          ctx.lineTo(px, py)
          ctx.strokeStyle = `rgba(${primaryRgb}, ${alpha})`
          ctx.lineWidth = isUserConn ? 1.2 : 0.7
          ctx.stroke()
        }
        prevVisible = visible
        prevPx = px
        prevPy = py
      }
    })

    // Advance arc progress
    arcProgressRef.current = arcProgress.map((p, i) => {
      const speed = connections[i] && nodesRef.current[connections[i][0]]?.isUser ? 0.006 : 0.003
      return p >= 1.05 ? 0 : p + speed
    })

    // ---- Draw nodes ----
    nodesRef.current.forEach((node) => {
      let [x, y, z] = latLngToXYZ(node.lat, node.lng)
      ;[x, y, z] = rotateX(x, y, z, tilt)
      ;[x, y, z] = rotateY(x, y, z, angle)
      if (z < 0) return  // behind globe

      const [px, py] = project(x, y, z, cx, cy, r)
      const alpha = Math.max(0, Math.min(1, (z + 0.1) * 3))

      if (node.isUser) {
        // User node: bright, pulsing ring
        const t = Date.now() / 600
        const pulseR = 5 + Math.sin(t) * 2.5
        ctx.beginPath()
        ctx.arc(px, py, pulseR, 0, Math.PI * 2)
        ctx.strokeStyle = `rgba(${primaryRgb}, ${0.25 * alpha})`
        ctx.lineWidth = 1.5
        ctx.stroke()

        ctx.beginPath()
        ctx.arc(px, py, 3.5, 0, Math.PI * 2)
        ctx.fillStyle = `rgba(${primaryRgb}, ${alpha})`
        ctx.fill()
      } else if (node.isActive) {
        ctx.beginPath()
        ctx.arc(px, py, 2, 0, Math.PI * 2)
        ctx.fillStyle = `rgba(${primaryRgb}, ${0.75 * alpha})`
        ctx.fill()
      } else {
        ctx.beginPath()
        ctx.arc(px, py, 1.5, 0, Math.PI * 2)
        ctx.fillStyle = `rgba(${mutedRgb}, ${0.5 * alpha})`
        ctx.fill()
      }
    })

    // Advance rotation
    rotationRef.current += 0.0018

    animRef.current = requestAnimationFrame(draw)
  }, [])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return

    const resize = () => {
      const parent = canvas.parentElement
      if (!parent) return
      canvas.width = parent.clientWidth
      canvas.height = parent.clientHeight
    }
    resize()
    window.addEventListener("resize", resize)

    animRef.current = requestAnimationFrame(draw)

    return () => {
      window.removeEventListener("resize", resize)
      cancelAnimationFrame(animRef.current)
    }
  }, [draw])

  return (
    <canvas
      ref={canvasRef}
      className={className}
      aria-hidden="true"
    />
  )
}
