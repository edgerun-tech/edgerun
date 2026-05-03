/**
 * WebGL shader programs and utilities.
 */

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
}`;

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
}`;

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
}`;

export const EDGE_FS = `#version 300 es
precision mediump float;
in vec3 v_color;
out vec4 fragColor;
void main() {
    fragColor = vec4(v_color, 1.0);
}`;

function compileShader(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const info = gl.getShaderInfoLog(shader);
    gl.deleteShader(shader);
    throw new Error(`Shader compile: ${info}`);
  }
  return shader;
}

function buildProgram(gl, vs, fs) {
  const vShader = compileShader(gl, gl.VERTEX_SHADER, vs);
  const fShader = compileShader(gl, gl.FRAGMENT_SHADER, fs);
  const prog = gl.createProgram();
  gl.attachShader(prog, vShader);
  gl.attachShader(prog, fShader);
  gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
    const info = gl.getProgramInfoLog(prog);
    gl.deleteProgram(prog);
    throw new Error(`Program link: ${info}`);
  }
  return prog;
}

export function buildNodeProgram(gl) {
  return buildProgram(gl, NODE_VS, NODE_FS);
}

export function buildEdgeProgram(gl) {
  return buildProgram(gl, EDGE_VS, EDGE_FS);
}

export const LANG_COLORS = {
  c: [0.35, 0.6, 0.95, 0.9],
  rust: [0.95, 0.5, 0.15, 0.9],
  typescript: [0.4, 0.8, 0.4, 0.9],
  javascript: [0.9, 0.8, 0.2, 0.9],
  unknown: [0.5, 0.5, 0.55, 0.7],
};

/** Convert screen coords to graph coords */
export function screenToGraph(sx, sy, zoomVal, panXVal, panYVal, rot, vw, vh) {
  let x = (sx - vw * 0.5) / zoomVal;
  let y = (vh * 0.5 - sy) / zoomVal;
  const c = Math.cos(-rot);
  const s = Math.sin(-rot);
  const rx = x * c - y * s;
  const ry = x * s + y * c;
  return [rx - panXVal, ry - panYVal];
}

/** Convert graph coords to screen coords */
export function graphToScreen(gx, gy, zoomVal, panXVal, panYVal, rot, vw, vh) {
  const px = gx + panXVal;
  const py = gy + panYVal;
  const c = Math.cos(rot);
  const s = Math.sin(rot);
  const rx = px * c - py * s;
  const ry = px * s + py * c;
  return [rx * zoomVal + vw * 0.5, vh * 0.5 - ry * zoomVal];
}
