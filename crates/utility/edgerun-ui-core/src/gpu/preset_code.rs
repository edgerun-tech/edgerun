//! shadcn preset-code recipe extracted from the live preset builder.

use std::string::String;

use super::UiStyleFamily;

pub const PRESET_CODE_ALPHABET: &str =
    "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

const THEME_COLORS: &[&str] = &[
    "neutral", "stone", "zinc", "gray", "amber", "blue", "cyan", "emerald", "fuchsia", "green",
    "indigo", "lime", "orange", "pink", "purple", "red", "rose", "sky", "teal", "violet", "yellow",
    "mauve", "olive", "mist", "taupe",
];

const BASE_COLORS: &[&str] = &[
    "neutral", "stone", "zinc", "gray", "mauve", "olive", "mist", "taupe",
];

const FONTS: &[&str] = &[
    "inter",
    "noto-sans",
    "nunito-sans",
    "figtree",
    "roboto",
    "raleway",
    "dm-sans",
    "public-sans",
    "outfit",
    "jetbrains-mono",
    "geist",
    "geist-mono",
    "lora",
    "merriweather",
    "playfair-display",
    "noto-serif",
    "roboto-slab",
    "oxanium",
    "manrope",
    "space-grotesk",
    "montserrat",
    "ibm-plex-sans",
    "source-sans-3",
    "instrument-sans",
    "eb-garamond",
    "instrument-serif",
];

const HEADING_FONTS: &[&str] = &[
    "inherit",
    "inter",
    "noto-sans",
    "nunito-sans",
    "figtree",
    "roboto",
    "raleway",
    "dm-sans",
    "public-sans",
    "outfit",
    "jetbrains-mono",
    "geist",
    "geist-mono",
    "lora",
    "merriweather",
    "playfair-display",
    "noto-serif",
    "roboto-slab",
    "oxanium",
    "manrope",
    "space-grotesk",
    "montserrat",
    "ibm-plex-sans",
    "source-sans-3",
    "instrument-sans",
    "eb-garamond",
    "instrument-serif",
];

const MENU_COLORS: &[&str] = &[
    "default",
    "inverted",
    "default-translucent",
    "inverted-translucent",
];

const MENU_ACCENTS: &[&str] = &["subtle", "bold"];
const RADII: &[&str] = &["default", "none", "small", "medium", "large"];
const ICON_LIBRARIES: &[&str] = &["lucide", "hugeicons", "tabler", "phosphor", "remixicon"];
const STYLES: &[&str] = &["nova", "vega", "maia", "lyra", "mira", "luma", "sera"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiPresetRecipe {
    pub style: &'static str,
    pub base_color: &'static str,
    pub theme: &'static str,
    pub chart_color: &'static str,
    pub icon_library: &'static str,
    pub font: &'static str,
    pub font_heading: &'static str,
    pub encoded_radius: &'static str,
    pub effective_radius: &'static str,
    pub menu_color: &'static str,
    pub menu_accent: &'static str,
}

impl UiPresetRecipe {
    pub fn encode_preset_code(self) -> Option<String> {
        encode_preset_code(self)
    }

    pub fn matches_preset_code(self, preset_code: &str) -> bool {
        self.encode_preset_code().as_deref() == Some(preset_code)
    }
}

#[derive(Clone, Copy)]
struct PresetField {
    key: &'static str,
    values: &'static [&'static str],
    bits: u32,
}

const PRESET_FIELDS: &[PresetField] = &[
    PresetField {
        key: "menuColor",
        values: MENU_COLORS,
        bits: 3,
    },
    PresetField {
        key: "menuAccent",
        values: MENU_ACCENTS,
        bits: 3,
    },
    PresetField {
        key: "radius",
        values: RADII,
        bits: 4,
    },
    PresetField {
        key: "font",
        values: FONTS,
        bits: 6,
    },
    PresetField {
        key: "iconLibrary",
        values: ICON_LIBRARIES,
        bits: 6,
    },
    PresetField {
        key: "theme",
        values: THEME_COLORS,
        bits: 6,
    },
    PresetField {
        key: "baseColor",
        values: BASE_COLORS,
        bits: 6,
    },
    PresetField {
        key: "style",
        values: STYLES,
        bits: 6,
    },
    PresetField {
        key: "chartColor",
        values: THEME_COLORS,
        bits: 6,
    },
    PresetField {
        key: "fontHeading",
        values: HEADING_FONTS,
        bits: 5,
    },
];

pub const fn preset_recipe_for_style_family(family: UiStyleFamily) -> UiPresetRecipe {
    match family {
        UiStyleFamily::Vega => preset_recipe(
            "vega", "neutral", "neutral", "neutral", "lucide", "inter", "inherit", "default",
            "default", "default", "subtle",
        ),
        UiStyleFamily::Nova => preset_recipe(
            "nova", "neutral", "neutral", "neutral", "lucide", "geist", "inherit", "default",
            "default", "default", "subtle",
        ),
        UiStyleFamily::Maia => preset_recipe(
            "maia",
            "neutral",
            "neutral",
            "neutral",
            "hugeicons",
            "figtree",
            "inherit",
            "default",
            "default",
            "default",
            "subtle",
        ),
        UiStyleFamily::Lyra => preset_recipe(
            "lyra",
            "neutral",
            "neutral",
            "neutral",
            "phosphor",
            "jetbrains-mono",
            "inherit",
            "default",
            "none",
            "default",
            "subtle",
        ),
        UiStyleFamily::Mira => preset_recipe(
            "mira",
            "neutral",
            "neutral",
            "neutral",
            "hugeicons",
            "inter",
            "inherit",
            "default",
            "default",
            "default",
            "subtle",
        ),
        UiStyleFamily::Luma => preset_recipe(
            "luma", "neutral", "neutral", "neutral", "lucide", "inter", "inherit", "default",
            "default", "default", "subtle",
        ),
        UiStyleFamily::Sera => preset_recipe(
            "sera",
            "taupe",
            "taupe",
            "taupe",
            "lucide",
            "noto-sans",
            "playfair-display",
            "default",
            "none",
            "default",
            "subtle",
        ),
    }
}

const fn preset_recipe(
    style: &'static str,
    base_color: &'static str,
    theme: &'static str,
    chart_color: &'static str,
    icon_library: &'static str,
    font: &'static str,
    font_heading: &'static str,
    encoded_radius: &'static str,
    effective_radius: &'static str,
    menu_color: &'static str,
    menu_accent: &'static str,
) -> UiPresetRecipe {
    UiPresetRecipe {
        style,
        base_color,
        theme,
        chart_color,
        icon_library,
        font,
        font_heading,
        encoded_radius,
        effective_radius,
        menu_color,
        menu_accent,
    }
}

pub fn encode_preset_code(recipe: UiPresetRecipe) -> Option<String> {
    let mut value = 0_u64;
    let mut shift = 0_u32;
    for field in PRESET_FIELDS {
        let selected = recipe_value(recipe, field.key);
        let index = lookup_index(field.values, selected)? as u64;
        value += index << shift;
        shift += field.bits;
    }
    let mut encoded = String::from("b");
    encoded.push_str(&encode_base62(value));
    Some(encoded)
}

pub fn decode_preset_code(preset_code: &str) -> Option<UiPresetRecipe> {
    let suffix = preset_code.strip_prefix('b')?;
    let value = decode_base62(suffix)?;
    let mut shift = 0_u32;
    let mut decoded = preset_recipe_for_style_family(UiStyleFamily::Nova);
    for field in PRESET_FIELDS {
        let mask = (1_u64 << field.bits) - 1;
        let index = ((value >> shift) & mask) as usize;
        let selected = *field.values.get(index).unwrap_or(&field.values[0]);
        decoded = with_recipe_value(decoded, field.key, selected);
        shift += field.bits;
    }
    Some(decoded)
}

pub fn is_preset_code(preset_code: &str) -> bool {
    if preset_code.len() < 2 || preset_code.len() > 10 {
        return false;
    }
    if !preset_code.starts_with('a') && !preset_code.starts_with('b') {
        return false;
    }
    preset_code[1..]
        .bytes()
        .all(|byte| alphabet_index(byte).is_some())
}

fn recipe_value(recipe: UiPresetRecipe, key: &str) -> &'static str {
    match key {
        "menuColor" => recipe.menu_color,
        "menuAccent" => recipe.menu_accent,
        "radius" => recipe.encoded_radius,
        "font" => recipe.font,
        "iconLibrary" => recipe.icon_library,
        "theme" => recipe.theme,
        "baseColor" => recipe.base_color,
        "style" => recipe.style,
        "chartColor" => recipe.chart_color,
        "fontHeading" => recipe.font_heading,
        _ => "",
    }
}

fn with_recipe_value(mut recipe: UiPresetRecipe, key: &str, value: &'static str) -> UiPresetRecipe {
    match key {
        "menuColor" => recipe.menu_color = value,
        "menuAccent" => recipe.menu_accent = value,
        "radius" => {
            recipe.encoded_radius = value;
            recipe.effective_radius = if recipe.style == "lyra" || recipe.style == "sera" {
                "none"
            } else {
                value
            };
        }
        "font" => recipe.font = value,
        "iconLibrary" => recipe.icon_library = value,
        "theme" => recipe.theme = value,
        "baseColor" => recipe.base_color = value,
        "style" => {
            recipe.style = value;
            if value == "lyra" || value == "sera" {
                recipe.effective_radius = "none";
            }
        }
        "chartColor" => recipe.chart_color = value,
        "fontHeading" => recipe.font_heading = value,
        _ => {}
    }
    recipe
}

fn lookup_index(values: &[&'static str], selected: &str) -> Option<usize> {
    values.iter().position(|value| *value == selected)
}

fn encode_base62(mut value: u64) -> String {
    if value == 0 {
        return String::from("0");
    }
    let alphabet = PRESET_CODE_ALPHABET.as_bytes();
    let mut reversed = [0_u8; 11];
    let mut len = 0_usize;
    while value > 0 {
        reversed[len] = alphabet[(value % 62) as usize];
        value /= 62;
        len += 1;
    }
    let mut encoded = String::new();
    while len > 0 {
        len -= 1;
        encoded.push(reversed[len] as char);
    }
    encoded
}

fn decode_base62(value: &str) -> Option<u64> {
    let mut decoded = 0_u64;
    for byte in value.bytes() {
        decoded = decoded.checked_mul(62)?;
        decoded = decoded.checked_add(alphabet_index(byte)? as u64)?;
    }
    Some(decoded)
}

fn alphabet_index(byte: u8) -> Option<usize> {
    PRESET_CODE_ALPHABET
        .as_bytes()
        .iter()
        .position(|candidate| *candidate == byte)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_captured_family_presets() {
        let expected = [
            (UiStyleFamily::Vega, "bIkeymG"),
            (UiStyleFamily::Nova, "b2fA"),
            (UiStyleFamily::Maia, "bbVKFP6"),
            (UiStyleFamily::Lyra, "buFznsW"),
            (UiStyleFamily::Mira, "b1D0eCA4"),
            (UiStyleFamily::Luma, "b1VlIttI"),
            (UiStyleFamily::Sera, "b4xFeBLg4O"),
        ];
        for (family, code) in expected {
            assert_eq!(
                preset_recipe_for_style_family(family)
                    .encode_preset_code()
                    .as_deref(),
                Some(code)
            );
        }
    }

    #[test]
    fn decodes_sera_recipe_and_effective_radius() {
        let recipe = decode_preset_code("b4xFeBLg4O").expect("valid Sera preset");
        assert_eq!(recipe.style, "sera");
        assert_eq!(recipe.base_color, "taupe");
        assert_eq!(recipe.theme, "taupe");
        assert_eq!(recipe.chart_color, "taupe");
        assert_eq!(recipe.font, "noto-sans");
        assert_eq!(recipe.font_heading, "playfair-display");
        assert_eq!(recipe.encoded_radius, "default");
        assert_eq!(recipe.effective_radius, "none");
    }

    #[test]
    fn encodes_live_sera_sidebar_deltas() {
        let sera = preset_recipe_for_style_family(UiStyleFamily::Sera);

        assert_eq!(
            UiPresetRecipe {
                style: "luma",
                base_color: "neutral",
                theme: "neutral",
                chart_color: "neutral",
                icon_library: "lucide",
                font: "inter",
                font_heading: "inherit",
                encoded_radius: "default",
                effective_radius: "default",
                menu_color: "default",
                menu_accent: "subtle",
            }
            .encode_preset_code()
            .as_deref(),
            Some("b1VlIttI")
        );
        assert_eq!(
            UiPresetRecipe {
                base_color: "neutral",
                theme: "neutral",
                chart_color: "neutral",
                ..sera
            }
            .encode_preset_code()
            .as_deref(),
            Some("b4pl227H7Y")
        );
        assert_eq!(
            UiPresetRecipe {
                font_heading: "inter",
                ..sera
            }
            .encode_preset_code()
            .as_deref(),
            Some("bRVJIXa1w")
        );
        assert_eq!(
            UiPresetRecipe {
                font: "inter",
                ..sera
            }
            .encode_preset_code()
            .as_deref(),
            Some("b4xFeBLfns")
        );
        assert_eq!(
            UiPresetRecipe {
                icon_library: "hugeicons",
                ..sera
            }
            .encode_preset_code()
            .as_deref(),
            Some("b4xFeBLx7Q")
        );
        assert_eq!(
            UiPresetRecipe {
                menu_accent: "bold",
                ..sera
            }
            .encode_preset_code()
            .as_deref(),
            Some("b4xFeBLg4W")
        );
    }
}
