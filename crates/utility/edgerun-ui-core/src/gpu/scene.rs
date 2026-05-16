#[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
use super::UiIconAtlasRect;
use super::runtime::GpuHit;
#[cfg(feature = "fontdue-text")]
use super::{FontAtlas, TextQuad};
use core::fmt::Write;
use std::string::String;
use std::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color4 {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color4 {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb_u8(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
    }

    pub const fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::rgba(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }

    pub const fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RectMode {
    Fill,
    Shadow,
    Border,
    LinearGradient,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub radius: f32,
    pub color: Color4,
    pub color2: Color4,
    pub mode: RectMode,
    pub shadow: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuClip {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconQuad {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
    pub color: Color4,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GpuSceneStats {
    pub rects: usize,
    pub hits: usize,
    pub drag_sources: usize,
    pub drop_targets: usize,
    pub transitions: usize,
    pub clips: usize,
    pub icon_quads: usize,
    pub text_quads: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuSceneRenderLayer {
    Clear,
    Rects,
    IconQuads,
    TextQuads,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuSceneInteractionLayer {
    Hits,
    DragSources,
    DropTargets,
}

pub const GPU_SCENE_RENDER_ORDER: &[GpuSceneRenderLayer] = &[
    GpuSceneRenderLayer::Clear,
    GpuSceneRenderLayer::Rects,
    GpuSceneRenderLayer::IconQuads,
    GpuSceneRenderLayer::TextQuads,
];

pub const GPU_SCENE_INTERACTION_ORDER: &[GpuSceneInteractionLayer] = &[
    GpuSceneInteractionLayer::Hits,
    GpuSceneInteractionLayer::DragSources,
    GpuSceneInteractionLayer::DropTargets,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuSceneZOrder {
    pub render_layers: &'static [GpuSceneRenderLayer],
    pub interaction_layers: &'static [GpuSceneInteractionLayer],
    pub topmost_interaction_is_last: bool,
}

impl GpuSceneZOrder {
    pub const fn canonical() -> Self {
        Self {
            render_layers: GPU_SCENE_RENDER_ORDER,
            interaction_layers: GPU_SCENE_INTERACTION_ORDER,
            topmost_interaction_is_last: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuSceneBlendMode {
    SourceOverStraightAlpha,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuSceneBlendContract {
    pub mode: GpuSceneBlendMode,
    pub render_layers: &'static [GpuSceneRenderLayer],
    pub text_and_icons_are_over_rects: bool,
}

impl GpuSceneBlendContract {
    pub const fn canonical() -> Self {
        Self {
            mode: GpuSceneBlendMode::SourceOverStraightAlpha,
            render_layers: GPU_SCENE_RENDER_ORDER,
            text_and_icons_are_over_rects: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuSceneBudget {
    pub rects: usize,
    pub hits: usize,
    pub drag_sources: usize,
    pub drop_targets: usize,
    pub transitions: usize,
    pub icon_quads: usize,
    pub text_quads: usize,
}

impl GpuSceneBudget {
    pub const fn native_interactive_frame() -> Self {
        Self {
            rects: 2_000,
            hits: 900,
            drag_sources: 160,
            drop_targets: 160,
            transitions: 240,
            icon_quads: 1_200,
            text_quads: 8_000,
        }
    }

    pub const fn browser_interactive_frame() -> Self {
        Self {
            rects: 1_400,
            hits: 600,
            drag_sources: 96,
            drop_targets: 96,
            transitions: 160,
            icon_quads: 800,
            text_quads: 5_000,
        }
    }

    pub const fn public_showcase_frame() -> Self {
        Self {
            rects: 1_000,
            hits: 420,
            drag_sources: 80,
            drop_targets: 80,
            transitions: 120,
            icon_quads: 640,
            text_quads: 3_600,
        }
    }
}

impl GpuSceneStats {
    pub fn fits_budget(self, budget: GpuSceneBudget) -> bool {
        self.first_budget_violation(budget).is_none()
    }

    pub fn first_budget_violation(self, budget: GpuSceneBudget) -> Option<GpuSceneBudgetViolation> {
        [
            ("rects", self.rects, budget.rects),
            ("hits", self.hits, budget.hits),
            ("drag_sources", self.drag_sources, budget.drag_sources),
            ("drop_targets", self.drop_targets, budget.drop_targets),
            ("transitions", self.transitions, budget.transitions),
            ("icon_quads", self.icon_quads, budget.icon_quads),
            ("text_quads", self.text_quads, budget.text_quads),
        ]
        .into_iter()
        .find_map(|(name, actual, limit)| {
            (actual > limit).then_some(GpuSceneBudgetViolation {
                name,
                actual,
                limit,
            })
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuSceneBudgetViolation {
    pub name: &'static str,
    pub actual: usize,
    pub limit: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuFrameTimeBudget {
    pub build_ms: f32,
    pub render_ms: f32,
}

impl GpuFrameTimeBudget {
    pub const fn native_interactive_frame() -> Self {
        Self {
            build_ms: 3.0,
            render_ms: 5.0,
        }
    }

    pub const fn browser_interactive_frame() -> Self {
        Self {
            build_ms: 4.0,
            render_ms: 6.0,
        }
    }

    pub const fn public_showcase_frame() -> Self {
        Self {
            build_ms: 4.0,
            render_ms: 6.0,
        }
    }

    pub fn fits(self, build_ms: f32, render_ms: f32) -> bool {
        build_ms <= self.build_ms && render_ms <= self.render_ms
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiTransitionProperty {
    Opacity,
    TranslateX,
    TranslateY,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiTransitionEasing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl UiTransitionEasing {
    pub fn sample(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Self::EaseInOut if t < 0.5 => 2.0 * t * t,
            Self::EaseInOut => 1.0 - (-2.0 * t + 2.0).powi(2) * 0.5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiTransitionSpec {
    pub id: u32,
    pub property: UiTransitionProperty,
    pub from: f32,
    pub to: f32,
    pub duration_ms: u32,
    pub delay_ms: u32,
    pub easing: UiTransitionEasing,
}

impl UiTransitionSpec {
    pub const fn new(
        id: u32,
        property: UiTransitionProperty,
        from: f32,
        to: f32,
        duration_ms: u32,
        delay_ms: u32,
        easing: UiTransitionEasing,
    ) -> Self {
        Self {
            id,
            property,
            from,
            to,
            duration_ms,
            delay_ms,
            easing,
        }
    }

    pub const fn opacity(id: u32, from: f32, to: f32, duration_ms: u32) -> Self {
        Self::new(
            id,
            UiTransitionProperty::Opacity,
            from,
            to,
            duration_ms,
            0,
            UiTransitionEasing::EaseOut,
        )
    }

    pub const fn translate_x(id: u32, from: f32, to: f32, duration_ms: u32) -> Self {
        Self::new(
            id,
            UiTransitionProperty::TranslateX,
            from,
            to,
            duration_ms,
            0,
            UiTransitionEasing::EaseOut,
        )
    }

    pub const fn translate_y(id: u32, from: f32, to: f32, duration_ms: u32) -> Self {
        Self::new(
            id,
            UiTransitionProperty::TranslateY,
            from,
            to,
            duration_ms,
            0,
            UiTransitionEasing::EaseOut,
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GpuSceneCursor {
    rects: usize,
    hits: usize,
    drag_sources: usize,
    drop_targets: usize,
    transitions: usize,
    #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
    icon_quads: usize,
    #[cfg(feature = "fontdue-text")]
    text_quads: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuSceneDebugBatch {
    pub name: String,
    pub layer: GpuSceneRenderLayer,
    pub start: usize,
    pub len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuDragSource {
    /// Reorder namespace. Sources and targets only match inside the same scope.
    pub scope_id: u32,
    /// Stable item identity returned in reorder actions.
    pub item_id: u32,
    /// Current visual order for this frame.
    pub index: usize,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl GpuDragSource {
    pub const fn new(
        scope_id: u32,
        item_id: u32,
        index: usize,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) -> Self {
        Self {
            scope_id,
            item_id,
            index,
            x,
            y,
            w,
            h,
        }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x <= self.x + self.w && y <= self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuDropTarget {
    /// Reorder namespace. Sources and targets only match inside the same scope.
    pub scope_id: u32,
    /// Target insertion index for the active frame.
    pub index: usize,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl GpuDropTarget {
    pub const fn new(scope_id: u32, index: usize, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            scope_id,
            index,
            x,
            y,
            w,
            h,
        }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x <= self.x + self.w && y <= self.y + self.h
    }
}

impl GpuClip {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn intersect(self, other: Self) -> Option<Self> {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = (self.x + self.w).min(other.x + other.w);
        let y1 = (self.y + self.h).min(other.y + other.h);
        let w = x1 - x0;
        let h = y1 - y0;
        (w > 0.0 && h > 0.0).then_some(Self::new(x0, y0, w, h))
    }
}

impl GpuRect {
    pub const fn fill(x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius,
            color,
            color2: color,
            mode: RectMode::Fill,
            shadow: 0.0,
        }
    }

    pub const fn linear_gradient(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        from: Color4,
        to: Color4,
    ) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius,
            color: from,
            color2: to,
            mode: RectMode::LinearGradient,
            shadow: 0.0,
        }
    }

    pub const fn shadow(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        color: Color4,
        shadow: f32,
    ) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius,
            color,
            color2: color,
            mode: RectMode::Shadow,
            shadow,
        }
    }

    pub const fn border(x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius,
            color,
            color2: color,
            mode: RectMode::Border,
            shadow: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GpuScene {
    pub clear: Color4,
    rects: Vec<GpuRect>,
    hits: Vec<GpuHit>,
    drag_sources: Vec<GpuDragSource>,
    drop_targets: Vec<GpuDropTarget>,
    transitions: Vec<UiTransitionSpec>,
    clip_stack: Vec<GpuClip>,
    #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
    icon_quads: Vec<IconQuad>,
    #[cfg(feature = "fontdue-text")]
    text_quads: Vec<TextQuad>,
    debug_batches: Vec<GpuSceneDebugBatch>,
}

impl GpuScene {
    pub fn new(clear: Color4) -> Self {
        Self {
            clear,
            rects: Vec::new(),
            hits: Vec::new(),
            drag_sources: Vec::new(),
            drop_targets: Vec::new(),
            transitions: Vec::new(),
            clip_stack: Vec::new(),
            #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
            icon_quads: Vec::new(),
            #[cfg(feature = "fontdue-text")]
            text_quads: Vec::new(),
            debug_batches: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.clear_commands();
    }

    pub fn clear_commands(&mut self) {
        self.rects.clear();
        self.hits.clear();
        self.drag_sources.clear();
        self.drop_targets.clear();
        self.transitions.clear();
        self.clip_stack.clear();
        #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
        self.icon_quads.clear();
        #[cfg(feature = "fontdue-text")]
        self.text_quads.clear();
        self.debug_batches.clear();
    }

    pub fn clear_rects(&mut self) {
        self.clear_commands();
    }

    pub fn push_rect(&mut self, rect: GpuRect) {
        if let Some(rect) = self.clip_rect(rect) {
            self.rects.push(rect);
        }
    }

    pub fn push_hit(&mut self, hit: GpuHit) {
        if let Some(hit) = self.clip_hit(hit) {
            self.hits.push(hit);
        }
    }

    pub fn push_drag_source(&mut self, source: GpuDragSource) {
        if let Some(source) = self.clip_drag_source(source) {
            self.drag_sources.push(source);
        }
    }

    pub fn push_drop_target(&mut self, target: GpuDropTarget) {
        if let Some(target) = self.clip_drop_target(target) {
            self.drop_targets.push(target);
        }
    }

    pub fn push_transition(&mut self, transition: UiTransitionSpec) {
        if valid_transition(transition) {
            self.transitions.push(transition);
        }
    }

    pub fn cursor(&self) -> GpuSceneCursor {
        GpuSceneCursor {
            rects: self.rects.len(),
            hits: self.hits.len(),
            drag_sources: self.drag_sources.len(),
            drop_targets: self.drop_targets.len(),
            transitions: self.transitions.len(),
            #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
            icon_quads: self.icon_quads.len(),
            #[cfg(feature = "fontdue-text")]
            text_quads: self.text_quads.len(),
        }
    }

    pub fn apply_opacity_since(&mut self, cursor: GpuSceneCursor, opacity: f32) {
        let opacity = opacity.clamp(0.0, 1.0);
        for rect in &mut self.rects[cursor.rects..] {
            rect.color.a *= opacity;
        }
        #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
        for quad in &mut self.icon_quads[cursor.icon_quads..] {
            quad.color.a *= opacity;
        }
        #[cfg(feature = "fontdue-text")]
        for quad in &mut self.text_quads[cursor.text_quads..] {
            quad.color.a *= opacity;
        }
    }

    pub fn label_commands_since(&mut self, cursor: GpuSceneCursor, name: &str) {
        if name.is_empty() {
            return;
        }
        self.push_debug_batch(
            name,
            GpuSceneRenderLayer::Rects,
            cursor.rects,
            self.rects.len().saturating_sub(cursor.rects),
        );
        #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
        self.push_debug_batch(
            name,
            GpuSceneRenderLayer::IconQuads,
            cursor.icon_quads,
            self.icon_quads.len().saturating_sub(cursor.icon_quads),
        );
        #[cfg(feature = "fontdue-text")]
        self.push_debug_batch(
            name,
            GpuSceneRenderLayer::TextQuads,
            cursor.text_quads,
            self.text_quads.len().saturating_sub(cursor.text_quads),
        );
    }

    pub fn translate_since(&mut self, cursor: GpuSceneCursor, dx: f32, dy: f32) {
        if !dx.is_finite() || !dy.is_finite() {
            return;
        }
        for rect in &mut self.rects[cursor.rects..] {
            rect.x += dx;
            rect.y += dy;
        }
        for hit in &mut self.hits[cursor.hits..] {
            hit.x += dx;
            hit.y += dy;
        }
        for source in &mut self.drag_sources[cursor.drag_sources..] {
            source.x += dx;
            source.y += dy;
        }
        for target in &mut self.drop_targets[cursor.drop_targets..] {
            target.x += dx;
            target.y += dy;
        }
        #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
        for quad in &mut self.icon_quads[cursor.icon_quads..] {
            quad.x += dx;
            quad.y += dy;
        }
        #[cfg(feature = "fontdue-text")]
        for quad in &mut self.text_quads[cursor.text_quads..] {
            quad.x += dx;
            quad.y += dy;
        }
    }

    pub fn hit_count(&self) -> usize {
        self.hits.len()
    }

    pub fn stats(&self) -> GpuSceneStats {
        GpuSceneStats {
            rects: self.rects.len(),
            hits: self.hits.len(),
            drag_sources: self.drag_sources.len(),
            drop_targets: self.drop_targets.len(),
            transitions: self.transitions.len(),
            clips: self.clip_stack.len(),
            #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
            icon_quads: self.icon_quads.len(),
            #[cfg(not(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas")))]
            icon_quads: 0,
            #[cfg(feature = "fontdue-text")]
            text_quads: self.text_quads.len(),
            #[cfg(not(feature = "fontdue-text"))]
            text_quads: 0,
        }
    }

    pub fn debug_batches(&self) -> &[GpuSceneDebugBatch] {
        &self.debug_batches
    }

    pub const fn z_order() -> GpuSceneZOrder {
        GpuSceneZOrder::canonical()
    }

    pub const fn blend_contract() -> GpuSceneBlendContract {
        GpuSceneBlendContract::canonical()
    }

    pub fn debug_dump(&self) -> String {
        let stats = self.stats();
        let mut out = String::new();
        let _ = writeln!(
            out,
            "GpuScene rects={} hits={} drag_sources={} drop_targets={} transitions={} clips={} icon_quads={} text_quads={}",
            stats.rects,
            stats.hits,
            stats.drag_sources,
            stats.drop_targets,
            stats.transitions,
            stats.clips,
            stats.icon_quads,
            stats.text_quads
        );
        let _ = writeln!(
            out,
            "clear rgba({:.3},{:.3},{:.3},{:.3})",
            self.clear.r, self.clear.g, self.clear.b, self.clear.a
        );
        for (index, rect) in self.rects.iter().take(12).enumerate() {
            let _ = writeln!(
                out,
                "rect[{index}] {:?} x={:.1} y={:.1} w={:.1} h={:.1} r={:.1}",
                rect.mode, rect.x, rect.y, rect.w, rect.h, rect.radius
            );
        }
        for (index, hit) in self.hits.iter().take(12).enumerate() {
            let _ = writeln!(
                out,
                "hit[{index}] {:?} id={} x={:.1} y={:.1} w={:.1} h={:.1}",
                hit.kind, hit.id, hit.x, hit.y, hit.w, hit.h
            );
        }
        for (index, source) in self.drag_sources.iter().take(12).enumerate() {
            let _ = writeln!(
                out,
                "drag_source[{index}] scope={} item={} index={} x={:.1} y={:.1} w={:.1} h={:.1}",
                source.scope_id,
                source.item_id,
                source.index,
                source.x,
                source.y,
                source.w,
                source.h
            );
        }
        for (index, target) in self.drop_targets.iter().take(12).enumerate() {
            let _ = writeln!(
                out,
                "drop_target[{index}] scope={} index={} x={:.1} y={:.1} w={:.1} h={:.1}",
                target.scope_id, target.index, target.x, target.y, target.w, target.h
            );
        }
        for (index, transition) in self.transitions.iter().take(12).enumerate() {
            let _ = writeln!(
                out,
                "transition[{index}] id={} {:?} {:.3}->{:.3} duration={} delay={} {:?}",
                transition.id,
                transition.property,
                transition.from,
                transition.to,
                transition.duration_ms,
                transition.delay_ms,
                transition.easing
            );
        }
        for (index, batch) in self.debug_batches.iter().take(12).enumerate() {
            let _ = writeln!(
                out,
                "batch[{index}] {:?} name={} start={} len={}",
                batch.layer, batch.name, batch.start, batch.len
            );
        }
        out
    }

    pub fn truncate_hits(&mut self, len: usize) {
        self.hits.truncate(len);
    }

    pub fn push_clip(&mut self, clip: GpuClip) -> bool {
        let next = if let Some(current) = self.current_clip() {
            current.intersect(clip)
        } else if clip.w > 0.0 && clip.h > 0.0 {
            Some(clip)
        } else {
            None
        };
        if let Some(next) = next {
            self.clip_stack.push(next);
            true
        } else {
            false
        }
    }

    pub fn pop_clip(&mut self) {
        self.clip_stack.pop();
    }

    pub fn current_clip(&self) -> Option<GpuClip> {
        self.clip_stack.last().copied()
    }

    fn clip_rect(&self, rect: GpuRect) -> Option<GpuRect> {
        let mut rect = normalize_rect(rect)?;
        let Some(clip) = self.current_clip() else {
            return Some(rect);
        };
        let clipped = GpuClip::new(rect.x, rect.y, rect.w, rect.h).intersect(clip)?;
        rect.x = clipped.x;
        rect.y = clipped.y;
        rect.w = clipped.w;
        rect.h = clipped.h;
        rect.radius = rect.radius.min(rect.w * 0.5).min(rect.h * 0.5);
        Some(rect)
    }

    fn push_debug_batch(
        &mut self,
        name: &str,
        layer: GpuSceneRenderLayer,
        start: usize,
        len: usize,
    ) {
        if len == 0 {
            return;
        }
        self.debug_batches.push(GpuSceneDebugBatch {
            name: name.to_string(),
            layer,
            start,
            len,
        });
    }

    fn clip_hit(&self, mut hit: GpuHit) -> Option<GpuHit> {
        let Some(clip) = self.current_clip() else {
            return (hit.w > 0.0 && hit.h > 0.0).then_some(hit);
        };
        let clipped = GpuClip::new(hit.x, hit.y, hit.w, hit.h).intersect(clip)?;
        hit.x = clipped.x;
        hit.y = clipped.y;
        hit.w = clipped.w;
        hit.h = clipped.h;
        Some(hit)
    }

    fn clip_drag_source(&self, mut source: GpuDragSource) -> Option<GpuDragSource> {
        let Some(clip) = self.current_clip() else {
            return valid_interaction_rect(source.x, source.y, source.w, source.h)
                .then_some(source);
        };
        let clipped = GpuClip::new(source.x, source.y, source.w, source.h).intersect(clip)?;
        source.x = clipped.x;
        source.y = clipped.y;
        source.w = clipped.w;
        source.h = clipped.h;
        Some(source)
    }

    fn clip_drop_target(&self, mut target: GpuDropTarget) -> Option<GpuDropTarget> {
        let Some(clip) = self.current_clip() else {
            return valid_interaction_rect(target.x, target.y, target.w, target.h)
                .then_some(target);
        };
        let clipped = GpuClip::new(target.x, target.y, target.w, target.h).intersect(clip)?;
        target.x = clipped.x;
        target.y = clipped.y;
        target.w = clipped.w;
        target.h = clipped.h;
        Some(target)
    }

    pub fn push_text(&mut self, mut x: f32, y: f32, text: &str, scale: f32, color: Color4) {
        let cell = scale.max(1.0);
        let step = cell * 6.0;
        let start_x = x;
        for ch in text.chars() {
            match ch {
                '\n' => {
                    x = start_x;
                }
                '\r' => {}
                ' ' => x += step,
                _ => {
                    let glyph = super::bitmap_font::glyph5x7(ch);
                    for (row, bits) in glyph.iter().copied().enumerate() {
                        for col in 0..5 {
                            if ((bits >> (4 - col)) & 1) == 0 {
                                continue;
                            }
                            self.push_rect(GpuRect::fill(
                                x + col as f32 * cell,
                                y + row as f32 * cell,
                                cell,
                                cell,
                                0.0,
                                color,
                            ));
                        }
                    }
                    x += step;
                }
            }
        }
    }

    pub fn rects(&self) -> &[GpuRect] {
        &self.rects
    }

    pub fn hits(&self) -> &[GpuHit] {
        &self.hits
    }

    pub fn drag_sources(&self) -> &[GpuDragSource] {
        &self.drag_sources
    }

    pub fn drop_targets(&self) -> &[GpuDropTarget] {
        &self.drop_targets
    }

    pub fn transitions(&self) -> &[UiTransitionSpec] {
        &self.transitions
    }

    pub fn drag_source_at(&self, x: f32, y: f32) -> Option<GpuDragSource> {
        self.drag_sources
            .iter()
            .rev()
            .copied()
            .find(|source| source.contains(x, y))
    }

    pub fn drop_target_at(&self, x: f32, y: f32, scope_id: u32) -> Option<GpuDropTarget> {
        self.drop_targets
            .iter()
            .rev()
            .copied()
            .find(|target| target.scope_id == scope_id && target.contains(x, y))
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<GpuHit> {
        self.hits
            .iter()
            .rev()
            .copied()
            .find(|hit| hit.contains(x, y))
    }

    pub fn apply_color_scheme(&mut self, scheme: UiColorScheme) {
        if scheme == UiColorScheme::Dark {
            return;
        }
        let from = SchemePalette::dark();
        let to = scheme.palette();
        self.clear = remap_scheme_color(self.clear, from, to);
        for rect in &mut self.rects {
            rect.color = remap_scheme_color(rect.color, from, to);
        }
        #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
        for quad in &mut self.icon_quads {
            quad.color = remap_scheme_color(quad.color, from, to);
        }
        #[cfg(feature = "fontdue-text")]
        for quad in &mut self.text_quads {
            quad.color = remap_scheme_color(quad.color, from, to);
        }
    }

    #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
    pub fn push_icon_quad(&mut self, rect: super::UiRect, atlas: UiIconAtlasRect, color: Color4) {
        let quad = IconQuad {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
            u0: atlas.u0,
            v0: atlas.v0,
            u1: atlas.u1,
            v1: atlas.v1,
            color,
        };
        if let Some(quad) = self.clip_icon_quad(quad) {
            self.icon_quads.push(quad);
        }
    }

    #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
    fn clip_icon_quad(&self, mut quad: IconQuad) -> Option<IconQuad> {
        if !valid_quad_geometry(quad.x, quad.y, quad.w, quad.h) {
            return None;
        }
        let Some(clip) = self.current_clip() else {
            return Some(quad);
        };
        let x0 = quad.x;
        let y0 = quad.y;
        let x1 = quad.x + quad.w;
        let y1 = quad.y + quad.h;
        let clipped = GpuClip::new(quad.x, quad.y, quad.w, quad.h).intersect(clip)?;
        let u_span = quad.u1 - quad.u0;
        let v_span = quad.v1 - quad.v0;
        let left = ((clipped.x - x0) / (x1 - x0)).clamp(0.0, 1.0);
        let top = ((clipped.y - y0) / (y1 - y0)).clamp(0.0, 1.0);
        let right = ((clipped.x + clipped.w - x0) / (x1 - x0)).clamp(0.0, 1.0);
        let bottom = ((clipped.y + clipped.h - y0) / (y1 - y0)).clamp(0.0, 1.0);
        quad.x = clipped.x;
        quad.y = clipped.y;
        quad.w = clipped.w;
        quad.h = clipped.h;
        quad.u1 = quad.u0 + u_span * right;
        quad.v1 = quad.v0 + v_span * bottom;
        quad.u0 += u_span * left;
        quad.v0 += v_span * top;
        Some(quad)
    }

    #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
    pub fn icon_quads(&self) -> &[IconQuad] {
        &self.icon_quads
    }

    #[cfg(feature = "fontdue-text")]
    pub fn push_font_text(&mut self, atlas: &FontAtlas, x: f32, y: f32, text: &str, color: Color4) {
        atlas.layout_text(self, x, y, text, color);
    }

    #[cfg(feature = "fontdue-text")]
    pub(super) fn push_text_quad(&mut self, quad: TextQuad) {
        if let Some(quad) = self.clip_text_quad(quad) {
            self.text_quads.push(quad);
        }
    }

    #[cfg(feature = "fontdue-text")]
    fn clip_text_quad(&self, mut quad: TextQuad) -> Option<TextQuad> {
        if !valid_quad_geometry(quad.x, quad.y, quad.w, quad.h) {
            return None;
        }
        let Some(clip) = self.current_clip() else {
            return Some(quad);
        };
        let x0 = quad.x;
        let y0 = quad.y;
        let x1 = quad.x + quad.w;
        let y1 = quad.y + quad.h;
        let clipped = GpuClip::new(quad.x, quad.y, quad.w, quad.h).intersect(clip)?;
        let u_span = quad.u1 - quad.u0;
        let v_span = quad.v1 - quad.v0;
        let left = ((clipped.x - x0) / (x1 - x0)).clamp(0.0, 1.0);
        let top = ((clipped.y - y0) / (y1 - y0)).clamp(0.0, 1.0);
        let right = ((clipped.x + clipped.w - x0) / (x1 - x0)).clamp(0.0, 1.0);
        let bottom = ((clipped.y + clipped.h - y0) / (y1 - y0)).clamp(0.0, 1.0);
        quad.x = clipped.x;
        quad.y = clipped.y;
        quad.w = clipped.w;
        quad.h = clipped.h;
        quad.u1 = quad.u0 + u_span * right;
        quad.v1 = quad.v0 + v_span * bottom;
        quad.u0 += u_span * left;
        quad.v0 += v_span * top;
        Some(quad)
    }

    #[cfg(feature = "fontdue-text")]
    pub fn text_quads(&self) -> &[TextQuad] {
        &self.text_quads
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiColorScheme {
    #[default]
    Dark,
    Light,
    Terminal,
}

impl UiColorScheme {
    pub const fn from_code(code: u32) -> Self {
        match code {
            1 => Self::Light,
            2 => Self::Terminal,
            _ => Self::Dark,
        }
    }

    pub const fn code(self) -> u32 {
        match self {
            Self::Dark => 0,
            Self::Light => 1,
            Self::Terminal => 2,
        }
    }

    fn palette(self) -> SchemePalette {
        match self {
            Self::Dark => SchemePalette::dark(),
            Self::Light => SchemePalette::light(),
            Self::Terminal => SchemePalette::terminal(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SchemePalette {
    bg: Color4,
    sidebar: Color4,
    topbar: Color4,
    panel: Color4,
    row: Color4,
    active: Color4,
    composer: Color4,
    text: Color4,
    muted: Color4,
    border: Color4,
    accent: Color4,
    accent_text: Color4,
    green: Color4,
    violet: Color4,
    amber: Color4,
    danger: Color4,
}

impl SchemePalette {
    const fn dark() -> Self {
        Self {
            bg: super::palette::BG,
            sidebar: super::palette::SIDEBAR,
            topbar: super::palette::TOPBAR,
            panel: super::palette::PANEL,
            row: super::palette::ROW,
            active: super::palette::ACTIVE_ROW,
            composer: super::palette::COMPOSER,
            text: super::palette::TEXT,
            muted: super::palette::MUTED,
            border: super::palette::BORDER,
            accent: super::palette::ACCENT,
            accent_text: super::palette::ACCENT_TEXT,
            green: super::palette::GREEN,
            violet: super::palette::VIOLET,
            amber: super::palette::AMBER,
            danger: super::palette::DANGER,
        }
    }

    const fn light() -> Self {
        Self {
            bg: Color4::rgba(0.965, 0.965, 0.94, 1.0),
            sidebar: Color4::rgba(0.92, 0.925, 0.91, 1.0),
            topbar: Color4::rgba(0.985, 0.985, 0.965, 1.0),
            panel: Color4::rgba(1.0, 1.0, 0.98, 1.0),
            row: Color4::rgba(0.925, 0.93, 0.91, 1.0),
            active: Color4::rgba(0.86, 0.91, 0.92, 1.0),
            composer: Color4::rgba(0.955, 0.955, 0.94, 1.0),
            text: Color4::rgba(0.07, 0.075, 0.07, 1.0),
            muted: Color4::rgba(0.32, 0.33, 0.31, 1.0),
            border: Color4::rgba(0.74, 0.75, 0.71, 1.0),
            accent: Color4::rgba(0.0, 0.48, 0.54, 1.0),
            accent_text: Color4::rgba(0.98, 1.0, 1.0, 1.0),
            green: Color4::rgba(0.0, 0.48, 0.32, 1.0),
            violet: Color4::rgba(0.43, 0.28, 0.62, 1.0),
            amber: Color4::rgba(0.68, 0.42, 0.0, 1.0),
            danger: Color4::rgba(0.72, 0.15, 0.12, 1.0),
        }
    }

    const fn terminal() -> Self {
        Self {
            bg: Color4::rgba(0.0, 0.03, 0.018, 1.0),
            sidebar: Color4::rgba(0.0, 0.07, 0.04, 1.0),
            topbar: Color4::rgba(0.0, 0.09, 0.055, 1.0),
            panel: Color4::rgba(0.0, 0.065, 0.04, 1.0),
            row: Color4::rgba(0.0, 0.11, 0.065, 1.0),
            active: Color4::rgba(0.02, 0.20, 0.11, 1.0),
            composer: Color4::rgba(0.0, 0.085, 0.052, 1.0),
            text: Color4::rgba(0.63, 1.0, 0.72, 1.0),
            muted: Color4::rgba(0.32, 0.67, 0.43, 1.0),
            border: Color4::rgba(0.08, 0.37, 0.17, 1.0),
            accent: Color4::rgba(0.0, 0.92, 0.42, 1.0),
            accent_text: Color4::rgba(0.0, 0.025, 0.015, 1.0),
            green: Color4::rgba(0.05, 0.90, 0.36, 1.0),
            violet: Color4::rgba(0.48, 0.90, 0.68, 1.0),
            amber: Color4::rgba(0.78, 0.95, 0.32, 1.0),
            danger: Color4::rgba(1.0, 0.24, 0.20, 1.0),
        }
    }
}

fn remap_scheme_color(color: Color4, from: SchemePalette, to: SchemePalette) -> Color4 {
    const EPS: f32 = 0.002;
    let candidates = [
        (from.bg, to.bg),
        (from.sidebar, to.sidebar),
        (from.topbar, to.topbar),
        (from.panel, to.panel),
        (from.row, to.row),
        (from.active, to.active),
        (from.composer, to.composer),
        (from.text, to.text),
        (from.muted, to.muted),
        (from.border, to.border),
        (from.accent, to.accent),
        (from.accent_text, to.accent_text),
        (from.green, to.green),
        (from.violet, to.violet),
        (from.amber, to.amber),
        (from.danger, to.danger),
    ];
    for (source, target) in candidates {
        if (color.r - source.r).abs() < EPS
            && (color.g - source.g).abs() < EPS
            && (color.b - source.b).abs() < EPS
        {
            return target.with_alpha(color.a);
        }
    }
    color
}

#[cfg(test)]
fn source_over_straight_alpha(src: Color4, dst: Color4) -> Color4 {
    let src_a = src.a.clamp(0.0, 1.0);
    let dst_a = dst.a.clamp(0.0, 1.0);
    let out_a = src_a + dst_a * (1.0 - src_a);
    if out_a <= f32::EPSILON {
        return Color4::rgba(0.0, 0.0, 0.0, 0.0);
    }

    let dst_scale = dst_a * (1.0 - src_a);
    Color4::rgba(
        (src.r * src_a + dst.r * dst_scale) / out_a,
        (src.g * src_a + dst.g * dst_scale) / out_a,
        (src.b * src_a + dst.b * dst_scale) / out_a,
        out_a,
    )
}

fn normalize_rect(mut rect: GpuRect) -> Option<GpuRect> {
    if !rect.x.is_finite()
        || !rect.y.is_finite()
        || !rect.w.is_finite()
        || !rect.h.is_finite()
        || !rect.radius.is_finite()
        || !rect.shadow.is_finite()
        || rect.w <= 0.0
        || rect.h <= 0.0
    {
        return None;
    }
    rect.radius = rect.radius.max(0.0).min(rect.w * 0.5).min(rect.h * 0.5);
    rect.shadow = rect.shadow.max(0.0);
    Some(rect)
}

fn valid_interaction_rect(x: f32, y: f32, w: f32, h: f32) -> bool {
    x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0
}

fn valid_transition(transition: UiTransitionSpec) -> bool {
    transition.from.is_finite()
        && transition.to.is_finite()
        && transition.duration_ms > 0
        && transition.duration_ms <= 60_000
}

#[cfg(any(
    feature = "fontdue-text",
    any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas")
))]
fn valid_quad_geometry(x: f32, y: f32, w: f32, h: f32) -> bool {
    x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::{HitKind, UiRect, palette};

    #[test]
    fn scene_stats_track_public_buffers() {
        let mut scene = GpuScene::new(palette::BG);

        scene.push_rect(GpuRect::fill(0.0, 0.0, 10.0, 10.0, 0.0, palette::TEXT));
        scene.push_hit(GpuHit::new(HitKind::Button, 7, 1.0, 1.0, 8.0, 8.0));
        scene.push_transition(UiTransitionSpec::opacity(90, 0.0, 1.0, 120));
        assert!(scene.push_clip(GpuClip::new(0.0, 0.0, 5.0, 5.0)));

        assert_eq!(
            scene.stats(),
            GpuSceneStats {
                rects: 1,
                hits: 1,
                drag_sources: 0,
                drop_targets: 0,
                transitions: 1,
                clips: 1,
                icon_quads: 0,
                text_quads: 0,
            }
        );

        scene.clear();
        assert_eq!(scene.stats(), GpuSceneStats::default());
    }

    #[test]
    fn scene_z_order_documents_renderer_and_interaction_order() {
        let z_order = GpuScene::z_order();

        assert_eq!(
            z_order.render_layers,
            &[
                GpuSceneRenderLayer::Clear,
                GpuSceneRenderLayer::Rects,
                GpuSceneRenderLayer::IconQuads,
                GpuSceneRenderLayer::TextQuads,
            ]
        );
        assert_eq!(
            z_order.interaction_layers,
            &[
                GpuSceneInteractionLayer::Hits,
                GpuSceneInteractionLayer::DragSources,
                GpuSceneInteractionLayer::DropTargets,
            ]
        );
        assert!(z_order.topmost_interaction_is_last);
    }

    #[test]
    fn scene_blend_contract_keeps_text_and_icons_above_rects() {
        let blend = GpuScene::blend_contract();

        assert_eq!(blend.mode, GpuSceneBlendMode::SourceOverStraightAlpha);
        assert_eq!(blend.render_layers, GPU_SCENE_RENDER_ORDER);
        assert!(blend.text_and_icons_are_over_rects);
        assert!(
            blend
                .render_layers
                .iter()
                .position(|layer| *layer == GpuSceneRenderLayer::Rects)
                < blend
                    .render_layers
                    .iter()
                    .position(|layer| *layer == GpuSceneRenderLayer::IconQuads)
        );
        assert!(
            blend
                .render_layers
                .iter()
                .position(|layer| *layer == GpuSceneRenderLayer::Rects)
                < blend
                    .render_layers
                    .iter()
                    .position(|layer| *layer == GpuSceneRenderLayer::TextQuads)
        );
    }

    #[test]
    fn source_over_alpha_keeps_glyph_pixels_visible_on_translucent_panels() {
        let clear = Color4::rgba(0.01, 0.012, 0.015, 1.0);
        let panel = Color4::rgba(0.18, 0.20, 0.23, 0.72);
        let glyph_pixel = Color4::rgba(0.94, 0.96, 0.99, 0.82);

        let panel_over_clear = source_over_straight_alpha(panel, clear);
        let glyph_over_panel = source_over_straight_alpha(glyph_pixel, panel_over_clear);

        assert!(luminance(glyph_over_panel) > luminance(panel_over_clear));
        assert!(glyph_over_panel.a >= panel_over_clear.a);
    }

    #[test]
    fn source_over_alpha_proves_translucent_layer_order_is_observable() {
        let panel = Color4::rgba(0.10, 0.12, 0.16, 0.65);
        let glyph_pixel = Color4::rgba(0.92, 0.94, 0.98, 0.58);

        let glyph_over_panel = source_over_straight_alpha(glyph_pixel, panel);
        let panel_over_glyph = source_over_straight_alpha(panel, glyph_pixel);

        assert!((glyph_over_panel.r - panel_over_glyph.r).abs() > 0.1);
        assert!((glyph_over_panel.g - panel_over_glyph.g).abs() > 0.1);
        assert!((glyph_over_panel.b - panel_over_glyph.b).abs() > 0.1);
    }

    #[test]
    fn scene_hit_and_drag_queries_treat_last_inserted_item_as_topmost() {
        let mut scene = GpuScene::new(palette::BG);

        scene.push_hit(GpuHit::new(HitKind::Button, 1, 0.0, 0.0, 30.0, 30.0));
        scene.push_hit(GpuHit::new(HitKind::Button, 2, 0.0, 0.0, 30.0, 30.0));
        scene.push_drag_source(GpuDragSource::new(10, 1, 0, 0.0, 0.0, 30.0, 30.0));
        scene.push_drag_source(GpuDragSource::new(10, 2, 1, 0.0, 0.0, 30.0, 30.0));
        scene.push_drop_target(GpuDropTarget::new(10, 0, 0.0, 0.0, 30.0, 30.0));
        scene.push_drop_target(GpuDropTarget::new(10, 1, 0.0, 0.0, 30.0, 30.0));

        assert_eq!(scene.hit_test(4.0, 4.0).expect("hit").id, 2);
        assert_eq!(
            scene.drag_source_at(4.0, 4.0).expect("drag source").item_id,
            2
        );
        assert_eq!(
            scene
                .drop_target_at(4.0, 4.0, 10)
                .expect("drop target")
                .index,
            1
        );
    }

    #[test]
    fn scene_stats_report_first_budget_violation() {
        let stats = GpuSceneStats {
            rects: 11,
            hits: 4,
            drag_sources: 0,
            drop_targets: 0,
            transitions: 0,
            clips: 0,
            icon_quads: 0,
            text_quads: 0,
        };
        let budget = GpuSceneBudget {
            rects: 10,
            hits: 8,
            drag_sources: 1,
            drop_targets: 1,
            transitions: 1,
            icon_quads: 1,
            text_quads: 1,
        };

        assert_eq!(
            stats.first_budget_violation(budget),
            Some(GpuSceneBudgetViolation {
                name: "rects",
                actual: 11,
                limit: 10,
            })
        );
        assert!(!stats.fits_budget(budget));
        assert!(stats.fits_budget(GpuSceneBudget::native_interactive_frame()));
    }

    #[test]
    fn frame_time_budgets_keep_build_and_render_limits_separate() {
        let budget = GpuFrameTimeBudget::browser_interactive_frame();

        assert!(budget.fits(3.5, 5.5));
        assert!(!budget.fits(4.5, 5.5));
        assert!(!budget.fits(3.5, 6.5));
    }

    #[test]
    fn scene_debug_dump_reports_counts_and_samples() {
        let mut scene = GpuScene::new(palette::BG);
        let cursor = scene.cursor();
        scene.push_rect(GpuRect::fill(2.0, 3.0, 10.0, 11.0, 2.0, palette::TEXT));
        scene.push_hit(GpuHit::new(HitKind::Button, 42, 2.0, 3.0, 10.0, 11.0));
        scene.push_drag_source(GpuDragSource::new(3, 42, 0, 2.0, 3.0, 10.0, 11.0));
        scene.push_drop_target(GpuDropTarget::new(3, 1, 2.0, 16.0, 10.0, 11.0));
        scene.push_transition(UiTransitionSpec::translate_y(44, -8.0, 0.0, 160));
        scene.label_commands_since(cursor, "trust-manager-card");

        let dump = scene.debug_dump();

        assert!(
            dump.contains("GpuScene rects=1 hits=1 drag_sources=1 drop_targets=1 transitions=1")
        );
        assert!(dump.contains("rect[0] Fill x=2.0 y=3.0 w=10.0 h=11.0"));
        assert!(dump.contains("hit[0] Button id=42"));
        assert!(dump.contains("drag_source[0] scope=3 item=42 index=0"));
        assert!(dump.contains("drop_target[0] scope=3 index=1"));
        assert!(dump.contains("transition[0] id=44 TranslateY"));
        assert!(dump.contains("batch[0] Rects name=trust-manager-card start=0 len=1"));
    }

    #[test]
    fn scene_debug_batches_label_render_command_ranges_since_cursor() {
        let mut scene = GpuScene::new(palette::BG);
        scene.push_rect(GpuRect::fill(0.0, 0.0, 4.0, 4.0, 0.0, palette::BORDER));
        let cursor = scene.cursor();

        scene.push_rect(GpuRect::fill(8.0, 0.0, 4.0, 4.0, 0.0, palette::TEXT));
        scene.push_rect(GpuRect::fill(16.0, 0.0, 4.0, 4.0, 0.0, palette::TEXT));
        scene.label_commands_since(cursor, "proof-row");
        scene.label_commands_since(scene.cursor(), "empty-range");
        scene.label_commands_since(cursor, "");

        assert_eq!(
            scene.debug_batches(),
            &[GpuSceneDebugBatch {
                name: "proof-row".to_string(),
                layer: GpuSceneRenderLayer::Rects,
                start: 1,
                len: 2,
            }]
        );
    }

    #[test]
    fn scene_clear_commands_explicitly_clears_draw_and_interaction_state() {
        let mut scene = GpuScene::new(palette::BG);

        scene.push_rect(GpuRect::fill(0.0, 0.0, 8.0, 8.0, 0.0, palette::TEXT));
        scene.push_hit(GpuHit::new(HitKind::Button, 1, 0.0, 0.0, 8.0, 8.0));
        scene.push_drag_source(GpuDragSource::new(1, 1, 0, 0.0, 0.0, 8.0, 8.0));
        scene.push_drop_target(GpuDropTarget::new(1, 0, 0.0, 0.0, 8.0, 8.0));
        scene.push_transition(UiTransitionSpec::opacity(1, 0.0, 1.0, 100));
        assert!(scene.push_clip(GpuClip::new(0.0, 0.0, 4.0, 4.0)));
        scene.label_commands_since(GpuSceneCursor::default(), "clear-contract");

        scene.clear_commands();

        assert_eq!(scene.stats(), GpuSceneStats::default());
        assert_eq!(scene.clear, palette::BG);
        assert!(scene.debug_batches().is_empty());
    }

    #[test]
    fn scene_hit_tests_drag_sources_and_scope_bound_drop_targets() {
        let mut scene = GpuScene::new(palette::BG);
        scene.push_drag_source(GpuDragSource::new(7, 100, 0, 0.0, 0.0, 40.0, 40.0));
        scene.push_drag_source(GpuDragSource::new(7, 101, 1, 10.0, 10.0, 40.0, 40.0));
        scene.push_drop_target(GpuDropTarget::new(7, 0, 0.0, 0.0, 40.0, 40.0));
        scene.push_drop_target(GpuDropTarget::new(8, 0, 10.0, 10.0, 40.0, 40.0));

        assert_eq!(
            scene
                .drag_source_at(12.0, 12.0)
                .map(|source| source.item_id),
            Some(101)
        );
        assert_eq!(
            scene
                .drop_target_at(12.0, 12.0, 7)
                .map(|target| target.scope_id),
            Some(7)
        );
        assert_eq!(
            scene
                .drop_target_at(12.0, 12.0, 8)
                .map(|target| target.scope_id),
            Some(8)
        );
        assert_eq!(scene.drop_target_at(80.0, 80.0, 7), None);
    }

    #[test]
    fn scene_rejects_pathological_rects_and_clamps_radius() {
        let mut scene = GpuScene::new(palette::BG);

        scene.push_rect(GpuRect::fill(0.0, 0.0, 0.0, 10.0, 0.0, palette::TEXT));
        scene.push_rect(GpuRect::fill(0.0, 0.0, -10.0, 10.0, 0.0, palette::TEXT));
        scene.push_rect(GpuRect::fill(f32::NAN, 0.0, 10.0, 10.0, 0.0, palette::TEXT));
        scene.push_rect(GpuRect::fill(0.0, 0.0, 20.0, 10.0, 999.0, palette::TEXT));
        scene.push_rect(GpuRect::shadow(
            24.0,
            0.0,
            20.0,
            10.0,
            -4.0,
            palette::TEXT,
            -8.0,
        ));

        assert_eq!(scene.rects().len(), 2);
        assert_eq!(scene.rects()[0].radius, 5.0);
        assert_eq!(scene.rects()[1].radius, 0.0);
        assert_eq!(scene.rects()[1].shadow, 0.0);
    }

    #[test]
    fn scene_clips_rect_shadows_and_hits_to_current_clip() {
        let mut scene = GpuScene::new(palette::BG);

        assert!(scene.push_clip(GpuClip::new(5.0, 5.0, 10.0, 10.0)));
        scene.push_rect(GpuRect::shadow(
            0.0,
            0.0,
            20.0,
            20.0,
            12.0,
            palette::TEXT,
            6.0,
        ));
        scene.push_hit(GpuHit::new(HitKind::Button, 8, 0.0, 0.0, 20.0, 20.0));

        assert_eq!(scene.rects()[0].x, 5.0);
        assert_eq!(scene.rects()[0].y, 5.0);
        assert_eq!(scene.rects()[0].w, 10.0);
        assert_eq!(scene.rects()[0].h, 10.0);
        assert_eq!(scene.rects()[0].radius, 5.0);
        assert_eq!(scene.hits()[0].x, 5.0);
        assert_eq!(scene.hits()[0].y, 5.0);
        assert_eq!(scene.hits()[0].w, 10.0);
        assert_eq!(scene.hits()[0].h, 10.0);
    }

    #[cfg(feature = "fontdue-text")]
    #[test]
    fn scene_clips_text_quads_and_uvs_to_current_clip() {
        let mut scene = GpuScene::new(palette::BG);

        assert!(scene.push_clip(GpuClip::new(5.0, 0.0, 10.0, 10.0)));
        scene.push_text_quad(TextQuad {
            x: 0.0,
            y: 0.0,
            w: 20.0,
            h: 10.0,
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
            color: palette::TEXT,
        });
        scene.push_text_quad(TextQuad {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 10.0,
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
            color: palette::TEXT,
        });

        assert_eq!(scene.text_quads().len(), 1);
        let quad = scene.text_quads()[0];
        assert_eq!(quad.x, 5.0);
        assert_eq!(quad.w, 10.0);
        assert_eq!(quad.u0, 0.25);
        assert_eq!(quad.u1, 0.75);
    }

    #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
    #[test]
    fn scene_clips_icon_quads_and_uvs_to_current_clip() {
        let mut scene = GpuScene::new(palette::BG);

        assert!(scene.push_clip(GpuClip::new(5.0, 0.0, 10.0, 10.0)));
        scene.push_icon_quad(
            UiRect::new(0.0, 0.0, 20.0, 10.0),
            UiIconAtlasRect {
                name: "fixture",
                x: 0,
                y: 0,
                w: 20,
                h: 10,
                u0: 0.0,
                v0: 0.0,
                u1: 1.0,
                v1: 1.0,
            },
            palette::TEXT,
        );

        assert_eq!(scene.icon_quads().len(), 1);
        let quad = scene.icon_quads()[0];
        assert_eq!(quad.x, 5.0);
        assert_eq!(quad.w, 10.0);
        assert_eq!(quad.u0, 0.25);
        assert_eq!(quad.u1, 0.75);
    }

    fn luminance(color: Color4) -> f32 {
        color.r * 0.2126 + color.g * 0.7152 + color.b * 0.0722
    }
}
