export const NODE_VS = `#version 300 es
layout(location=0) in vec2 a_position;
layout(location=1) in vec4 a_color;
layout(location=2) in float a_size;
layout(location=3) in float a_selected;
uniform vec2 u_resolution;
uniform float u_zoom;
uniform vec2 u_pan;
uniform float u_rotation;
out vec4 v_color;
out float v_selected;
void main() {
    vec2 pos = (a_position + u_pan) * u_zoom;
    float c = cos(u_rotation);
    float s = sin(u_rotation);
    pos = vec2(pos.x * c - pos.y * s, pos.x * s + pos.y * c);
    vec2 clip = pos / (u_resolution * 0.5);
    gl_Position = vec4(clip.x, clip.y, 0.0, 1.0);
    gl_PointSize = a_size * u_zoom;
    v_color = a_color;
    v_selected = a_selected;
}`

export const NODE_FS = `#version 300 es
precision mediump float;
in vec4 v_color;
in float v_selected;
out vec4 fragColor;
void main() {
    vec2 c = gl_PointCoord - 0.5;
    float d = length(c);
    if (d > 0.5) discard;
    vec4 color = v_color;
    if (v_selected > 0.5 && d > 0.4) {
        color = vec4(1.0, 1.0, 1.0, 1.0);
    }
    float alpha = smoothstep(0.5, 0.35, d) * color.a;
    fragColor = vec4(color.rgb * alpha, alpha);
}`

export const EDGE_VS = `#version 300 es
layout(location=0) in vec2 a_position;
layout(location=1) in vec3 a_color;
uniform vec2 u_resolution;
uniform float u_zoom;
uniform vec2 u_pan;
uniform float u_rotation;
out vec3 v_color;
void main() {
    vec2 pos = (a_position + u_pan) * u_zoom;
    float c = cos(u_rotation);
    float s = sin(u_rotation);
    pos = vec2(pos.x * c - pos.y * s, pos.x * s + pos.y * c);
    vec2 clip = pos / (u_resolution * 0.5);
    gl_Position = vec4(clip.x, clip.y, 0.0, 1.0);
    v_color = a_color;
}`

export const EDGE_FS = `#version 300 es
precision mediump float;
in vec3 v_color;
out vec4 fragColor;
void main() {
    fragColor = vec4(v_color, 1.0);
}`

export function compileShader(gl: WebGL2RenderingContext, type: number, source: string): WebGLShader {
  const shader = gl.createShader(type)!
  gl.shaderSource(shader, source)
  gl.compileShader(shader)
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const info = gl.getShaderInfoLog(shader)
    gl.deleteShader(shader)
    throw new Error(`Shader compile: ${info}`)
  }
  return shader
}

export function buildProgram(gl: WebGL2RenderingContext, vs: WebGLShader, fs: WebGLShader): WebGLProgram {
  const prog = gl.createProgram()!
  gl.attachShader(prog, vs)
  gl.attachShader(prog, fs)
  gl.linkProgram(prog)
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
    const info = gl.getProgramInfoLog(prog)
    gl.deleteProgram(prog)
    throw new Error(`Program link: ${info}`)
  }
  return prog
}

export function buildNodeProgram(gl: WebGL2RenderingContext): WebGLProgram {
  const vs = compileShader(gl, gl.VERTEX_SHADER, NODE_VS)
  const fs = compileShader(gl, gl.FRAGMENT_SHADER, NODE_FS)
  return buildProgram(gl, vs, fs)
}

export function buildEdgeProgram(gl: WebGL2RenderingContext): WebGLProgram {
  const vs = compileShader(gl, gl.VERTEX_SHADER, EDGE_VS)
  const fs = compileShader(gl, gl.FRAGMENT_SHADER, EDGE_FS)
  return buildProgram(gl, vs, fs)
}

export function screenToGraph(sx: number, sy: number, zoom: number, panX: number, panY: number, rotation: number, vw: number, vh: number): [number, number] {
  let x = (sx - vw * 0.5) / zoom
  let y = (vh * 0.5 - sy) / zoom
  const c = Math.cos(-rotation)
  const s = Math.sin(-rotation)
  const rx = x * c - y * s
  const ry = x * s + y * c
  return [rx - panX, ry - panY]
}
