const vertexSource = `#version 300 es
layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec4 a_rect;
layout(location = 2) in vec4 a_color;
layout(location = 3) in vec2 a_shape;
layout(location = 4) in float a_mode;
uniform vec2 u_screen;
out vec2 v_local;
out vec2 v_size;
out vec4 v_color;
out float v_radius;
out float v_shadow;
flat out int v_mode;
void main() {
  vec2 px = a_rect.xy + a_pos * a_rect.zw;
  vec2 ndc = vec2(px.x / u_screen.x * 2.0 - 1.0, 1.0 - px.y / u_screen.y * 2.0);
  gl_Position = vec4(ndc, 0.0, 1.0);
  v_local = a_pos * a_rect.zw;
  v_size = a_rect.zw;
  v_color = a_color;
  v_radius = a_shape.x;
  v_shadow = a_shape.y;
  v_mode = int(a_mode + 0.5);
}`;

const fragmentSource = `#version 300 es
precision highp float;
in vec2 v_local;
in vec2 v_size;
in vec4 v_color;
in float v_radius;
in float v_shadow;
flat in int v_mode;
out vec4 out_color;
float rounded_box(vec2 p, vec2 b, float r) {
  vec2 q = abs(p) - b + vec2(r);
  return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}
void main() {
  vec2 p = v_local - v_size * 0.5;
  float d = rounded_box(p, v_size * 0.5, v_radius);
  float aa = max(fwidth(d), 0.75);
  float alpha = 1.0 - smoothstep(0.0, aa, d);
  if (v_mode == 1) {
    float sd = rounded_box(p - vec2(0.0, -v_shadow * 0.18), v_size * 0.5, v_radius + v_shadow * 0.35);
    float blur = max(v_shadow, 1.0);
    alpha = 1.0 - smoothstep(-blur, blur, sd);
    out_color = vec4(v_color.rgb, v_color.a * alpha * 0.28);
  } else if (v_mode == 2) {
    float inner = rounded_box(p, v_size * 0.5 - vec2(1.25), max(v_radius - 1.25, 0.0));
    float border = (1.0 - smoothstep(0.0, aa, d)) * smoothstep(0.0, aa, inner);
    out_color = vec4(v_color.rgb, v_color.a * border);
  } else {
    out_color = vec4(v_color.rgb, v_color.a * alpha);
  }
}`;

const texturedVertexSource = `#version 300 es
layout(location = 0) in vec4 a_data;
layout(location = 1) in vec4 a_color;
uniform vec2 u_screen;
out vec2 v_uv;
out vec4 v_color;
void main() {
  vec2 px = a_data.xy;
  vec2 ndc = vec2(px.x / u_screen.x * 2.0 - 1.0, 1.0 - px.y / u_screen.y * 2.0);
  gl_Position = vec4(ndc, 0.0, 1.0);
  v_uv = a_data.zw;
  v_color = a_color;
}`;

const texturedFragmentSource = `#version 300 es
precision highp float;
in vec2 v_uv;
in vec4 v_color;
out vec4 out_color;
uniform sampler2D u_tex;
void main() {
  float a = texture(u_tex, v_uv).r;
  out_color = vec4(v_color.rgb, v_color.a * a);
}`;

const canvas = document.getElementById("edgerun");
const gl = canvas.getContext("webgl2", { antialias: true, alpha: false });
if (!gl) throw new Error("WebGL2 is required");

function shader(type, source) {
  const value = gl.createShader(type);
  gl.shaderSource(value, source);
  gl.compileShader(value);
  if (!gl.getShaderParameter(value, gl.COMPILE_STATUS)) {
    throw new Error(gl.getShaderInfoLog(value));
  }
  return value;
}

function program(vertex, fragment) {
  const value = gl.createProgram();
  gl.attachShader(value, shader(gl.VERTEX_SHADER, vertex));
  gl.attachShader(value, shader(gl.FRAGMENT_SHADER, fragment));
  gl.linkProgram(value);
  if (!gl.getProgramParameter(value, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(value));
  }
  return value;
}

async function instantiateWasm(path) {
  const response = await fetch(path);
  if (WebAssembly.instantiateStreaming && response.headers.get("content-type") === "application/wasm") {
    return WebAssembly.instantiateStreaming(response, {});
  }
  return WebAssembly.instantiate(await response.arrayBuffer(), {});
}

const wasm = await instantiateWasm("./edgerun_docs_ui_web.wasm");
const api = wasm.instance.exports;
const rectStride = api.edgerun_docs_rect_float_stride();
const textStride = api.edgerun_docs_text_vertex_float_stride();
const iconStride = api.edgerun_docs_icon_vertex_float_stride();
const rectBytes = rectStride * 4;
const modeOffset = (rectStride - 1) * 4;

const shapeProgram = program(vertexSource, fragmentSource);
const shapeVao = gl.createVertexArray();
gl.bindVertexArray(shapeVao);
const quadVbo = gl.createBuffer();
gl.bindBuffer(gl.ARRAY_BUFFER, quadVbo);
gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([
  0, 0, 1, 0, 1, 1,
  0, 0, 1, 1, 0, 1,
]), gl.STATIC_DRAW);
gl.enableVertexAttribArray(0);
gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 8, 0);
const rectVbo = gl.createBuffer();
gl.bindBuffer(gl.ARRAY_BUFFER, rectVbo);
gl.enableVertexAttribArray(1);
gl.vertexAttribPointer(1, 4, gl.FLOAT, false, rectBytes, 0);
gl.vertexAttribDivisor(1, 1);
gl.enableVertexAttribArray(2);
gl.vertexAttribPointer(2, 4, gl.FLOAT, false, rectBytes, 24);
gl.vertexAttribDivisor(2, 1);
gl.enableVertexAttribArray(3);
gl.vertexAttribPointer(3, 2, gl.FLOAT, false, rectBytes, 16);
gl.vertexAttribDivisor(3, 1);
gl.enableVertexAttribArray(4);
gl.vertexAttribPointer(4, 1, gl.FLOAT, false, rectBytes, modeOffset);
gl.vertexAttribDivisor(4, 1);
const shapeScreen = gl.getUniformLocation(shapeProgram, "u_screen");

const texturedProgram = program(texturedVertexSource, texturedFragmentSource);
const texturedVao = gl.createVertexArray();
gl.bindVertexArray(texturedVao);
const texturedVbo = gl.createBuffer();
gl.bindBuffer(gl.ARRAY_BUFFER, texturedVbo);
gl.enableVertexAttribArray(0);
gl.vertexAttribPointer(0, 4, gl.FLOAT, false, 32, 0);
gl.enableVertexAttribArray(1);
gl.vertexAttribPointer(1, 4, gl.FLOAT, false, 32, 16);
const texturedScreen = gl.getUniformLocation(texturedProgram, "u_screen");
const texturedTex = gl.getUniformLocation(texturedProgram, "u_tex");

function alphaTexture(width, height, ptr) {
  const texture = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  const bytes = new Uint8Array(api.memory.buffer, ptr, width * height);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, width, height, 0, gl.RED, gl.UNSIGNED_BYTE, bytes);
  return texture;
}

const fontTexture = alphaTexture(
  api.edgerun_docs_font_atlas_width(),
  api.edgerun_docs_font_atlas_height(),
  api.edgerun_docs_font_atlas_ptr(),
);
const iconTexture = alphaTexture(
  api.edgerun_docs_icon_atlas_width(),
  api.edgerun_docs_icon_atlas_height(),
  api.edgerun_docs_icon_atlas_ptr(),
);

let dirty = true;
let scheduled = false;
let lastTime = 0;

function resize() {
  const dpr = Math.max(1, window.devicePixelRatio || 1);
  const width = Math.max(1, Math.floor(canvas.clientWidth * dpr));
  const height = Math.max(1, Math.floor(canvas.clientHeight * dpr));
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
    dirty = true;
  }
  return { width, height };
}

function schedule() {
  if (scheduled) return;
  scheduled = true;
  requestAnimationFrame(draw);
}

function mark(changed) {
  if (changed || !scheduled) {
    dirty = true;
    schedule();
  }
}

function point(event) {
  const dpr = Math.max(1, window.devicePixelRatio || 1);
  const rect = canvas.getBoundingClientRect();
  return {
    x: (event.clientX - rect.left) * dpr,
    y: (event.clientY - rect.top) * dpr,
  };
}

canvas.addEventListener("pointerdown", (event) => {
  canvas.focus();
  canvas.setPointerCapture(event.pointerId);
  const p = point(event);
  mark(api.edgerun_docs_handle_pointer_down(p.x, p.y));
});

canvas.addEventListener("pointermove", (event) => {
  const p = point(event);
  mark(api.edgerun_docs_handle_pointer_move(p.x, p.y));
});

canvas.addEventListener("pointerup", (event) => {
  const p = point(event);
  mark(api.edgerun_docs_handle_pointer_up(p.x, p.y));
});

canvas.addEventListener("wheel", (event) => {
  event.preventDefault();
  const p = point(event);
  mark(api.edgerun_docs_handle_wheel(p.x, p.y, event.deltaY));
}, { passive: false });

canvas.addEventListener("keydown", (event) => {
  mark(api.edgerun_docs_handle_key(event.keyCode));
});

canvas.addEventListener("blur", () => {
  mark(api.edgerun_docs_handle_blur());
});

new ResizeObserver(() => mark(1)).observe(canvas);

function drawTextured(texture, ptrFn, lenFn, stride) {
  const len = lenFn();
  if (len === 0) return;
  const ptr = ptrFn();
  const values = new Float32Array(api.memory.buffer, ptr, len);
  gl.useProgram(texturedProgram);
  gl.bindVertexArray(texturedVao);
  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.uniform1i(texturedTex, 0);
  gl.uniform2f(texturedScreen, canvas.width, canvas.height);
  gl.bindBuffer(gl.ARRAY_BUFFER, texturedVbo);
  gl.bufferData(gl.ARRAY_BUFFER, values, gl.DYNAMIC_DRAW);
  gl.drawArrays(gl.TRIANGLES, 0, len / stride);
}

function draw(time) {
  scheduled = false;
  const { width, height } = resize();
  const delta = Math.min(64, Math.max(16, Math.floor(time - lastTime) || 16));
  lastTime = time;
  if (!dirty) return;
  api.edgerun_docs_build_frame(width, height, delta);
  dirty = false;

  gl.viewport(0, 0, width, height);
  gl.clearColor(0.972, 0.98, 0.988, 1.0);
  gl.clear(gl.COLOR_BUFFER_BIT);
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

  const rectLen = api.edgerun_docs_rect_buffer_len();
  if (rectLen > 0) {
    const rectPtr = api.edgerun_docs_rect_buffer_ptr();
    const rects = new Float32Array(api.memory.buffer, rectPtr, rectLen);
    gl.useProgram(shapeProgram);
    gl.bindVertexArray(shapeVao);
    gl.uniform2f(shapeScreen, width, height);
    gl.bindBuffer(gl.ARRAY_BUFFER, rectVbo);
    gl.bufferData(gl.ARRAY_BUFFER, rects, gl.DYNAMIC_DRAW);
    gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, rectLen / rectStride);
  }

  drawTextured(
    iconTexture,
    api.edgerun_docs_icon_vertex_buffer_ptr,
    api.edgerun_docs_icon_vertex_buffer_len,
    iconStride,
  );
  drawTextured(
    fontTexture,
    api.edgerun_docs_text_vertex_buffer_ptr,
    api.edgerun_docs_text_vertex_buffer_len,
    textStride,
  );
}

schedule();
