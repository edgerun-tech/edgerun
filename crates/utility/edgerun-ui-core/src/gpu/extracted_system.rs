//! Single entry point for the extracted UI design system metadata.

use super::component_inventory::{
    EXTRACTED_COMPONENT_KINDS, EXTRACTED_PATTERN_KINDS, EXTRACTED_STATE_KINDS,
};
use super::extracted_blocks::{EXTRACTED_BLOCK_IDS, EXTRACTED_BLOCK_KINDS};
use super::source_captures::{UiExtractedSourceCapture, EXTRACTED_SOURCE_CAPTURES};
use super::style_family::EXTRACTED_STYLE_TOKEN_KINDS;
use super::{
    build_extracted_block, build_shadcn_component_preview,
    build_shadcn_component_preview_by_source_component, build_shadcn_demo_preview,
    UiExtractedBlockId, UiExtractedBlockKind, UiExtractedBlockSpec, UiExtractedComponentKind,
    UiExtractedComponentSpec, UiExtractedIconSpec, UiExtractedPatternKind, UiExtractedPatternSpec,
    UiExtractedSlotSpec, UiExtractedStateKind, UiExtractedStateSpec, UiExtractedStyleToken,
    UiExtractedStyleTokenKind, UiIcon, UiIconSet, UiNode, UiRect, UiShadcnDemoCategory,
    UiShadcnDemoSpec, UiShadcnDemoStatus, UiShadcnPortManifest, UiShadcnPortMapping,
    UiShadcnResolvedDemo, UiStyleFamily, UiStyleFamilySpec, EXTRACTED_BLOCKS, EXTRACTED_COMPONENTS,
    EXTRACTED_PATTERNS, EXTRACTED_SLOTS, EXTRACTED_SOURCE_ICONS, EXTRACTED_STATES,
    EXTRACTED_STYLE_TOKENS, SHADCN_DEMO_CATEGORIES, SHADCN_DEMO_COMPONENTS, SHADCN_DEMO_STATUSES,
    STYLE_FAMILY_SPECS,
};
use super::{
    build_shadcn_component_preview_by_identifier, build_shadcn_demo_preview_by_identifier,
    resolve_shadcn_demo_identifier, shadcn_port_manifest,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedDesignSystem {
    pub name: &'static str,
    pub source: &'static str,
    pub source_captures: &'static [UiExtractedSourceCapture],
    pub style_families: &'static [UiStyleFamilySpec],
    pub style_tokens: &'static [UiExtractedStyleToken],
    pub components: &'static [UiExtractedComponentSpec],
    pub slots: &'static [UiExtractedSlotSpec],
    pub source_icons: &'static [UiExtractedIconSpec],
    pub states: &'static [UiExtractedStateSpec],
    pub patterns: &'static [UiExtractedPatternSpec],
    pub blocks: &'static [UiExtractedBlockSpec],
    pub shadcn_demos: &'static [UiShadcnDemoSpec],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiExtractedInventoryKind {
    SourceCapture,
    StyleFamily,
    StyleToken,
    Component,
    Slot,
    SourceIcon,
    State,
    Pattern,
    Block,
    ShadcnDemo,
}

pub const EXTRACTED_INVENTORY_KINDS: [UiExtractedInventoryKind; 10] = [
    UiExtractedInventoryKind::SourceCapture,
    UiExtractedInventoryKind::StyleFamily,
    UiExtractedInventoryKind::StyleToken,
    UiExtractedInventoryKind::Component,
    UiExtractedInventoryKind::Slot,
    UiExtractedInventoryKind::SourceIcon,
    UiExtractedInventoryKind::State,
    UiExtractedInventoryKind::Pattern,
    UiExtractedInventoryKind::Block,
    UiExtractedInventoryKind::ShadcnDemo,
];

impl UiExtractedInventoryKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SourceCapture => "Source Captures",
            Self::StyleFamily => "Style Families",
            Self::StyleToken => "Style Tokens",
            Self::Component => "Components",
            Self::Slot => "Slots",
            Self::SourceIcon => "Source Icons",
            Self::State => "States",
            Self::Pattern => "Patterns",
            Self::Block => "Blocks",
            Self::ShadcnDemo => "shadcn Demos",
        }
    }

    pub const fn role(self) -> &'static str {
        match self {
            Self::SourceCapture => "HTML capture provenance",
            Self::StyleFamily => "user-selectable visual language",
            Self::StyleToken => "semantic CSS and renderer token bridge",
            Self::Component => "named reusable UI component inventory",
            Self::Slot => "data-slot bridge from HTML captures",
            Self::SourceIcon => "provider icon names and canonical fallbacks",
            Self::State => "interaction and accessibility state vocabulary",
            Self::Pattern => {
                "reusable recipes that combine components, slots, states, tokens, and icons"
            }
            Self::Block => "renderable extracted preview surfaces",
            Self::ShadcnDemo => "compatibility map for porting shadcn component surfaces",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedInventorySection {
    pub kind: UiExtractedInventoryKind,
    pub label: &'static str,
    pub count: usize,
    pub role: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedCatalogCounts {
    pub source_captures: usize,
    pub style_families: usize,
    pub style_tokens: usize,
    pub components: usize,
    pub slots: usize,
    pub source_icons: usize,
    pub states: usize,
    pub patterns: usize,
    pub blocks: usize,
    pub shadcn_demos: usize,
}

impl UiExtractedCatalogCounts {
    pub const fn total(self) -> usize {
        self.source_captures
            + self.style_families
            + self.style_tokens
            + self.components
            + self.slots
            + self.source_icons
            + self.states
            + self.patterns
            + self.blocks
            + self.shadcn_demos
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedCoverageReport {
    pub source_capture_coverage_complete: bool,
    pub style_families_with_source_captures: usize,
    pub style_family_count: usize,
    pub duplicate_source_preset_codes: usize,
    pub canonical_source_icons: usize,
    pub uncataloged_source_icons: usize,
    pub source_icon_count: usize,
    pub renderable_blocks: usize,
    pub block_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedPatternReferenceReport {
    pub missing_components: usize,
    pub missing_slots: usize,
    pub missing_states: usize,
    pub missing_tokens: usize,
    pub missing_icons: usize,
}

impl UiExtractedPatternReferenceReport {
    pub const fn total_missing(self) -> usize {
        self.missing_components
            + self.missing_slots
            + self.missing_states
            + self.missing_tokens
            + self.missing_icons
    }

    pub const fn complete(self) -> bool {
        self.total_missing() == 0
    }
}

impl UiExtractedCoverageReport {
    pub const fn style_family_coverage_ratio(self) -> (usize, usize) {
        (
            self.style_families_with_source_captures,
            self.style_family_count,
        )
    }

    pub const fn source_icon_canonical_ratio(self) -> (usize, usize) {
        (self.canonical_source_icons, self.source_icon_count)
    }

    pub const fn renderable_block_ratio(self) -> (usize, usize) {
        (self.renderable_blocks, self.block_count)
    }

    pub const fn complete_enough_for_component_gallery(self) -> bool {
        self.source_capture_coverage_complete && self.renderable_blocks == self.block_count
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiExtractedWorkItemKind {
    SourceCaptureCoverage,
    DuplicatePresetCode,
    CanonicalIconMapping,
    RenderableBlockCoverage,
    ShadcnNativePortCoverage,
}

impl UiExtractedWorkItemKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SourceCaptureCoverage => "Source Capture Coverage",
            Self::DuplicatePresetCode => "Duplicate Preset Code",
            Self::CanonicalIconMapping => "Canonical Icon Mapping",
            Self::RenderableBlockCoverage => "Renderable Block Coverage",
            Self::ShadcnNativePortCoverage => "shadcn Native Port Coverage",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedWorkItem {
    pub kind: UiExtractedWorkItemKind,
    pub label: &'static str,
    pub open_count: usize,
    pub complete: bool,
    pub role: &'static str,
}

impl UiExtractedWorkItem {
    pub const fn is_open(self) -> bool {
        !self.complete
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedComponentKindSummary {
    pub kind: UiExtractedComponentKind,
    pub label: &'static str,
    pub count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedStyleTokenKindSummary {
    pub kind: UiExtractedStyleTokenKind,
    pub label: &'static str,
    pub count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedStateKindSummary {
    pub kind: UiExtractedStateKind,
    pub label: &'static str,
    pub count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedPatternKindSummary {
    pub kind: UiExtractedPatternKind,
    pub label: &'static str,
    pub count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedBlockKindSummary {
    pub kind: UiExtractedBlockKind,
    pub label: &'static str,
    pub count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnDemoCategorySummary {
    pub category: UiShadcnDemoCategory,
    pub label: &'static str,
    pub count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnDemoStatusSummary {
    pub status: UiShadcnDemoStatus,
    pub label: &'static str,
    pub count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiShadcnCompatibilityReport {
    pub demo_count: usize,
    pub native_demo_count: usize,
    pub exact_port_count: usize,
    pub native_primitive_count: usize,
    pub cataloged_only_count: usize,
}

impl UiShadcnCompatibilityReport {
    pub const fn native_ratio(self) -> (usize, usize) {
        (self.native_demo_count, self.demo_count)
    }

    pub const fn complete(self) -> bool {
        self.cataloged_only_count == 0 && self.native_demo_count == self.demo_count
    }
}

impl UiExtractedDesignSystem {
    pub const fn component_count(self) -> usize {
        self.components.len()
    }

    pub const fn slot_count(self) -> usize {
        self.slots.len()
    }

    pub const fn source_icon_count(self) -> usize {
        self.source_icons.len()
    }

    pub const fn state_count(self) -> usize {
        self.states.len()
    }

    pub const fn pattern_count(self) -> usize {
        self.patterns.len()
    }

    pub const fn style_family_count(self) -> usize {
        self.style_families.len()
    }

    pub const fn source_capture_count(self) -> usize {
        self.source_captures.len()
    }

    pub const fn token_count(self) -> usize {
        self.style_tokens.len()
    }

    pub const fn block_count(self) -> usize {
        self.blocks.len()
    }

    pub const fn shadcn_demo_count(self) -> usize {
        self.shadcn_demos.len()
    }

    pub const fn catalog_counts(self) -> UiExtractedCatalogCounts {
        UiExtractedCatalogCounts {
            source_captures: self.source_capture_count(),
            style_families: self.style_family_count(),
            style_tokens: self.token_count(),
            components: self.component_count(),
            slots: self.slot_count(),
            source_icons: self.source_icon_count(),
            states: self.state_count(),
            patterns: self.pattern_count(),
            blocks: self.block_count(),
            shadcn_demos: self.shadcn_demo_count(),
        }
    }

    pub const fn total_inventory_count(self) -> usize {
        self.catalog_counts().total()
    }

    pub fn coverage_report(self) -> UiExtractedCoverageReport {
        UiExtractedCoverageReport {
            source_capture_coverage_complete: self.source_capture_coverage_complete(),
            style_families_with_source_captures: self.count_style_families_with_source_captures(),
            style_family_count: self.style_family_count(),
            duplicate_source_preset_codes: self.count_duplicate_source_preset_codes(),
            canonical_source_icons: self.count_source_icons_with_canonical(),
            uncataloged_source_icons: self.count_source_icons_without_canonical(),
            source_icon_count: self.source_icon_count(),
            renderable_blocks: self.count_renderable_blocks(),
            block_count: self.block_count(),
        }
    }

    pub fn shadcn_compatibility_report(self) -> UiShadcnCompatibilityReport {
        let exact_port_count = self.count_shadcn_demos_by_status(UiShadcnDemoStatus::ExactPort);
        let native_primitive_count =
            self.count_shadcn_demos_by_status(UiShadcnDemoStatus::NativePrimitive);
        let cataloged_only_count = self.count_shadcn_demos_by_status(UiShadcnDemoStatus::Cataloged);
        UiShadcnCompatibilityReport {
            demo_count: self.shadcn_demo_count(),
            native_demo_count: exact_port_count + native_primitive_count,
            exact_port_count,
            native_primitive_count,
            cataloged_only_count,
        }
    }

    pub fn pattern_reference_report(self) -> UiExtractedPatternReferenceReport {
        UiExtractedPatternReferenceReport {
            missing_components: self.count_missing_pattern_components(),
            missing_slots: self.count_missing_pattern_slots(),
            missing_states: self.count_missing_pattern_states(),
            missing_tokens: self.count_missing_pattern_tokens(),
            missing_icons: self.count_missing_pattern_icons(),
        }
    }

    pub fn coverage_work_items(self) -> [UiExtractedWorkItem; 5] {
        let report = self.coverage_report();
        let shadcn_report = self.shadcn_compatibility_report();
        [
            UiExtractedWorkItem {
                kind: UiExtractedWorkItemKind::SourceCaptureCoverage,
                label: UiExtractedWorkItemKind::SourceCaptureCoverage.label(),
                open_count: report
                    .style_family_count
                    .saturating_sub(report.style_families_with_source_captures),
                complete: report.source_capture_coverage_complete,
                role: "add or relink captures for style families missing source HTML",
            },
            UiExtractedWorkItem {
                kind: UiExtractedWorkItemKind::DuplicatePresetCode,
                label: UiExtractedWorkItemKind::DuplicatePresetCode.label(),
                open_count: report.duplicate_source_preset_codes,
                complete: report.duplicate_source_preset_codes == 0,
                role: "confirm intentional duplicate preset codes or replace them with distinct captures",
            },
            UiExtractedWorkItem {
                kind: UiExtractedWorkItemKind::CanonicalIconMapping,
                label: UiExtractedWorkItemKind::CanonicalIconMapping.label(),
                open_count: report.uncataloged_source_icons,
                complete: report.uncataloged_source_icons == 0,
                role: "add canonical UiIcon variants or keep explicit provider-only mappings",
            },
            UiExtractedWorkItem {
                kind: UiExtractedWorkItemKind::RenderableBlockCoverage,
                label: UiExtractedWorkItemKind::RenderableBlockCoverage.label(),
                open_count: report.block_count.saturating_sub(report.renderable_blocks),
                complete: report.renderable_blocks == report.block_count,
                role: "ensure every extracted block id has a preview builder",
            },
            UiExtractedWorkItem {
                kind: UiExtractedWorkItemKind::ShadcnNativePortCoverage,
                label: UiExtractedWorkItemKind::ShadcnNativePortCoverage.label(),
                open_count: shadcn_report.cataloged_only_count,
                complete: shadcn_report.complete(),
                role: "port cataloged shadcn demos to native EdgeRun builders and preview fixtures",
            },
        ]
    }

    pub fn count_open_coverage_work_items(self) -> usize {
        self.coverage_work_items()
            .iter()
            .filter(|item| item.is_open())
            .count()
    }

    pub const fn inventory_kinds(self) -> &'static [UiExtractedInventoryKind] {
        &EXTRACTED_INVENTORY_KINDS
    }

    pub const fn block_ids(self) -> &'static [UiExtractedBlockId] {
        &EXTRACTED_BLOCK_IDS
    }

    pub const fn block_kinds(self) -> &'static [UiExtractedBlockKind] {
        &EXTRACTED_BLOCK_KINDS
    }

    pub const fn component_kinds(self) -> &'static [UiExtractedComponentKind] {
        &EXTRACTED_COMPONENT_KINDS
    }

    pub const fn style_token_kinds(self) -> &'static [UiExtractedStyleTokenKind] {
        &EXTRACTED_STYLE_TOKEN_KINDS
    }

    pub const fn state_kinds(self) -> &'static [UiExtractedStateKind] {
        &EXTRACTED_STATE_KINDS
    }

    pub const fn pattern_kinds(self) -> &'static [UiExtractedPatternKind] {
        &EXTRACTED_PATTERN_KINDS
    }

    pub fn inventory_section(self, kind: UiExtractedInventoryKind) -> UiExtractedInventorySection {
        UiExtractedInventorySection {
            kind,
            label: kind.label(),
            count: self.count_inventory_items(kind),
            role: kind.role(),
        }
    }

    pub fn inventory_sections(self) -> [UiExtractedInventorySection; 10] {
        EXTRACTED_INVENTORY_KINDS.map(|kind| self.inventory_section(kind))
    }

    pub const fn count_inventory_items(self, kind: UiExtractedInventoryKind) -> usize {
        match kind {
            UiExtractedInventoryKind::SourceCapture => self.source_capture_count(),
            UiExtractedInventoryKind::StyleFamily => self.style_family_count(),
            UiExtractedInventoryKind::StyleToken => self.token_count(),
            UiExtractedInventoryKind::Component => self.component_count(),
            UiExtractedInventoryKind::Slot => self.slot_count(),
            UiExtractedInventoryKind::SourceIcon => self.source_icon_count(),
            UiExtractedInventoryKind::State => self.state_count(),
            UiExtractedInventoryKind::Pattern => self.pattern_count(),
            UiExtractedInventoryKind::Block => self.block_count(),
            UiExtractedInventoryKind::ShadcnDemo => self.shadcn_demo_count(),
        }
    }

    pub fn component_kind_summary(
        self,
        kind: UiExtractedComponentKind,
    ) -> UiExtractedComponentKindSummary {
        UiExtractedComponentKindSummary {
            kind,
            label: kind.label(),
            count: self.count_components_by_kind(kind),
        }
    }

    pub fn component_kind_summaries(self) -> [UiExtractedComponentKindSummary; 13] {
        EXTRACTED_COMPONENT_KINDS.map(|kind| self.component_kind_summary(kind))
    }

    pub fn block_kind_summary(self, kind: UiExtractedBlockKind) -> UiExtractedBlockKindSummary {
        UiExtractedBlockKindSummary {
            kind,
            label: kind.label(),
            count: self.count_blocks_by_kind(kind),
        }
    }

    pub fn block_kind_summaries(self) -> [UiExtractedBlockKindSummary; 7] {
        EXTRACTED_BLOCK_KINDS.map(|kind| self.block_kind_summary(kind))
    }

    pub const fn shadcn_demo_categories(self) -> &'static [UiShadcnDemoCategory] {
        &SHADCN_DEMO_CATEGORIES
    }

    pub const fn shadcn_demo_statuses(self) -> &'static [UiShadcnDemoStatus] {
        &SHADCN_DEMO_STATUSES
    }

    pub fn shadcn_demo_category_summary(
        self,
        category: UiShadcnDemoCategory,
    ) -> UiShadcnDemoCategorySummary {
        UiShadcnDemoCategorySummary {
            category,
            label: category.label(),
            count: self.count_shadcn_demos_by_category(category),
        }
    }

    pub fn shadcn_demo_category_summaries(self) -> [UiShadcnDemoCategorySummary; 8] {
        SHADCN_DEMO_CATEGORIES.map(|category| self.shadcn_demo_category_summary(category))
    }

    pub fn shadcn_demo_status_summary(
        self,
        status: UiShadcnDemoStatus,
    ) -> UiShadcnDemoStatusSummary {
        UiShadcnDemoStatusSummary {
            status,
            label: status.label(),
            count: self.count_shadcn_demos_by_status(status),
        }
    }

    pub fn shadcn_demo_status_summaries(self) -> [UiShadcnDemoStatusSummary; 3] {
        SHADCN_DEMO_STATUSES.map(|status| self.shadcn_demo_status_summary(status))
    }

    pub fn find_component(self, name: &str) -> Option<&'static UiExtractedComponentSpec> {
        self.components
            .iter()
            .find(|component| str_eq(component.name, name))
    }

    pub fn find_source_capture(self, path: &str) -> Option<&'static UiExtractedSourceCapture> {
        self.source_captures
            .iter()
            .find(|capture| str_eq(capture.path, path))
    }

    pub fn find_source_capture_for_family(
        self,
        family: UiStyleFamily,
    ) -> Option<&'static UiExtractedSourceCapture> {
        self.source_captures
            .iter()
            .find(|capture| capture.style_family == Some(family))
    }

    pub fn style_family_has_source_capture(self, family: UiStyleFamily) -> bool {
        self.find_source_capture_for_family(family).is_some()
    }

    pub fn count_style_families_with_source_captures(self) -> usize {
        self.style_families
            .iter()
            .filter(|spec| self.style_family_has_source_capture(spec.family))
            .count()
    }

    pub fn source_capture_coverage_complete(self) -> bool {
        self.count_style_families_with_source_captures() == self.style_family_count()
    }

    pub fn find_source_capture_by_preset_code(
        self,
        preset_code: &str,
    ) -> Option<&'static UiExtractedSourceCapture> {
        self.source_captures
            .iter()
            .find(|capture| str_eq(capture.preset_code, preset_code))
    }

    pub fn count_duplicate_source_preset_codes(self) -> usize {
        let mut duplicates = 0;
        let mut index = 0;
        while index < self.source_captures.len() {
            let preset_code = self.source_captures[index].preset_code;
            let mut earlier = 0;
            let mut seen = false;
            while earlier < index {
                if str_eq(self.source_captures[earlier].preset_code, preset_code) {
                    seen = true;
                    break;
                }
                earlier += 1;
            }
            if seen {
                duplicates += 1;
            }
            index += 1;
        }
        duplicates
    }

    pub fn find_component_by_builder(
        self,
        builder: &str,
    ) -> Option<&'static UiExtractedComponentSpec> {
        self.components
            .iter()
            .find(|component| component.uses_builder(builder))
    }

    pub fn components_by_source_class(
        self,
        class_name: &str,
    ) -> impl Iterator<Item = &'static UiExtractedComponentSpec> + '_ {
        self.components
            .iter()
            .filter(move |component| component.uses_source_class(class_name))
    }

    pub fn find_block(self, id: UiExtractedBlockId) -> Option<&'static UiExtractedBlockSpec> {
        self.blocks.iter().find(|block| block.id == id)
    }

    pub fn find_block_by_name(self, name: &str) -> Option<&'static UiExtractedBlockSpec> {
        self.blocks.iter().find(|block| str_eq(block.name, name))
    }

    pub fn find_shadcn_demo(self, slug: &str) -> Option<&'static UiShadcnDemoSpec> {
        self.shadcn_demos
            .iter()
            .find(|demo| str_eq(demo.slug, slug))
    }

    pub fn find_shadcn_demo_by_source_component(
        self,
        source_component: &str,
    ) -> Option<&'static UiShadcnDemoSpec> {
        self.shadcn_demos
            .iter()
            .find(|demo| str_eq(demo.source_component, source_component))
    }

    pub fn resolve_shadcn_demo_identifier(self, identifier: &str) -> Option<UiShadcnResolvedDemo> {
        let resolved = resolve_shadcn_demo_identifier(identifier)?;
        self.find_shadcn_demo(resolved.spec.slug).map(|_| resolved)
    }

    pub fn shadcn_port_mapping_for_identifier(
        self,
        identifier: &'static str,
    ) -> Option<UiShadcnPortMapping> {
        let resolved = self.resolve_shadcn_demo_identifier(identifier)?;
        Some(resolved.spec.port_mapping(identifier, resolved.kind))
    }

    pub const fn shadcn_port_manifest(
        self,
        identifiers: &'static [&'static str],
    ) -> UiShadcnPortManifest {
        shadcn_port_manifest(identifiers)
    }

    pub fn shadcn_demos_by_edge_builder(
        self,
        edge_builder: &str,
    ) -> impl Iterator<Item = &'static UiShadcnDemoSpec> + '_ {
        self.shadcn_demos
            .iter()
            .filter(move |demo| str_eq(demo.edge_builder, edge_builder))
    }

    pub fn shadcn_demos_by_category(
        self,
        category: UiShadcnDemoCategory,
    ) -> impl Iterator<Item = &'static UiShadcnDemoSpec> {
        self.shadcn_demos
            .iter()
            .filter(move |demo| demo.category == category)
    }

    pub fn shadcn_demos_by_status(
        self,
        status: UiShadcnDemoStatus,
    ) -> impl Iterator<Item = &'static UiShadcnDemoSpec> {
        self.shadcn_demos
            .iter()
            .filter(move |demo| demo.status == status)
    }

    pub fn shadcn_demos_missing_native_renderer(
        self,
    ) -> impl Iterator<Item = &'static UiShadcnDemoSpec> {
        self.shadcn_demos
            .iter()
            .filter(|demo| !demo.has_native_renderer())
    }

    pub fn shadcn_demos_using_slot(
        self,
        slot: &str,
    ) -> impl Iterator<Item = &'static UiShadcnDemoSpec> + '_ {
        self.shadcn_demos
            .iter()
            .filter(move |demo| demo.uses_slot(slot))
    }

    pub fn shadcn_demos_using_state(
        self,
        state: &str,
    ) -> impl Iterator<Item = &'static UiShadcnDemoSpec> + '_ {
        self.shadcn_demos
            .iter()
            .filter(move |demo| demo.uses_state(state))
    }

    pub fn count_shadcn_demos_by_category(self, category: UiShadcnDemoCategory) -> usize {
        self.shadcn_demos_by_category(category).count()
    }

    pub fn count_shadcn_demos_by_status(self, status: UiShadcnDemoStatus) -> usize {
        self.shadcn_demos_by_status(status).count()
    }

    pub fn count_native_shadcn_demos(self) -> usize {
        self.shadcn_demos
            .iter()
            .filter(|demo| demo.has_native_renderer())
            .count()
    }

    pub fn build_shadcn_demo(self, slug: &str) -> Option<UiNode> {
        self.find_shadcn_demo(slug)
            .and_then(|demo| build_shadcn_demo_preview(demo.slug))
    }

    pub fn build_shadcn_demo_by_identifier(self, identifier: &str) -> Option<UiNode> {
        self.resolve_shadcn_demo_identifier(identifier)
            .and_then(|resolved| build_shadcn_demo_preview_by_identifier(resolved.spec.slug))
    }

    pub fn build_shadcn_component(self, slug: &str) -> Option<UiNode> {
        self.find_shadcn_demo(slug)
            .and_then(|demo| build_shadcn_component_preview(demo.slug))
    }

    pub fn build_shadcn_component_by_identifier(self, identifier: &str) -> Option<UiNode> {
        self.resolve_shadcn_demo_identifier(identifier)
            .and_then(|resolved| build_shadcn_component_preview_by_identifier(resolved.spec.slug))
    }

    pub fn build_shadcn_component_by_source_component(
        self,
        source_component: &str,
    ) -> Option<UiNode> {
        self.find_shadcn_demo_by_source_component(source_component)
            .and_then(|demo| {
                build_shadcn_component_preview_by_source_component(demo.source_component)
            })
    }

    pub fn count_blocks_by_kind(self, kind: UiExtractedBlockKind) -> usize {
        self.blocks
            .iter()
            .filter(|block| block.kind == kind)
            .count()
    }

    pub fn blocks_by_kind(
        self,
        kind: UiExtractedBlockKind,
    ) -> impl Iterator<Item = &'static UiExtractedBlockSpec> {
        self.blocks.iter().filter(move |block| block.kind == kind)
    }

    pub fn count_renderable_blocks(self) -> usize {
        self.blocks
            .iter()
            .filter(|block| self.build_block(block.id).is_some())
            .count()
    }

    pub fn build_block(self, id: UiExtractedBlockId) -> Option<UiNode> {
        self.find_block(id).map(|_| build_extracted_block(id))
    }

    pub fn block_preview_rect(self, id: UiExtractedBlockId) -> Option<UiRect> {
        self.find_block(id).map(|block| block.preview_rect())
    }

    pub fn count_components_by_kind(self, kind: UiExtractedComponentKind) -> usize {
        self.components
            .iter()
            .filter(|component| component.kind == kind)
            .count()
    }

    pub fn components_by_kind(
        self,
        kind: UiExtractedComponentKind,
    ) -> impl Iterator<Item = &'static UiExtractedComponentSpec> {
        self.components
            .iter()
            .filter(move |component| component.kind == kind)
    }

    pub fn find_slot(self, slot: &str) -> Option<&'static UiExtractedSlotSpec> {
        self.slots
            .iter()
            .find(|candidate| str_eq(candidate.slot, slot))
    }

    pub fn count_slots_by_kind(self, kind: UiExtractedComponentKind) -> usize {
        self.slots.iter().filter(|slot| slot.kind == kind).count()
    }

    pub fn slots_by_kind(
        self,
        kind: UiExtractedComponentKind,
    ) -> impl Iterator<Item = &'static UiExtractedSlotSpec> {
        self.slots.iter().filter(move |slot| slot.kind == kind)
    }

    pub fn slots_by_source_class(
        self,
        class_name: &str,
    ) -> impl Iterator<Item = &'static UiExtractedSlotSpec> + '_ {
        self.slots
            .iter()
            .filter(move |slot| slot.uses_source_class(class_name))
    }

    pub fn find_source_icon(self, name: &str) -> Option<&'static UiExtractedIconSpec> {
        self.source_icons
            .iter()
            .find(|icon| str_eq(icon.name, name))
    }

    pub fn find_source_icon_by_provider_name(
        self,
        set: UiIconSet,
        provider_name: &str,
    ) -> Option<&'static UiExtractedIconSpec> {
        self.source_icons
            .iter()
            .find(|icon| str_eq(icon.provider_name(set), provider_name))
    }

    pub fn source_icons_with_canonical(self) -> impl Iterator<Item = &'static UiExtractedIconSpec> {
        self.source_icons
            .iter()
            .filter(|icon| icon.has_canonical_icon())
    }

    pub fn source_icons_without_canonical(
        self,
    ) -> impl Iterator<Item = &'static UiExtractedIconSpec> {
        self.source_icons
            .iter()
            .filter(|icon| !icon.has_canonical_icon())
    }

    pub fn count_source_icons_with_canonical(self) -> usize {
        self.source_icons_with_canonical().count()
    }

    pub fn count_source_icons_without_canonical(self) -> usize {
        self.source_icons_without_canonical().count()
    }

    pub fn source_icons_for_canonical(
        self,
        canonical: UiIcon,
    ) -> impl Iterator<Item = &'static UiExtractedIconSpec> {
        self.source_icons
            .iter()
            .filter(move |icon| icon.canonical == Some(canonical))
    }

    pub fn state_kind_summary(self, kind: UiExtractedStateKind) -> UiExtractedStateKindSummary {
        UiExtractedStateKindSummary {
            kind,
            label: kind.label(),
            count: self.count_states_by_kind(kind),
        }
    }

    pub fn style_token_kind_summary(
        self,
        kind: UiExtractedStyleTokenKind,
    ) -> UiExtractedStyleTokenKindSummary {
        UiExtractedStyleTokenKindSummary {
            kind,
            label: kind.label(),
            count: self.count_style_tokens_by_kind(kind),
        }
    }

    pub fn style_token_kind_summaries(self) -> [UiExtractedStyleTokenKindSummary; 5] {
        EXTRACTED_STYLE_TOKEN_KINDS.map(|kind| self.style_token_kind_summary(kind))
    }

    pub fn count_style_tokens_by_kind(self, kind: UiExtractedStyleTokenKind) -> usize {
        self.style_tokens
            .iter()
            .filter(|token| token.kind == kind)
            .count()
    }

    pub fn style_tokens_by_kind(
        self,
        kind: UiExtractedStyleTokenKind,
    ) -> impl Iterator<Item = &'static UiExtractedStyleToken> {
        self.style_tokens
            .iter()
            .filter(move |token| token.kind == kind)
    }

    pub fn find_style_family(self, family: UiStyleFamily) -> Option<&'static UiStyleFamilySpec> {
        self.style_families
            .iter()
            .find(|spec| spec.family == family)
    }

    pub fn style_family_at(self, index: usize) -> Option<&'static UiStyleFamilySpec> {
        self.style_families.get(index)
    }

    pub fn find_style_family_by_preset_code(
        self,
        preset_code: &str,
    ) -> Option<&'static UiStyleFamilySpec> {
        self.style_families
            .iter()
            .find(|spec| str_eq(spec.preset_code, preset_code))
    }

    pub fn find_style_token(self, name: &str) -> Option<&'static UiExtractedStyleToken> {
        self.style_tokens
            .iter()
            .find(|token| str_eq(token.name, name))
    }

    pub fn find_style_token_by_class(
        self,
        class_name: &str,
    ) -> Option<&'static UiExtractedStyleToken> {
        self.style_tokens
            .iter()
            .find(|token| token.has_class(class_name))
    }

    pub fn state_kind_summaries(self) -> [UiExtractedStateKindSummary; 7] {
        EXTRACTED_STATE_KINDS.map(|kind| self.state_kind_summary(kind))
    }

    pub fn find_state(self, name: &str) -> Option<&'static UiExtractedStateSpec> {
        self.states.iter().find(|state| str_eq(state.name, name))
    }

    pub fn find_state_by_selector(self, selector: &str) -> Option<&'static UiExtractedStateSpec> {
        self.states
            .iter()
            .find(|state| state.has_selector(selector))
    }

    pub fn count_states_by_kind(self, kind: UiExtractedStateKind) -> usize {
        self.states
            .iter()
            .filter(|state| state.kind == kind)
            .count()
    }

    pub fn states_by_kind(
        self,
        kind: UiExtractedStateKind,
    ) -> impl Iterator<Item = &'static UiExtractedStateSpec> {
        self.states.iter().filter(move |state| state.kind == kind)
    }

    pub fn pattern_kind_summary(
        self,
        kind: UiExtractedPatternKind,
    ) -> UiExtractedPatternKindSummary {
        UiExtractedPatternKindSummary {
            kind,
            label: kind.label(),
            count: self.count_patterns_by_kind(kind),
        }
    }

    pub fn pattern_kind_summaries(self) -> [UiExtractedPatternKindSummary; 8] {
        EXTRACTED_PATTERN_KINDS.map(|kind| self.pattern_kind_summary(kind))
    }

    pub fn find_pattern(self, name: &str) -> Option<&'static UiExtractedPatternSpec> {
        self.patterns
            .iter()
            .find(|pattern| str_eq(pattern.name, name))
    }

    pub fn count_patterns_by_kind(self, kind: UiExtractedPatternKind) -> usize {
        self.patterns
            .iter()
            .filter(|pattern| pattern.kind == kind)
            .count()
    }

    pub fn patterns_by_kind(
        self,
        kind: UiExtractedPatternKind,
    ) -> impl Iterator<Item = &'static UiExtractedPatternSpec> {
        self.patterns
            .iter()
            .filter(move |pattern| pattern.kind == kind)
    }

    pub fn patterns_using_component(
        self,
        name: &str,
    ) -> impl Iterator<Item = &'static UiExtractedPatternSpec> + '_ {
        self.patterns
            .iter()
            .filter(move |pattern| pattern.uses_component(name))
    }

    pub fn patterns_using_slot(
        self,
        slot: &str,
    ) -> impl Iterator<Item = &'static UiExtractedPatternSpec> + '_ {
        self.patterns
            .iter()
            .filter(move |pattern| pattern.uses_slot(slot))
    }

    pub fn patterns_using_state(
        self,
        state: &str,
    ) -> impl Iterator<Item = &'static UiExtractedPatternSpec> + '_ {
        self.patterns
            .iter()
            .filter(move |pattern| pattern.uses_state(state))
    }

    pub fn patterns_using_token(
        self,
        token: &str,
    ) -> impl Iterator<Item = &'static UiExtractedPatternSpec> + '_ {
        self.patterns
            .iter()
            .filter(move |pattern| pattern.uses_token(token))
    }

    pub fn patterns_using_icon(
        self,
        icon: &str,
    ) -> impl Iterator<Item = &'static UiExtractedPatternSpec> + '_ {
        self.patterns
            .iter()
            .filter(move |pattern| pattern.uses_icon(icon))
    }

    pub fn count_missing_pattern_components(self) -> usize {
        self.patterns
            .iter()
            .map(|pattern| {
                pattern
                    .components
                    .iter()
                    .filter(|component| self.find_component(component).is_none())
                    .count()
            })
            .sum()
    }

    pub fn count_missing_pattern_slots(self) -> usize {
        self.patterns
            .iter()
            .map(|pattern| {
                pattern
                    .slots
                    .iter()
                    .filter(|slot| self.find_slot(slot).is_none())
                    .count()
            })
            .sum()
    }

    pub fn count_missing_pattern_states(self) -> usize {
        self.patterns
            .iter()
            .map(|pattern| {
                pattern
                    .states
                    .iter()
                    .filter(|state| self.find_state(state).is_none())
                    .count()
            })
            .sum()
    }

    pub fn count_missing_pattern_tokens(self) -> usize {
        self.patterns
            .iter()
            .map(|pattern| {
                pattern
                    .tokens
                    .iter()
                    .filter(|token| self.find_style_token(token).is_none())
                    .count()
            })
            .sum()
    }

    pub fn count_missing_pattern_icons(self) -> usize {
        self.patterns
            .iter()
            .map(|pattern| {
                pattern
                    .icons
                    .iter()
                    .filter(|icon| self.find_source_icon(icon).is_none())
                    .count()
            })
            .sum()
    }
}

pub const EDGERUN_EXTRACTED_UI_SYSTEM: UiExtractedDesignSystem = UiExtractedDesignSystem {
    name: "EdgeRun extracted UI system",
    source: "ui.html plus seven style-family captures",
    source_captures: EXTRACTED_SOURCE_CAPTURES,
    style_families: STYLE_FAMILY_SPECS,
    style_tokens: EXTRACTED_STYLE_TOKENS,
    components: EXTRACTED_COMPONENTS,
    slots: EXTRACTED_SLOTS,
    source_icons: EXTRACTED_SOURCE_ICONS,
    states: EXTRACTED_STATES,
    patterns: EXTRACTED_PATTERNS,
    blocks: EXTRACTED_BLOCKS,
    shadcn_demos: SHADCN_DEMO_COMPONENTS,
};

fn str_eq(left: &str, right: &str) -> bool {
    left == right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracted_system_groups_families_tokens_and_components() {
        assert_eq!(EDGERUN_EXTRACTED_UI_SYSTEM.source_capture_count(), 7);
        assert_eq!(EDGERUN_EXTRACTED_UI_SYSTEM.style_family_count(), 7);
        assert_eq!(
            EDGERUN_EXTRACTED_UI_SYSTEM.count_style_families_with_source_captures(),
            7
        );
        assert!(EDGERUN_EXTRACTED_UI_SYSTEM.source_capture_coverage_complete());
        assert!(EDGERUN_EXTRACTED_UI_SYSTEM.token_count() >= 8);
        assert!(EDGERUN_EXTRACTED_UI_SYSTEM.component_count() >= 30);
        assert!(EDGERUN_EXTRACTED_UI_SYSTEM.slot_count() >= 20);
        assert!(EDGERUN_EXTRACTED_UI_SYSTEM.source_icon_count() >= 10);
        assert!(EDGERUN_EXTRACTED_UI_SYSTEM.state_count() >= 10);
        assert!(EDGERUN_EXTRACTED_UI_SYSTEM.pattern_count() >= 9);
        assert!(EDGERUN_EXTRACTED_UI_SYSTEM.block_count() >= 10);
        assert_eq!(EDGERUN_EXTRACTED_UI_SYSTEM.shadcn_demo_count(), 57);
        assert!(
            EDGERUN_EXTRACTED_UI_SYSTEM.total_inventory_count()
                > EDGERUN_EXTRACTED_UI_SYSTEM.component_count()
        );
        assert_eq!(
            EDGERUN_EXTRACTED_UI_SYSTEM.components[0].kind.label(),
            "Shell"
        );
    }

    #[test]
    fn extracted_system_supports_common_catalog_queries() {
        let system = EDGERUN_EXTRACTED_UI_SYSTEM;

        assert_eq!(
            system
                .find_component("Network App Card")
                .unwrap()
                .edgerun_builder,
            "package_card + route_path + button Run"
        );
        assert_eq!(
            system
                .find_component_by_builder("proof_event_row(title, hash, status, id)")
                .unwrap()
                .name,
            "Proof Event Row"
        );
        assert_eq!(
            system
                .find_source_capture_for_family(UiStyleFamily::Vega)
                .unwrap()
                .path,
            "vega.html"
        );
        assert_eq!(
            system
                .find_source_capture_by_preset_code("buFznsW")
                .unwrap()
                .project_slug,
            "radix-lyra"
        );
        assert_eq!(
            system.find_source_capture("ui.html").unwrap().style_family,
            Some(UiStyleFamily::Mira)
        );
        assert_eq!(
            system
                .find_source_capture_for_family(UiStyleFamily::Sera)
                .unwrap()
                .preset_code,
            "b4xFeBLg4O"
        );
        assert!(system.count_components_by_kind(UiExtractedComponentKind::EdgeRunDomain) >= 4);
        assert!(system
            .components_by_source_class("rounded-md")
            .any(|component| component.name == "Network App Card"));
        assert_eq!(
            system.find_slot("dropdown-menu-content").unwrap().kind,
            UiExtractedComponentKind::Overlay
        );
        assert!(system.count_slots_by_kind(UiExtractedComponentKind::Feedback) >= 5);
        assert!(system
            .slots_by_kind(UiExtractedComponentKind::Card)
            .any(|slot| slot.slot == "card-content"));
        assert!(system
            .slots_by_source_class("rounded-xl")
            .any(|slot| slot.slot == "card"));
        assert_eq!(
            system
                .find_source_icon_by_provider_name(UiIconSet::Lucide, "badge-check")
                .unwrap()
                .canonical,
            Some(UiIcon::Trust)
        );
        assert_eq!(
            system.find_source_icon("loader circle").unwrap().canonical,
            None
        );
        assert!(system
            .source_icons_with_canonical()
            .any(|icon| icon.name == "search"));
        assert!(system
            .source_icons_for_canonical(UiIcon::ChevronRight)
            .any(|icon| icon.lucide_name == "arrow-right"));
        assert_eq!(
            system
                .find_state_by_selector("focus-visible:ring-ring/50")
                .unwrap()
                .kind,
            UiExtractedStateKind::Accessibility
        );
        assert_eq!(
            system.find_state("invalid").unwrap().kind_label(),
            "Validation"
        );
        assert!(system.count_states_by_kind(UiExtractedStateKind::Selection) >= 3);
        assert!(system
            .states_by_kind(UiExtractedStateKind::Disclosure)
            .any(|state| state.name == "open"));
        assert_eq!(
            system
                .find_style_family(UiStyleFamily::Mira)
                .unwrap()
                .preset_code,
            "b1D0eCA4"
        );
        assert_eq!(system.style_family_at(0).unwrap().preset().name, "Vega");
        assert_eq!(
            system
                .find_style_family(UiStyleFamily::Mira)
                .unwrap()
                .colors(),
            super::super::colors_for_style_family(UiStyleFamily::Mira)
        );
        assert_eq!(
            system
                .find_style_family_by_preset_code("bbVKFP6")
                .unwrap()
                .family,
            UiStyleFamily::Maia
        );
        assert_eq!(
            system.find_style_token("background").unwrap().css_var,
            "--background"
        );
        assert_eq!(
            system.find_style_token("primary").unwrap().kind,
            UiExtractedStyleTokenKind::Action
        );
        assert_eq!(
            system
                .find_style_token_by_class("text-accent-foreground")
                .unwrap()
                .css_var,
            "--accent-foreground"
        );
        assert_eq!(
            system.find_style_token("chart 3").unwrap().kind,
            UiExtractedStyleTokenKind::Status
        );
        assert_eq!(
            system.find_style_token("success").unwrap().kind,
            UiExtractedStyleTokenKind::Status
        );
        assert_eq!(
            system
                .find_style_token_by_class("text-muted-foreground")
                .unwrap()
                .name,
            "muted foreground"
        );
        assert!(system.find_component("Unknown Component").is_none());
    }

    #[test]
    fn extracted_system_catalogs_component_patterns() {
        let system = EDGERUN_EXTRACTED_UI_SYSTEM;
        let network_app = system.find_pattern("Network App Card").unwrap();
        let reference_report = system.pattern_reference_report();

        assert_eq!(system.pattern_kinds()[0], UiExtractedPatternKind::Shell);
        assert_eq!(system.pattern_kinds()[1], UiExtractedPatternKind::Card);
        assert_eq!(network_app.kind, UiExtractedPatternKind::EdgeRunDomain);
        assert!(network_app.uses_component("Capability Grant Row"));
        assert!(network_app.uses_slot("card-footer"));
        assert!(network_app.uses_state("loading"));
        assert!(network_app.uses_token("success"));
        assert!(network_app.uses_icon("badge check"));
        assert_eq!(
            system
                .pattern_kind_summary(UiExtractedPatternKind::EdgeRunDomain)
                .label,
            "EdgeRun Domain"
        );
        assert!(system
            .pattern_kind_summaries()
            .iter()
            .any(|summary| summary.kind == UiExtractedPatternKind::Overlay && summary.count >= 2));
        assert!(system
            .patterns_by_kind(UiExtractedPatternKind::Overlay)
            .any(|pattern| pattern.name == "Dialog"));
        assert!(system
            .patterns_using_component("Primary Button")
            .any(|pattern| pattern.name == "Button"));
        assert!(system
            .patterns_using_slot("dropdown-menu-content")
            .any(|pattern| pattern.name == "Dropdown Menu"));
        assert!(system
            .patterns_using_state("focus visible")
            .any(|pattern| pattern.name == "Input Group"));
        assert!(system
            .patterns_using_token("success")
            .any(|pattern| pattern.name == "Network App Card"));
        assert!(system
            .patterns_using_icon("x")
            .any(|pattern| pattern.name == "Dialog"));
        assert!(reference_report.complete());
        assert_eq!(reference_report.total_missing(), 0);
    }

    #[test]
    fn extracted_system_catalogs_and_builds_preview_blocks() {
        let system = EDGERUN_EXTRACTED_UI_SYSTEM;

        assert_eq!(
            system
                .find_block(UiExtractedBlockId::NetworkApp)
                .unwrap()
                .builder,
            "network_app_block"
        );
        assert_eq!(
            system.find_block_by_name("Component Studio").unwrap().id,
            UiExtractedBlockId::ComponentStudio
        );
        assert_eq!(
            system
                .find_block(UiExtractedBlockId::ComponentStudio)
                .unwrap()
                .preview_height,
            4000
        );
        assert_eq!(
            system
                .block_preview_rect(UiExtractedBlockId::NetworkApp)
                .unwrap()
                .w,
            420.0
        );
        assert!(
            system
                .find_block(UiExtractedBlockId::StyleFamilyPicker)
                .unwrap()
                .preview_aspect_ratio()
                < 1.0
        );
        assert!(system
            .build_block(UiExtractedBlockId::TrustActivity)
            .is_some());
    }

    #[test]
    fn extracted_system_exposes_stable_block_ids_and_kinds() {
        let system = EDGERUN_EXTRACTED_UI_SYSTEM;
        let summaries = system.block_kind_summaries();

        assert_eq!(system.block_ids()[0], UiExtractedBlockId::StyleAuthority);
        assert_eq!(system.block_kinds()[0], UiExtractedBlockKind::Studio);
        assert_eq!(summaries[0].label, "Studio");
        assert_eq!(
            system.count_blocks_by_kind(UiExtractedBlockKind::DomainSurface),
            3
        );
        assert!(system
            .blocks_by_kind(UiExtractedBlockKind::DomainSurface)
            .any(|block| block.id == UiExtractedBlockId::NetworkApp));
    }

    #[test]
    fn extracted_system_exposes_stable_component_category_order() {
        let system = EDGERUN_EXTRACTED_UI_SYSTEM;
        let summaries = system.component_kind_summaries();

        assert_eq!(system.component_kinds()[0], UiExtractedComponentKind::Shell);
        assert_eq!(summaries[0].label, "Shell");
        assert_eq!(
            summaries[0].count,
            system.count_components_by_kind(UiExtractedComponentKind::Shell)
        );
        assert!(system
            .components_by_kind(UiExtractedComponentKind::Navigation)
            .any(|component| component.name == "Breadcrumb"));
        assert!(summaries.iter().any(|summary| {
            summary.kind == UiExtractedComponentKind::EdgeRunDomain && summary.count >= 4
        }));
    }

    #[test]
    fn extracted_system_exposes_stable_style_token_category_order() {
        let system = EDGERUN_EXTRACTED_UI_SYSTEM;
        let summaries = system.style_token_kind_summaries();

        assert_eq!(
            system.style_token_kinds()[0],
            UiExtractedStyleTokenKind::Surface
        );
        assert_eq!(summaries[0].label, "Surface");
        assert_eq!(
            summaries[0].count,
            system.count_style_tokens_by_kind(UiExtractedStyleTokenKind::Surface)
        );
        assert!(system
            .style_tokens_by_kind(UiExtractedStyleTokenKind::Text)
            .any(|token| token.css_var == "--muted-foreground"));
    }

    #[test]
    fn extracted_system_exposes_stable_state_category_order() {
        let system = EDGERUN_EXTRACTED_UI_SYSTEM;
        let summaries = system.state_kind_summaries();

        assert_eq!(system.state_kinds()[0], UiExtractedStateKind::Interaction);
        assert_eq!(summaries[0].label, "Interaction");
        assert_eq!(
            summaries[0].count,
            system.count_states_by_kind(UiExtractedStateKind::Interaction)
        );
        assert!(summaries
            .iter()
            .any(|summary| summary.kind == UiExtractedStateKind::Loading && summary.count == 1));
    }

    #[test]
    fn extracted_system_exposes_stable_inventory_sections() {
        let system = EDGERUN_EXTRACTED_UI_SYSTEM;
        let sections = system.inventory_sections();
        let counts = system.catalog_counts();

        assert_eq!(
            system.inventory_kinds()[0],
            UiExtractedInventoryKind::SourceCapture
        );
        assert_eq!(sections[0].label, "Source Captures");
        assert_eq!(sections[0].count, system.source_capture_count());
        assert_eq!(
            system.count_inventory_items(UiExtractedInventoryKind::Component),
            system.component_count()
        );
        assert_eq!(counts.components, system.component_count());
        assert_eq!(counts.patterns, system.pattern_count());
        assert_eq!(counts.shadcn_demos, system.shadcn_demo_count());
        assert_eq!(counts.total(), system.total_inventory_count());
        assert!(sections
            .iter()
            .any(|section| section.kind == UiExtractedInventoryKind::State && section.count >= 10));
        assert!(sections
            .iter()
            .any(|section| section.kind == UiExtractedInventoryKind::Pattern
                && section.count == system.pattern_count()));
        assert!(sections.iter().any(|section| section.kind
            == UiExtractedInventoryKind::ShadcnDemo
            && section.count == 57));
    }

    #[test]
    fn extracted_system_exposes_shadcn_compatibility_catalog() {
        let system = EDGERUN_EXTRACTED_UI_SYSTEM;
        let categories = system.shadcn_demo_category_summaries();
        let statuses = system.shadcn_demo_status_summaries();

        assert_eq!(
            system.shadcn_demo_categories()[0],
            UiShadcnDemoCategory::Foundation
        );
        assert_eq!(
            system.shadcn_demo_statuses()[0],
            UiShadcnDemoStatus::Cataloged
        );
        assert_eq!(
            system.find_shadcn_demo("button").unwrap().source_component,
            "Button"
        );
        assert_eq!(
            system.find_shadcn_demo("input-group").unwrap().edge_builder,
            "input_group_node"
        );
        assert_eq!(
            system
                .find_shadcn_demo_by_source_component("InputGroup")
                .unwrap()
                .slug,
            "input-group"
        );
        assert_eq!(
            system
                .resolve_shadcn_demo_identifier("@/components/ui/button")
                .unwrap()
                .spec
                .slug,
            "button"
        );
        assert_eq!(
            system
                .resolve_shadcn_demo_identifier("CardHeader")
                .unwrap()
                .spec
                .slug,
            "card"
        );
        assert_eq!(
            system
                .shadcn_port_mapping_for_identifier("@/components/ui/button")
                .unwrap()
                .edge_builder,
            "button"
        );
        assert_eq!(
            system
                .shadcn_port_manifest(&["@/components/ui/button", "Nope"])
                .resolved_count(),
            1
        );
        assert_eq!(
            system.count_native_shadcn_demos(),
            system.count_shadcn_demos_by_status(UiShadcnDemoStatus::NativePrimitive)
                + system.count_shadcn_demos_by_status(UiShadcnDemoStatus::ExactPort)
        );
        assert_eq!(system.count_native_shadcn_demos(), 57);
        assert!(system
            .shadcn_demos_by_category(UiShadcnDemoCategory::Overlay)
            .any(|demo| demo.slug == "dialog"));
        assert_eq!(
            system
                .shadcn_demos_by_status(UiShadcnDemoStatus::Cataloged)
                .count(),
            0
        );
        assert_eq!(system.shadcn_demos_missing_native_renderer().count(), 0);
        assert!(system
            .shadcn_demos_by_edge_builder("button")
            .any(|demo| demo.slug == "button"));
        assert!(system
            .shadcn_demos_using_slot("dialog-content")
            .any(|demo| demo.slug == "dialog"));
        assert!(system
            .shadcn_demos_using_state("disabled")
            .any(|demo| demo.slug == "button"));
        assert!(system.build_shadcn_demo("button").is_some());
        assert!(system.build_shadcn_demo("accordion").is_some());
        assert!(system
            .build_shadcn_demo_by_identifier("@/components/ui/button")
            .is_some());
        assert!(system.build_shadcn_component("button").is_some());
        assert!(system
            .build_shadcn_component_by_identifier("CardHeader")
            .is_some());
        assert!(system
            .build_shadcn_component_by_source_component("InputGroup")
            .is_some());
        assert!(system.build_shadcn_component("unknown-demo").is_none());
        assert!(categories
            .iter()
            .any(|summary| summary.category == UiShadcnDemoCategory::Form && summary.count >= 10));
        assert!(statuses.iter().any(|summary| {
            summary.status == UiShadcnDemoStatus::NativePrimitive && summary.count == 57
        }));
    }

    #[test]
    fn extracted_system_reports_shadcn_porting_coverage() {
        let system = EDGERUN_EXTRACTED_UI_SYSTEM;
        let report = system.shadcn_compatibility_report();
        let work_items = system.coverage_work_items();

        assert_eq!(report.demo_count, 57);
        assert_eq!(report.native_demo_count, 57);
        assert_eq!(report.native_primitive_count, 57);
        assert_eq!(report.exact_port_count, 0);
        assert_eq!(report.cataloged_only_count, 0);
        assert_eq!(report.native_ratio(), (57, 57));
        assert!(report.complete());
        assert_eq!(system.shadcn_demos_missing_native_renderer().count(), 0);
        assert!(work_items.iter().any(|item| item.kind
            == UiExtractedWorkItemKind::ShadcnNativePortCoverage
            && item.open_count == 0
            && !item.is_open()));
    }

    #[test]
    fn extracted_system_reports_coverage_and_partial_icon_mapping() {
        let system = EDGERUN_EXTRACTED_UI_SYSTEM;
        let report = system.coverage_report();
        let work_items = system.coverage_work_items();

        assert!(report.source_capture_coverage_complete);
        assert_eq!(report.style_family_coverage_ratio(), (7, 7));
        assert_eq!(report.duplicate_source_preset_codes, 0);
        assert_eq!(
            report.renderable_block_ratio(),
            (system.block_count(), system.block_count())
        );
        assert!(report.complete_enough_for_component_gallery());
        assert_eq!(
            report.source_icon_canonical_ratio(),
            (
                system.count_source_icons_with_canonical(),
                system.source_icon_count()
            )
        );
        assert!(report.uncataloged_source_icons > 0);
        assert!(system
            .source_icons_without_canonical()
            .any(|icon| icon.name == "loader circle"));
        assert_eq!(
            work_items[0].kind,
            UiExtractedWorkItemKind::SourceCaptureCoverage
        );
        assert_eq!(work_items[0].open_count, 0);
        assert!(work_items[0].complete);
        assert!(work_items.iter().any(|item| item.kind
            == UiExtractedWorkItemKind::DuplicatePresetCode
            && item.open_count == 0
            && !item.is_open()));
        assert!(work_items.iter().any(|item| item.kind
            == UiExtractedWorkItemKind::CanonicalIconMapping
            && item.open_count == report.uncataloged_source_icons
            && item.is_open()));
        assert_eq!(system.count_open_coverage_work_items(), 1);
    }
}
