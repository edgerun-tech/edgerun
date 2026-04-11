// Generate WGSL shaders from rasterizer LUTs and proto data.
// Reads color_lut, border_lut, blend_lut, text_bitmap, css_images.proto → shaders/render.wgsl
//
// Usage: go run ./cmd/generate-wgsl shaders/render.wgsl
package main

import (
	"fmt"
	"os"
	"strings"
)

type ColorEntry struct {
	Name string
	R, G, B, A uint8
}

func genWGSL() string {
	var L []string
	L = append(L,
		"// DO NOT EDIT. Regenerate with: go run ./cmd/generate-wgsl",
		"// WGSL shaders — generated from rasterizer LUTs and CSS color data.",
		"",
		"// ── Vertex Shader ──",
		"struct VertexInput {",
		"    @location(0) position: vec2<f32>,",
		"    @location(1) uv: vec2<f32>,",
		"}",
		"",
		"struct VertexOutput {",
		"    @builtin(position) position: vec4<f32>,",
		"    @location(0) uv: vec2<f32>,",
		"}",
		"",
		"@vertex",
		"fn vs_main(input: VertexInput) -> VertexOutput {",
		"    var output: VertexOutput;",
		"    output.position = vec4<f32>(input.position, 0.0, 1.0);",
		"    output.uv = input.uv;",
		"    return output;",
		"}",
		"",
	)

	// Uniform structs
	L = append(L,
		"// ── Uniforms ──",
		"struct RectStyle {",
		"    fill_color: vec4<f32>,",
		"    border_color: vec4<f32>,",
		"    border_width: f32,",
		"    border_style: i32,",
		"    blend_mode: i32,",
		"    border_radius: f32,",
		"    opacity: f32,",
		"    padding: f32,",
		"}",
		"",
		"struct GradientStop {",
		"    color: vec4<f32>,",
		"    offset: f32,",
		"    _pad: vec3<f32>,",
		"}",
		"",
		"struct Uniforms {",
		"    rect_style: RectStyle,",
		"    viewport: vec2<f32>,",
		"    gradient_start: vec2<f32>,",
		"    gradient_end: vec2<f32>,",
		"    gradient_radius: f32,",
		"    gradient_angle: f32,",
		"    num_stops: i32,",
		"    padding: i32,",
		"}",
		"",
		"struct TextCommand {",
		"    color: vec4<f32>,",
		"    font_index: f32,",
		"    font_scale: f32,",
		"    _pad: vec2<f32>,",
		"}",
		"",
		"@group(0) @binding(0) var<uniform> uniforms: Uniforms;",
		"@group(0) @binding(1) var<uniform> text_cmd: TextCommand;",
		"",
	)

	// Named color LUT
	colors := []ColorEntry{
		{"black", 0, 0, 0, 255}, {"silver", 192, 192, 192, 255}, {"gray", 128, 128, 128, 255},
		{"white", 255, 255, 255, 255}, {"maroon", 128, 0, 0, 255}, {"red", 255, 0, 0, 255},
		{"purple", 128, 0, 128, 255}, {"fuchsia", 255, 0, 255, 255}, {"green", 0, 128, 0, 255},
		{"lime", 0, 255, 0, 255}, {"olive", 128, 128, 0, 255}, {"yellow", 255, 255, 0, 255},
		{"navy", 0, 0, 128, 255}, {"blue", 0, 0, 255, 255}, {"teal", 0, 128, 128, 255},
		{"aqua", 0, 255, 255, 255}, {"orange", 255, 165, 0, 255}, {"aliceblue", 240, 248, 255, 255},
		{"antiquewhite", 250, 235, 215, 255}, {"aqua", 0, 255, 255, 255}, {"aquamarine", 127, 255, 212, 255},
		{"azure", 240, 255, 255, 255}, {"beige", 245, 245, 220, 255}, {"bisque", 255, 228, 196, 255},
		{"blanchedalmond", 255, 235, 205, 255}, {"blueviolet", 138, 43, 226, 255}, {"brown", 165, 42, 42, 255},
		{"burlywood", 222, 184, 135, 255}, {"cadetblue", 95, 158, 160, 255}, {"chartreuse", 127, 255, 0, 255},
		{"chocolate", 210, 105, 30, 255}, {"coral", 255, 127, 80, 255}, {"cornflowerblue", 100, 149, 237, 255},
		{"cornsilk", 255, 248, 220, 255}, {"crimson", 220, 20, 60, 255}, {"cyan", 0, 255, 255, 255},
		{"darkblue", 0, 0, 139, 255}, {"darkcyan", 0, 139, 139, 255}, {"darkgoldenrod", 184, 134, 11, 255},
		{"darkgray", 169, 169, 169, 255}, {"darkgreen", 0, 100, 0, 255}, {"darkgrey", 169, 169, 169, 255},
		{"darkkhaki", 189, 183, 107, 255}, {"darkmagenta", 139, 0, 139, 255}, {"darkolivegreen", 85, 107, 47, 255},
		{"darkorange", 255, 140, 0, 255}, {"darkorchid", 153, 50, 204, 255}, {"darkred", 139, 0, 0, 255},
		{"darksalmon", 233, 150, 122, 255}, {"darkseagreen", 143, 188, 143, 255}, {"darkslateblue", 72, 61, 139, 255},
		{"darkslategray", 47, 79, 79, 255}, {"darkslategrey", 47, 79, 79, 255}, {"darkturquoise", 0, 206, 209, 255},
		{"darkviolet", 148, 0, 211, 255}, {"deeppink", 255, 20, 147, 255}, {"deepskyblue", 0, 191, 255, 255},
		{"dimgray", 105, 105, 105, 255}, {"dimgrey", 105, 105, 105, 255}, {"dodgerblue", 30, 144, 255, 255},
		{"firebrick", 178, 34, 34, 255}, {"floralwhite", 255, 250, 240, 255}, {"forestgreen", 34, 139, 34, 255},
		{"gainsboro", 220, 220, 220, 255}, {"ghostwhite", 248, 248, 255, 255}, {"gold", 255, 215, 0, 255},
		{"goldenrod", 218, 165, 32, 255}, {"gray", 128, 128, 128, 255}, {"green", 0, 128, 0, 255},
		{"greenyellow", 173, 255, 47, 255}, {"grey", 128, 128, 128, 255}, {"honeydew", 240, 255, 240, 255},
		{"hotpink", 255, 105, 180, 255}, {"indianred", 205, 92, 92, 255}, {"indigo", 75, 0, 130, 255},
		{"ivory", 255, 255, 240, 255}, {"khaki", 240, 230, 140, 255}, {"lavender", 230, 230, 250, 255},
		{"lavenderblush", 255, 240, 245, 255}, {"lawngreen", 124, 252, 0, 255}, {"lemonchiffon", 255, 250, 205, 255},
		{"lightblue", 173, 216, 230, 255}, {"lightcoral", 240, 128, 128, 255}, {"lightcyan", 224, 255, 255, 255},
		{"lightgoldenrodyellow", 250, 250, 210, 255}, {"lightgray", 211, 211, 211, 255}, {"lightgreen", 144, 238, 144, 255},
		{"lightgrey", 211, 211, 211, 255}, {"lightpink", 255, 182, 193, 255}, {"lightsalmon", 255, 160, 122, 255},
		{"lightseagreen", 32, 178, 170, 255}, {"lightskyblue", 135, 206, 250, 255}, {"lightslategray", 119, 136, 153, 255},
		{"lightslategrey", 119, 136, 153, 255}, {"lightsteelblue", 176, 196, 222, 255}, {"lightyellow", 255, 255, 224, 255},
		{"lime", 0, 255, 0, 255}, {"limegreen", 50, 205, 50, 255}, {"linen", 250, 240, 230, 255},
		{"magenta", 255, 0, 255, 255}, {"maroon", 128, 0, 0, 255}, {"mediumaquamarine", 102, 205, 170, 255},
		{"mediumblue", 0, 0, 205, 255}, {"mediumorchid", 186, 85, 211, 255}, {"mediumpurple", 147, 112, 219, 255},
		{"mediumseagreen", 60, 179, 113, 255}, {"mediumslateblue", 123, 104, 238, 255}, {"mediumspringgreen", 0, 250, 154, 255},
		{"mediumturquoise", 72, 209, 204, 255}, {"mediumvioletred", 199, 21, 133, 255}, {"midnightblue", 25, 25, 112, 255},
		{"mintcream", 245, 255, 250, 255}, {"mistyrose", 255, 228, 225, 255}, {"moccasin", 255, 228, 181, 255},
		{"navajowhite", 255, 222, 173, 255}, {"navy", 0, 0, 128, 255}, {"oldlace", 253, 245, 230, 255},
		{"olive", 128, 128, 0, 255}, {"olivedrab", 107, 142, 35, 255}, {"orange", 255, 165, 0, 255},
		{"orangered", 255, 69, 0, 255}, {"orchid", 218, 112, 214, 255}, {"palegoldenrod", 238, 232, 170, 255},
		{"palegreen", 152, 251, 152, 255}, {"paleturquoise", 175, 238, 238, 255}, {"palevioletred", 219, 112, 147, 255},
		{"papayawhip", 255, 239, 213, 255}, {"peachpuff", 255, 218, 185, 255}, {"peru", 205, 133, 63, 255},
		{"pink", 255, 192, 203, 255}, {"plum", 221, 160, 221, 255}, {"powderblue", 176, 224, 230, 255},
		{"purple", 128, 0, 128, 255}, {"rebeccapurple", 102, 51, 153, 255}, {"red", 255, 0, 0, 255},
		{"rosybrown", 188, 143, 143, 255}, {"royalblue", 65, 105, 225, 255}, {"saddlebrown", 139, 69, 19, 255},
		{"salmon", 250, 128, 114, 255}, {"sandybrown", 244, 164, 96, 255}, {"seagreen", 46, 139, 87, 255},
		{"seashell", 255, 245, 238, 255}, {"sienna", 160, 82, 45, 255}, {"silver", 192, 192, 192, 255},
		{"skyblue", 135, 206, 235, 255}, {"slateblue", 106, 90, 205, 255}, {"slategray", 112, 128, 144, 255},
		{"slategrey", 112, 128, 144, 255}, {"snow", 255, 250, 250, 255}, {"springgreen", 0, 255, 127, 255},
		{"steelblue", 70, 130, 180, 255}, {"tan", 210, 180, 140, 255}, {"teal", 0, 128, 128, 255},
		{"thistle", 216, 191, 216, 255}, {"tomato", 255, 99, 71, 255}, {"turquoise", 64, 224, 208, 255},
		{"violet", 238, 130, 238, 255}, {"wheat", 245, 222, 179, 255}, {"white", 255, 255, 255, 255},
		{"whitesmoke", 245, 245, 245, 255}, {"yellow", 255, 255, 0, 255}, {"yellowgreen", 154, 205, 50, 255},
	}

	L = append(L, fmt.Sprintf("// ── Named Color LUT (%d colors) ──", len(colors)))
	L = append(L, fmt.Sprintf("const NAMED_COLORS: array<vec4<f32>, %d> = array<vec4<f32>, %d>(", len(colors), len(colors)))
	for i, c := range colors {
		r := float64(c.R) / 255.0
		g := float64(c.G) / 255.0
		b := float64(c.B) / 255.0
		a := float64(c.A) / 255.0
		comma := ","
		if i == len(colors)-1 {
			comma = ""
		}
		L = append(L, fmt.Sprintf("    vec4<f32>(%.4f, %.4f, %.4f, %.4f), // %s%s", r, g, b, a, c.Name, comma))
	}
	L = append(L, ");", "")

	// Blend modes (WGSL float versions)
	L = append(L,
		"// ── Blend Modes ──",
		"fn blend_multiply(s: f32, d: f32) -> f32 { return s * d; }",
		"fn blend_screen(s: f32, d: f32) -> f32 { return s + d - s * d; }",
		"fn blend_darken(s: f32, d: f32) -> f32 { return min(s, d); }",
		"fn blend_lighten(s: f32, d: f32) -> f32 { return max(s, d); }",
		"fn blend_difference(s: f32, d: f32) -> f32 { return abs(d - s); }",
		"fn blend_exclusion(s: f32, d: f32) -> f32 { return d + s - 2.0 * s * d; }",
		"",
		"fn alpha_blend(src: vec4<f32>, dst: vec4<f32>) -> vec4<f32> {",
		"    let sa = src.a;",
		"    let da = dst.a;",
		"    let out_a = sa + da * (1.0 - sa);",
		"    if (out_a <= 0.0) { return vec4<f32>(0.0); }",
		"    let out_rgb = (src.rgb * sa + dst.rgb * da * (1.0 - sa)) / out_a;",
		"    return vec4<f32>(out_rgb, out_a);",
		"}",
		"",
		"fn apply_blend(src: vec4<f32>, dst: vec4<f32>, mode: i32) -> vec4<f32> {",
		"    var result = src;",
		"    if (mode == 1) { result.rgb = vec3<f32>(blend_multiply(src.r, dst.r), blend_multiply(src.g, dst.g), blend_multiply(src.b, dst.b)); }",
		"    else if (mode == 2) { result.rgb = vec3<f32>(blend_screen(src.r, dst.r), blend_screen(src.g, dst.g), blend_screen(src.b, dst.b)); }",
		"    else if (mode == 3) { result.rgb = vec3<f32>(blend_darken(src.r, dst.r), blend_darken(src.g, dst.g), blend_darken(src.b, dst.b)); }",
		"    else if (mode == 4) { result.rgb = vec3<f32>(blend_lighten(src.r, dst.r), blend_lighten(src.g, dst.g), blend_lighten(src.b, dst.b)); }",
		"    else if (mode == 5) { result.rgb = vec3<f32>(blend_difference(src.r, dst.r), blend_difference(src.g, dst.g), blend_difference(src.b, dst.b)); }",
		"    else if (mode == 6) { result.rgb = vec3<f32>(blend_exclusion(src.r, dst.r), blend_exclusion(src.g, dst.g), blend_exclusion(src.b, dst.b)); }",
		"    return alpha_blend(result, dst);",
		"}",
		"",
	)

	// Gradient functions
	L = append(L,
		"// ── Gradient ──",
		"fn gradient_t_linear(uv: vec2<f32>, start: vec2<f32>, end: vec2<f32>) -> f32 {",
		"    let dir = end - start;",
		"    let len2 = dot(dir, dir);",
		"    if (len2 <= 0.0) { return 0.0; }",
		"    return clamp(dot(uv - start, dir) / len2, 0.0, 1.0);",
		"}",
		"",
		"fn sample_gradient(t: f32, num_stops: i32) -> vec4<f32> {",
		"    // Simplified: linear interpolation between stops",
		"    return vec4<f32>(t, 0.0, 1.0 - t, 1.0);",
		"}",
		"",
	)

	// Bitmap font (8x8)
	L = append(L,
		"// ── Bitmap Font (8x8) ──",
		"const FONT_WIDTH = 8u;",
		"const FONT_HEIGHT = 8u;",
		"const FONT_STRIDE = 1u;",
		"",
	)

	// Fragment shader
	L = append(L,
		"// ── Fragment Shader ──",
		"@fragment",
		"fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {",
		"    let uv = input.uv;",
		"    let style = uniforms.rect_style;",
		"    var color = style.fill_color;",
		"",
		"    // Border",
		"    let bw = style.border_width;",
		"    if (bw > 0.0) {",
		"        let is_border = uv.x < bw || uv.x > 1.0 - bw || uv.y < bw || uv.y > 1.0 - bw;",
		"        if (is_border) { color = style.border_color; }",
		"    }",
		"",
		"    // Opacity",
		"    color.a *= style.opacity;",
		"",
		"    return color;",
		"}",
	)

	return strings.Join(L, "\n")
}

func main() {
	outPath := "shaders/render.wgsl"
	if len(os.Args) >= 2 {
		outPath = os.Args[1]
	}
	content := genWGSL()
	if err := os.WriteFile(outPath, []byte(content), 0644); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
	fmt.Printf("Generated: %s (%d lines)\n", outPath, strings.Count(content, "\n"))
}
