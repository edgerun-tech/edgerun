// edgerun-layout.wgsl — GPU-native CSS cascade + height computation
//
// Two-pass design:
//   Pass 0: Cascade resolution (fully parallel — one workgroup per node)
//   Pass 1: Height computation (fully parallel — depends only on cascade results)
//
// Y positioning is done on the CPU (sequential tree walk) since WGSL
// compute shaders don't support recursion and sequential dependency.

// ─── Input Bindings ───

struct DomNode {
    tag_hash: u32,
    class_hash: u32,
    id_hash: u32,
    parent_idx: u32,
    first_child_idx: u32,
    next_sibling_idx: u32,
    text_offset: u32,
    text_len: u32,
}

// CSS rule with property declarations
struct CssRule {
    // Selector
    tag_hash: u32,
    class_hash: u32,
    id_hash: u32,
    // Specificity (a=ids, b=classes, c=elements)
    specificity_a: u32,
    specificity_b: u32,
    specificity_c: u32,
    // Declarations (-1.0 = not specified, uses default/inherited)
    font_size: f32,      // px
    color_r: f32, color_g: f32, color_b: f32,
    bg_r: f32, bg_g: f32, bg_b: f32,
    height: f32,         // explicit height, 0 = auto
    width: f32,          // explicit width, 0 = auto
    margin_top: f32,
    margin_bottom: f32,
    padding_top: f32,
    padding_bottom: f32,
    // Source order (rule index in stylesheet, for tie-breaking)
    source_order: u32,
    // Whether properties are important (for cascade priority)
    is_important: u32,   // bitmask: bit 0=font_size, 1=color, 2=bg, 3=height
    _pad0: u32,
}

// Cascade result — computed style per node
struct StyleResult {
    font_size: f32,
    color_r: f32, color_g: f32, color_b: f32, color_a: f32,
    bg_r: f32, bg_g: f32, bg_b: f32, bg_a: f32,
    has_explicit_bg: u32,
    explicit_height: f32,
    margin_top: f32, margin_bottom: f32,
    padding_top: f32, padding_bottom: f32,
    computed_width: f32,
}

// Layout result — one per DOM node
struct LayoutResult {
    x: f32, y: f32, w: f32, h: f32,
    content_x: f32, content_y: f32, content_w: f32, content_h: f32,
    font_size: f32,
    color_r: f32, color_g: f32, color_b: f32,
    bg_r: f32, bg_g: f32, bg_b: f32,
    has_bg: u32,
    is_text: u32,
    is_block: u32,
    margin_top: f32, margin_bottom: f32,
    padding_top: f32, padding_bottom: f32,
}

// Dispatch configuration
struct LayoutConfig {
    node_count: u32,
    rule_count: u32,
    text_len: u32,
    pass: u32,           // 0=cascade, 1=inherit, 2=heights
    avail_width: f32,
    base_x: f32,
    base_y: f32,
    _pad1: u32,
}

@group(0) @binding(0) var<storage, read> nodes: array<DomNode>;
@group(0) @binding(1) var<storage, read> rules: array<CssRule>;
@group(0) @binding(2) var<storage, read> text_buffer: array<u32>;
@group(0) @binding(3) var<uniform> config: LayoutConfig;
@group(0) @binding(4) var<storage, read_write> style_results: array<StyleResult>;
@group(0) @binding(5) var<storage, read_write> layout_results: array<LayoutResult>;

// ─── Default Values ───

const DEFAULT_FONT_SIZE: f32 = 16.0;
const DEFAULT_COLOR: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);
const DEFAULT_BG: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);

// ─── Cascade Win Comparison ───
/// Does the incoming rule beat the current winner for a given property?
///
/// Priority (per CSS spec):
///   1. !important > normal declarations
///   2. Higher specificity wins
///   3. Later source order wins (tie-breaker)
fn cascade_wins(
    r_sa: u32, r_sb: u32, r_sc: u32, r_so: u32, r_imp: bool,
    w_sa: u32, w_sb: u32, w_sc: u32, w_so: u32, w_imp: u32,
) -> bool {
    // If no winner yet, new rule wins
    if (w_sa == 0u && w_sb == 0u && w_sc == 0u && w_so == 0u) { return true; }

    // Importance check
    let r_imp_u = u32(r_imp);
    if (r_imp_u > w_imp) { return true; }
    if (w_imp > r_imp_u) { return false; }

    // Same importance → compare specificity (a=ids, b=classes, c=elements)
    if (r_sa > w_sa) { return true; }
    if (r_sa < w_sa) { return false; }
    if (r_sb > w_sb) { return true; }
    if (r_sb < w_sb) { return false; }
    if (r_sc > w_sc) { return true; }
    if (r_sc < w_sc) { return false; }

    // Same specificity → later source order wins
    if (r_so > w_so) { return true; }

    return false;
}

// ─── Pass 0: Cascade Resolution ───
/// Match each DOM node against all CSS rules, pick winning style per property.
/// Each property cascades independently — color from one rule, font-size from another.
///
/// Runs fully parallel: one invocation per node, all read-only rules,
/// writes to independent style_results[node_idx].

fn cascade_resolve(node_idx: u32) {
    let node = nodes[node_idx];
    let is_text = node.text_len > 0u;

    // Initialize with CSS defaults
    var font_size: f32 = DEFAULT_FONT_SIZE;
    var color: vec3<f32> = DEFAULT_COLOR;
    var bg: vec3<f32> = DEFAULT_BG;
    var explicit_height: f32 = 0.0;
    var computed_width: f32 = config.avail_width;
    var margin_top: f32 = 0.0;
    var margin_bottom: f32 = 0.0;
    var padding_top: f32 = 0.0;
    var padding_bottom: f32 = 0.0;
    var has_explicit_bg = false;

    // Winning specificity + source order per property
    var w_font_sa: u32 = 0u; var w_font_sb: u32 = 0u; var w_font_sc: u32 = 0u; var w_font_so: u32 = 0u; var w_font_imp: u32 = 0u;
    var w_col_sa: u32 = 0u; var w_col_sb: u32 = 0u; var w_col_sc: u32 = 0u; var w_col_so: u32 = 0u; var w_col_imp: u32 = 0u;
    var w_bg_sa: u32 = 0u; var w_bg_sb: u32 = 0u; var w_bg_sc: u32 = 0u; var w_bg_so: u32 = 0u; var w_bg_imp: u32 = 0u;
    var w_ht_sa: u32 = 0u; var w_ht_sb: u32 = 0u; var w_ht_sc: u32 = 0u; var w_ht_so: u32 = 0u; var w_ht_imp: u32 = 0u;
    var w_wd_sa: u32 = 0u; var w_wd_sb: u32 = 0u; var w_wd_sc: u32 = 0u; var w_wd_so: u32 = 0u; var w_wd_imp: u32 = 0u;

    // Iterate all rules — find best match per property
    for (var ri: u32 = 0u; ri < config.rule_count; ri = ri + 1u) {
        let rule = rules[ri];

        // ── Selector matching ──
        var matched = true;

        // Tag hash (0 = universal selector *)
        if (rule.tag_hash != 0u && rule.tag_hash != node.tag_hash) {
            matched = false;
        }

        // Class hash (0 = no class selector)
        if (matched && rule.class_hash != 0u && rule.class_hash != node.class_hash) {
            matched = false;
        }

        // ID hash (0 = no ID selector)
        if (matched && rule.id_hash != 0u && rule.id_hash != node.id_hash) {
            matched = false;
        }

        if (!matched) { continue; }

        let r_sa = rule.specificity_a;
        let r_sb = rule.specificity_b;
        let r_sc = rule.specificity_c;
        let r_so = rule.source_order;
        let r_imp = (rule.is_important != 0u);

        // ── font-size ──
        if (rule.font_size > 0.0) {
            if (cascade_wins(r_sa, r_sb, r_sc, r_so, r_imp,
                           w_font_sa, w_font_sb, w_font_sc, w_font_so, w_font_imp)) {
                font_size = rule.font_size;
                w_font_sa = r_sa; w_font_sb = r_sb; w_font_sc = r_sc;
                w_font_so = r_so; w_font_imp = u32(r_imp);
            }
        }

        // ── color ──
        if (rule.color_r >= 0.0) {
            if (cascade_wins(r_sa, r_sb, r_sc, r_so, r_imp,
                           w_col_sa, w_col_sb, w_col_sc, w_col_so, w_col_imp)) {
                color = vec3<f32>(rule.color_r, rule.color_g, rule.color_b);
                w_col_sa = r_sa; w_col_sb = r_sb; w_col_sc = r_sc;
                w_col_so = r_so; w_col_imp = u32(r_imp);
            }
        }

        // ── background-color ──
        if (rule.bg_r >= 0.0) {
            if (cascade_wins(r_sa, r_sb, r_sc, r_so, r_imp,
                           w_bg_sa, w_bg_sb, w_bg_sc, w_bg_so, w_bg_imp)) {
                bg = vec3<f32>(rule.bg_r, rule.bg_g, rule.bg_b);
                has_explicit_bg = true;
                w_bg_sa = r_sa; w_bg_sb = r_sb; w_bg_sc = r_sc;
                w_bg_so = r_so; w_bg_imp = u32(r_imp);
            }
        }

        // ── height ──
        if (rule.height > 0.0) {
            if (cascade_wins(r_sa, r_sb, r_sc, r_so, r_imp,
                           w_ht_sa, w_ht_sb, w_ht_sc, w_ht_so, w_ht_imp)) {
                explicit_height = rule.height;
                w_ht_sa = r_sa; w_ht_sb = r_sb; w_ht_sc = r_sc;
                w_ht_so = r_so; w_ht_imp = u32(r_imp);
            }
        }

        // ── width ──
        if (rule.width > 0.0) {
            if (cascade_wins(r_sa, r_sb, r_sc, r_so, r_imp,
                           w_wd_sa, w_wd_sb, w_wd_sc, w_wd_so, w_wd_imp)) {
                computed_width = rule.width;
                w_wd_sa = r_sa; w_wd_sb = r_sb; w_wd_sc = r_sc;
                w_wd_so = r_so; w_wd_imp = u32(r_imp);
            }
        }

        // Margins and padding accumulate (take max from any matching rule)
        margin_top = max(margin_top, rule.margin_top);
        margin_bottom = max(margin_bottom, rule.margin_bottom);
        padding_top = max(padding_top, rule.padding_top);
        padding_bottom = max(padding_bottom, rule.padding_bottom);
    }

    // Write style result
    var sr: StyleResult;
    sr.font_size = font_size;
    sr.color_r = color.r; sr.color_g = color.g; sr.color_b = color.b; sr.color_a = 1.0;
    sr.bg_r = bg.r; sr.bg_g = bg.g; sr.bg_b = bg.b; sr.bg_a = 1.0;
    sr.has_explicit_bg = u32(has_explicit_bg);
    sr.explicit_height = explicit_height;
    sr.margin_top = margin_top;
    sr.margin_bottom = margin_bottom;
    sr.padding_top = padding_top;
    sr.padding_bottom = padding_bottom;
    sr.computed_width = computed_width;
    style_results[node_idx] = sr;

    // Initialize layout result
    var lr: LayoutResult;
    lr.font_size = font_size;
    lr.color_r = color.r; lr.color_g = color.g; lr.color_b = color.b;
    lr.bg_r = bg.r; lr.bg_g = bg.g; lr.bg_b = bg.b;
    lr.has_bg = u32(has_explicit_bg);
    lr.is_text = u32(is_text);
    lr.is_block = u32(!is_text && node.tag_hash != 0u);
    lr.w = config.avail_width;
    lr.margin_top = margin_top;
    lr.margin_bottom = margin_bottom;
    lr.padding_top = padding_top;
    lr.padding_bottom = padding_bottom;
    layout_results[node_idx] = lr;
}

// ─── Pass 2: Style Inheritance ───
/// Propagate inherited CSS properties from parent to children.
/// Inherited properties: font-size, color.
/// Non-inherited: background-color, width, height, margin, padding, border.
///
/// Runs fully parallel: each node reads its parent's style_result.

fn inherit_styles(node_idx: u32) {
    let node = nodes[node_idx];

    // Text nodes inherit from their parent element
    // Root node has no parent to inherit from
    if (node.parent_idx == 0xFFFFFFFFu) { return; }

    let parent_style = style_results[node.parent_idx];
    var my_style = style_results[node_idx];
    var my_layout = layout_results[node_idx];

    // Inherit font-size if not explicitly set by any matching rule
    // (default value means no rule matched — inherit from parent)
    if (my_style.font_size == DEFAULT_FONT_SIZE) {
        my_style.font_size = parent_style.font_size;
    }

    // Inherit color if not explicitly set (default white means no rule matched)
    if (my_style.color_r == DEFAULT_COLOR.r &&
        my_style.color_g == DEFAULT_COLOR.g &&
        my_style.color_b == DEFAULT_COLOR.b) {
        my_style.color_r = parent_style.color_r;
        my_style.color_g = parent_style.color_g;
        my_style.color_b = parent_style.color_b;
    }

    // Background is NOT inherited per CSS spec
    // (has_explicit_bg flag already set during cascade)

    // Update both style and layout results
    style_results[node_idx] = my_style;

    my_layout.font_size = my_style.font_size;
    my_layout.color_r = my_style.color_r;
    my_layout.color_g = my_style.color_g;
    my_layout.color_b = my_style.color_b;
    my_layout.bg_r = my_style.bg_r;
    my_layout.bg_g = my_style.bg_g;
    my_layout.bg_b = my_style.bg_b;
    my_layout.has_bg = my_style.has_explicit_bg;
    layout_results[node_idx] = my_layout;
}

// ─── Pass 1: Height Computation ───
/// Text nodes: height based on font-size, text length, and word wrap.
/// Block elements: height = sum of children's heights + own margins + padding.
/// Explicit heights override computed values.
///
/// Runs fully parallel — each node computes its own height.
/// For blocks, this reads children's layout_results.h which were
/// computed in a previous dispatch (leaf-first ordering).

fn compute_heights(node_idx: u32) {
    let node = nodes[node_idx];
    var lr = layout_results[node_idx];
    let style = style_results[node_idx];

    if (lr.is_text != 0u) {
        // Text node: height = font_size × lines × line_height_ratio
        lr.h = compute_text_height(node.text_len, style.font_size);
        lr.w = config.avail_width;
        layout_results[node_idx] = lr;
        return;
    }

    if (lr.is_block != 0u) {
        // Block element: sum children's heights
        var child_h: f32 = 0.0;
        var ci = nodes[node_idx].first_child_idx;
        while (ci != 0xFFFFFFFFu) {
            child_h += layout_results[ci].h;
            ci = nodes[ci].next_sibling_idx;
        }

        // If explicit height is set, use max of computed and explicit
        let total_h = max(child_h, style.explicit_height);
        lr.h = total_h + style.padding_top + style.padding_bottom
             + style.margin_top + style.margin_bottom;
        lr.w = config.avail_width;
        layout_results[node_idx] = lr;
        return;
    }

    // Default: font-size based height
    lr.h = style.font_size * 1.5;
    layout_results[node_idx] = lr;
}

fn compute_text_height(text_len: u32, font_size: f32) -> f32 {
    // Estimate characters per line based on available width
    let char_w = font_size * 0.5;  // approximate monospace char width
    let cpl = max(u32(config.avail_width / char_w), 10u);
    let lines = max(u32(ceil(f32(text_len) / f32(cpl))), 1u);
    return font_size * 1.25 * f32(lines);  // 1.25 = line-height ratio
}

// ─── Compute Shader Entry Point ───
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let node_idx = global_id.x;
    if (node_idx >= config.node_count) { return; }

    if (config.pass == 0u) {
        cascade_resolve(node_idx);
    } else if (config.pass == 1u) {
        inherit_styles(node_idx);
    } else if (config.pass == 2u) {
        compute_heights(node_idx);
    }
}
