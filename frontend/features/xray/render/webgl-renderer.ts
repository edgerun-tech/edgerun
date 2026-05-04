import { buildNodeProgram, buildEdgeProgram, screenToGraph } from "./shaders"
import { getNodeColor, getNodeSize, getEdgeColor } from "./color-policy"
import type { XrayNode, XrayEdge, RuntimeNodeStats } from "../graph/types"

const MAX_BUFFERED_NODES = 20000
const MAX_BUFFERED_EDGES = 100000

interface RenderInput {
  nodes: Map<string, XrayNode>
  edges: XrayEdge[]
  runtimeStats: Map<string, RuntimeNodeStats>
  selectedId: string | null
  highlightedIds: Set<string>
  zoom: number
  panX: number
  panY: number
  rotation: number
  runtimeMode: boolean
}

export class WebGLRenderer {
  private gl: WebGL2RenderingContext
  private nodeProgram: WebGLProgram
  private edgeProgram: WebGLProgram
  private nodeBuffers: Record<string, WebGLBuffer>
  private edgeBuffer: WebGLBuffer
  private positions: Float32Array
  private colors: Float32Array
  private sizes: Float32Array
  private sels: Float32Array
  private edgeFloat: Float32Array
  private glowPositions: Float32Array
  private glowColors: Float32Array
  private glowSizes: Float32Array
  private glowSels: Float32Array
  private canvas: HTMLCanvasElement
  private animId = 0

  constructor(canvas: HTMLCanvasElement) {
    const gl = canvas.getContext("webgl2", { alpha: true, antialias: true })
    if (!gl) throw new Error("WebGL2 not supported")
    this.gl = gl
    this.canvas = canvas

    this.nodeProgram = buildNodeProgram(gl)
    this.edgeProgram = buildEdgeProgram(gl)

    this.nodeBuffers = {
      positions: gl.createBuffer()!,
      colors: gl.createBuffer()!,
      sizes: gl.createBuffer()!,
      selected: gl.createBuffer()!,
    }
    this.edgeBuffer = gl.createBuffer()!

    this.positions = new Float32Array(MAX_BUFFERED_NODES * 2)
    this.colors = new Float32Array(MAX_BUFFERED_NODES * 4)
    this.sizes = new Float32Array(MAX_BUFFERED_NODES)
    this.sels = new Float32Array(MAX_BUFFERED_NODES)
    this.edgeFloat = new Float32Array(MAX_BUFFERED_EDGES * 2 * 5)
    this.glowPositions = new Float32Array(MAX_BUFFERED_NODES * 2)
    this.glowColors = new Float32Array(MAX_BUFFERED_NODES * 4)
    this.glowSizes = new Float32Array(MAX_BUFFERED_NODES)
    this.glowSels = new Float32Array(MAX_BUFFERED_NODES)
  }

  resize() {
    const container = this.canvas.parentElement
    if (!container) return
    const w = container.clientWidth
    const h = container.clientHeight
    if (w <= 0 || h <= 0) return
    const dpr = window.devicePixelRatio || 1
    this.canvas.width = w * dpr
    this.canvas.height = h * dpr
    this.canvas.style.width = `${w}px`
    this.canvas.style.height = `${h}px`
    this.gl.viewport(0, 0, this.canvas.width, this.canvas.height)
  }

  render(input: RenderInput) {
    const { nodes, edges, runtimeStats, selectedId, highlightedIds, zoom, panX, panY, rotation, runtimeMode } = input
    const gl = this.gl
    const vw = this.canvas.width
    const vh = this.canvas.height
    const nodeCount = Math.min(nodes.size, MAX_BUFFERED_NODES)
    const hasFocus = selectedId !== null || highlightedIds.size > 0
    const pulse = 0.5 + 0.5 * Math.sin(performance.now() / 360)

    gl.clearColor(0.012, 0.012, 0.025, 1)
    gl.clear(gl.COLOR_BUFFER_BIT)
    gl.enable(gl.BLEND)
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA)

    if (nodeCount === 0) return

    let ni = 0
    let glowCount = 0
    const indexMap = new Map<string, number>()
    for (const [id, node] of nodes) {
      if (ni >= MAX_BUFFERED_NODES) break
      this.positions[ni * 2] = node.x ?? 0
      this.positions[ni * 2 + 1] = node.y ?? 0
      const isSelected = selectedId === id
      const isHighlighted = highlightedIds.has(id)
      const isDimmed = hasFocus && !isSelected && !isHighlighted
      const runtime = runtimeStats.get(id)
      const color = getNodeColor(node.layer, node.language, isHighlighted, isSelected, runtime, runtimeMode)
      const alpha = isDimmed ? color[3] * 0.22 : color[3]
      this.colors[ni * 4] = color[0]
      this.colors[ni * 4 + 1] = color[1]
      this.colors[ni * 4 + 2] = color[2]
      this.colors[ni * 4 + 3] = alpha
      this.sizes[ni] = getNodeSize(0, isHighlighted, isSelected, runtime, runtimeMode) * (isHighlighted || isSelected ? 1 + pulse * 0.16 : 1)
      this.sels[ni] = isSelected ? 1 : 0
      indexMap.set(id, ni)

      if (isHighlighted || isSelected) {
        this.glowPositions[glowCount * 2] = node.x ?? 0
        this.glowPositions[glowCount * 2 + 1] = node.y ?? 0
        this.glowColors[glowCount * 4] = color[0]
        this.glowColors[glowCount * 4 + 1] = color[1]
        this.glowColors[glowCount * 4 + 2] = color[2]
        this.glowColors[glowCount * 4 + 3] = 0.18 + pulse * 0.16
        this.glowSizes[glowCount] = this.sizes[ni] * (2.4 + pulse * 0.8)
        this.glowSels[glowCount] = 0
        glowCount++
      }
      ni++
    }

    let edgeVertCount = 0
    for (const edge of edges) {
      if (edgeVertCount >= MAX_BUFFERED_EDGES * 2) break
      const si = indexMap.get(edge.source)
      const ti = indexMap.get(edge.target)
      if (si === undefined || ti === undefined) continue
      const isHighlighted = highlightedIds.has(edge.source) || highlightedIds.has(edge.target)
      if (hasFocus && !isHighlighted) continue
      const ec = getEdgeColor(edge.kind, isHighlighted)

      const ei = edgeVertCount * 5
      this.edgeFloat[ei] = this.positions[si * 2]
      this.edgeFloat[ei + 1] = this.positions[si * 2 + 1]
      this.edgeFloat[ei + 2] = ec[0]
      this.edgeFloat[ei + 3] = ec[1]
      this.edgeFloat[ei + 4] = ec[2]
      edgeVertCount++

      const ti2 = edgeVertCount * 5
      this.edgeFloat[ti2] = this.positions[ti * 2]
      this.edgeFloat[ti2 + 1] = this.positions[ti * 2 + 1]
      this.edgeFloat[ti2 + 2] = ec[0]
      this.edgeFloat[ti2 + 3] = ec[1]
      this.edgeFloat[ti2 + 4] = ec[2]
      edgeVertCount++
    }

    if (edgeVertCount > 0) {
      this.drawEdges(edgeVertCount, vw, vh, zoom, panX, panY, rotation)
    }

    if (glowCount > 0) {
      gl.blendFunc(gl.SRC_ALPHA, gl.ONE)
      this.drawNodes(glowCount, this.glowPositions, this.glowColors, this.glowSizes, this.glowSels, vw, vh, zoom, panX, panY, rotation)
      gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA)
    }

    this.drawNodes(nodeCount, this.positions, this.colors, this.sizes, this.sels, vw, vh, zoom, panX, panY, rotation)
  }

  private drawEdges(edgeVertCount: number, vw: number, vh: number, zoom: number, panX: number, panY: number, rotation: number) {
    const gl = this.gl
    gl.useProgram(this.edgeProgram)
    for (let i = 0; i < 16; i++) gl.disableVertexAttribArray(i)

    gl.bindBuffer(gl.ARRAY_BUFFER, this.edgeBuffer)
    gl.bufferData(gl.ARRAY_BUFFER, this.edgeFloat.subarray(0, edgeVertCount * 5), gl.DYNAMIC_DRAW)

    const eRes = gl.getUniformLocation(this.edgeProgram, "u_resolution")
    const eZoom = gl.getUniformLocation(this.edgeProgram, "u_zoom")
    const ePan = gl.getUniformLocation(this.edgeProgram, "u_pan")
    const eRot = gl.getUniformLocation(this.edgeProgram, "u_rotation")
    const ePos = gl.getAttribLocation(this.edgeProgram, "a_position")
    const eCol = gl.getAttribLocation(this.edgeProgram, "a_color")

    if (eRes) gl.uniform2f(eRes, vw, vh)
    if (eZoom) gl.uniform1f(eZoom, zoom)
    if (ePan) gl.uniform2f(ePan, panX, panY)
    if (eRot) gl.uniform1f(eRot, rotation)

    if (ePos >= 0 && eCol >= 0) {
      gl.enableVertexAttribArray(ePos)
      gl.enableVertexAttribArray(eCol)
      gl.vertexAttribPointer(ePos, 2, gl.FLOAT, false, 20, 0)
      gl.vertexAttribPointer(eCol, 3, gl.FLOAT, false, 20, 8)
      gl.drawArrays(gl.LINES, 0, edgeVertCount)
    }
  }

  private drawNodes(
    nodeCount: number,
    positions: Float32Array,
    colors: Float32Array,
    sizes: Float32Array,
    selected: Float32Array,
    vw: number,
    vh: number,
    zoom: number,
    panX: number,
    panY: number,
    rotation: number,
  ) {
    const gl = this.gl
    gl.useProgram(this.nodeProgram)
    for (let i = 0; i < 16; i++) gl.disableVertexAttribArray(i)

    gl.bindBuffer(gl.ARRAY_BUFFER, this.nodeBuffers.positions)
    gl.bufferData(gl.ARRAY_BUFFER, positions.subarray(0, nodeCount * 2), gl.DYNAMIC_DRAW)
    const nPos = gl.getAttribLocation(this.nodeProgram, "a_position")
    if (nPos >= 0) {
      gl.enableVertexAttribArray(nPos)
      gl.vertexAttribPointer(nPos, 2, gl.FLOAT, false, 0, 0)
    }

    gl.bindBuffer(gl.ARRAY_BUFFER, this.nodeBuffers.colors)
    gl.bufferData(gl.ARRAY_BUFFER, colors.subarray(0, nodeCount * 4), gl.DYNAMIC_DRAW)
    const nCol = gl.getAttribLocation(this.nodeProgram, "a_color")
    if (nCol >= 0) {
      gl.enableVertexAttribArray(nCol)
      gl.vertexAttribPointer(nCol, 4, gl.FLOAT, false, 0, 0)
    }

    gl.bindBuffer(gl.ARRAY_BUFFER, this.nodeBuffers.sizes)
    gl.bufferData(gl.ARRAY_BUFFER, sizes.subarray(0, nodeCount), gl.DYNAMIC_DRAW)
    const nSize = gl.getAttribLocation(this.nodeProgram, "a_size")
    if (nSize >= 0) {
      gl.enableVertexAttribArray(nSize)
      gl.vertexAttribPointer(nSize, 1, gl.FLOAT, false, 0, 0)
    }

    gl.bindBuffer(gl.ARRAY_BUFFER, this.nodeBuffers.selected)
    gl.bufferData(gl.ARRAY_BUFFER, selected.subarray(0, nodeCount), gl.DYNAMIC_DRAW)
    const nSel = gl.getAttribLocation(this.nodeProgram, "a_selected")
    if (nSel >= 0) {
      gl.enableVertexAttribArray(nSel)
      gl.vertexAttribPointer(nSel, 1, gl.FLOAT, false, 0, 0)
    }

    const nRes = gl.getUniformLocation(this.nodeProgram, "u_resolution")
    const nZoom = gl.getUniformLocation(this.nodeProgram, "u_zoom")
    const nPan = gl.getUniformLocation(this.nodeProgram, "u_pan")
    const nRot = gl.getUniformLocation(this.nodeProgram, "u_rotation")

    if (nRes) gl.uniform2f(nRes, vw, vh)
    if (nZoom) gl.uniform1f(nZoom, zoom)
    if (nPan) gl.uniform2f(nPan, panX, panY)
    if (nRot) gl.uniform1f(nRot, rotation)

    gl.drawArrays(gl.POINTS, 0, nodeCount)
  }

  screenToGraphCoords(sx: number, sy: number, zoom: number, panX: number, panY: number, rotation: number): [number, number] {
    return screenToGraph(sx, sy, zoom, panX, panY, rotation, this.canvas.width, this.canvas.height)
  }

  startLoop(draw: () => void) {
    const loop = () => {
      draw()
      this.animId = requestAnimationFrame(loop)
    }
    this.animId = requestAnimationFrame(loop)
    return () => cancelAnimationFrame(this.animId)
  }

  destroy() {
    cancelAnimationFrame(this.animId)
    const gl = this.gl
    gl.deleteProgram(this.nodeProgram)
    gl.deleteProgram(this.edgeProgram)
    for (const buf of Object.values(this.nodeBuffers)) gl.deleteBuffer(buf)
    gl.deleteBuffer(this.edgeBuffer)
  }
}
