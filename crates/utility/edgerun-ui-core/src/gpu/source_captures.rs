//! Source capture provenance for the extracted UI system.

use super::{UiPresetRecipe, UiStyleFamily, preset_recipe_for_style_family};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedSourceCapture {
    pub path: &'static str,
    pub style_family: Option<UiStyleFamily>,
    pub preset_code: &'static str,
    pub preset_recipe: Option<UiPresetRecipe>,
    pub project_slug: &'static str,
    pub role: &'static str,
}

pub const EXTRACTED_SOURCE_CAPTURES: &[UiExtractedSourceCapture] = &[
    source_capture(
        "ui.html",
        Some(UiStyleFamily::Mira),
        "b1D0eCA4",
        "radix-mira",
        "original screenshot source mapped to the neutral Mira style family",
    ),
    source_capture(
        "vega.html",
        Some(UiStyleFamily::Vega),
        "bIkeymG",
        "radix-vega",
        "blue-black style-family capture",
    ),
    source_capture(
        "nova.html",
        Some(UiStyleFamily::Nova),
        "b2fA",
        "radix-nova",
        "graphite style-family capture",
    ),
    source_capture(
        "maia.html",
        Some(UiStyleFamily::Maia),
        "bbVKFP6",
        "radix-maia",
        "green trust style-family capture",
    ),
    source_capture(
        "lyra.html",
        Some(UiStyleFamily::Lyra),
        "buFznsW",
        "radix-lyra",
        "violet agent style-family capture",
    ),
    source_capture(
        "luma.html",
        Some(UiStyleFamily::Luma),
        "b1VlIttI",
        "radix-luma",
        "warm finance style-family capture",
    ),
    source_capture(
        "sera.html",
        Some(UiStyleFamily::Sera),
        "b4xFeBLg4O",
        "radix-sera",
        "rose collaboration style-family capture",
    ),
];

const fn source_capture(
    path: &'static str,
    style_family: Option<UiStyleFamily>,
    preset_code: &'static str,
    project_slug: &'static str,
    role: &'static str,
) -> UiExtractedSourceCapture {
    UiExtractedSourceCapture {
        path,
        style_family,
        preset_code,
        preset_recipe: match style_family {
            Some(family) => Some(preset_recipe_for_style_family(family)),
            None => None,
        },
        project_slug,
        role,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_captures_cover_style_family_files() {
        assert_eq!(EXTRACTED_SOURCE_CAPTURES.len(), 7);
        assert_eq!(EXTRACTED_SOURCE_CAPTURES[0].path, "ui.html");
        assert_eq!(
            EXTRACTED_SOURCE_CAPTURES[0].style_family,
            Some(UiStyleFamily::Mira)
        );
        assert!(EXTRACTED_SOURCE_CAPTURES.iter().any(|capture| {
            capture.path == "sera.html"
                && capture.style_family == Some(UiStyleFamily::Sera)
                && capture.preset_code == "b4xFeBLg4O"
                && capture.project_slug == "radix-sera"
        }));
    }

    #[test]
    fn source_capture_preset_codes_match_recipes() {
        for capture in EXTRACTED_SOURCE_CAPTURES {
            let recipe = capture
                .preset_recipe
                .expect("style-family captures have preset recipes");
            assert_eq!(
                recipe.encode_preset_code().as_deref(),
                Some(capture.preset_code)
            );
        }
    }

    #[test]
    fn sera_capture_preserves_taupe_recipe_and_effective_radius() {
        let capture = EXTRACTED_SOURCE_CAPTURES
            .iter()
            .find(|capture| capture.style_family == Some(UiStyleFamily::Sera))
            .expect("Sera source capture");
        let recipe = capture.preset_recipe.expect("Sera recipe");
        assert_eq!(recipe.base_color, "taupe");
        assert_eq!(recipe.theme, "taupe");
        assert_eq!(recipe.chart_color, "taupe");
        assert_eq!(recipe.font, "noto-sans");
        assert_eq!(recipe.font_heading, "playfair-display");
        assert_eq!(recipe.encoded_radius, "default");
        assert_eq!(recipe.effective_radius, "none");
    }
}
