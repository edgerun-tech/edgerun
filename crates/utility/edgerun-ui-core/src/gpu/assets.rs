//! Runtime-replaceable UI asset pack contracts.
//!
//! Hosts may swap icons, fonts, emoji, and component inventories at runtime
//! only after validating the replacement against this contract. Rendering code
//! can then assume required coverage exists instead of choosing ambiguous
//! runtime substitutes during paint.

use super::component_inventory::{EXTRACTED_COMPONENT_KINDS, UiExtractedComponentKind};
use super::{UiIcon, UiIconSet};

pub const REQUIRED_UI_EMOJI: &[&str] = &[
    "check", "warning", "locked", "unlocked", "route", "storage", "payment", "proof",
];

pub const REQUIRED_FONT_CHARS: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.:/#@[](){}<>+=%$!?&, ";

pub const EDGERUN_TABLER_ICON_PACK_ENTRIES: &[UiIconPackEntry] = &[
    UiIconPackEntry {
        icon: UiIcon::Activity,
        provider_name: "activity",
    },
    UiIconPackEntry {
        icon: UiIcon::App,
        provider_name: "apps",
    },
    UiIconPackEntry {
        icon: UiIcon::Bell,
        provider_name: "bell",
    },
    UiIconPackEntry {
        icon: UiIcon::Chat,
        provider_name: "message-circle",
    },
    UiIconPackEntry {
        icon: UiIcon::Check,
        provider_name: "check",
    },
    UiIconPackEntry {
        icon: UiIcon::ChevronRight,
        provider_name: "chevron-right",
    },
    UiIconPackEntry {
        icon: UiIcon::Code,
        provider_name: "code",
    },
    UiIconPackEntry {
        icon: UiIcon::Cpu,
        provider_name: "cpu",
    },
    UiIconPackEntry {
        icon: UiIcon::Database,
        provider_name: "database",
    },
    UiIconPackEntry {
        icon: UiIcon::Eye,
        provider_name: "eye",
    },
    UiIconPackEntry {
        icon: UiIcon::File,
        provider_name: "file",
    },
    UiIconPackEntry {
        icon: UiIcon::Key,
        provider_name: "key",
    },
    UiIconPackEntry {
        icon: UiIcon::Lock,
        provider_name: "lock",
    },
    UiIconPackEntry {
        icon: UiIcon::Menu,
        provider_name: "menu-2",
    },
    UiIconPackEntry {
        icon: UiIcon::MessagePlus,
        provider_name: "message-plus",
    },
    UiIconPackEntry {
        icon: UiIcon::Network,
        provider_name: "network",
    },
    UiIconPackEntry {
        icon: UiIcon::Route,
        provider_name: "route",
    },
    UiIconPackEntry {
        icon: UiIcon::Search,
        provider_name: "search",
    },
    UiIconPackEntry {
        icon: UiIcon::Send,
        provider_name: "arrow-up",
    },
    UiIconPackEntry {
        icon: UiIcon::Server,
        provider_name: "server",
    },
    UiIconPackEntry {
        icon: UiIcon::Settings,
        provider_name: "settings",
    },
    UiIconPackEntry {
        icon: UiIcon::Shield,
        provider_name: "shield-check",
    },
    UiIconPackEntry {
        icon: UiIcon::Sparkles,
        provider_name: "sparkles",
    },
    UiIconPackEntry {
        icon: UiIcon::Storage,
        provider_name: "database",
    },
    UiIconPackEntry {
        icon: UiIcon::Terminal,
        provider_name: "terminal-2",
    },
    UiIconPackEntry {
        icon: UiIcon::Trust,
        provider_name: "shield-check",
    },
    UiIconPackEntry {
        icon: UiIcon::Trash,
        provider_name: "trash",
    },
    UiIconPackEntry {
        icon: UiIcon::User,
        provider_name: "user",
    },
    UiIconPackEntry {
        icon: UiIcon::Wallet,
        provider_name: "wallet",
    },
    UiIconPackEntry {
        icon: UiIcon::Warning,
        provider_name: "alert-triangle",
    },
    UiIconPackEntry {
        icon: UiIcon::X,
        provider_name: "x",
    },
];

pub const EDGERUN_LUCIDE_ICON_PACK_ENTRIES: &[UiIconPackEntry] = &[
    UiIconPackEntry {
        icon: UiIcon::Activity,
        provider_name: "activity",
    },
    UiIconPackEntry {
        icon: UiIcon::App,
        provider_name: "app-window",
    },
    UiIconPackEntry {
        icon: UiIcon::Bell,
        provider_name: "bell",
    },
    UiIconPackEntry {
        icon: UiIcon::Chat,
        provider_name: "message-circle",
    },
    UiIconPackEntry {
        icon: UiIcon::Check,
        provider_name: "check",
    },
    UiIconPackEntry {
        icon: UiIcon::ChevronRight,
        provider_name: "chevron-right",
    },
    UiIconPackEntry {
        icon: UiIcon::Code,
        provider_name: "code",
    },
    UiIconPackEntry {
        icon: UiIcon::Cpu,
        provider_name: "cpu",
    },
    UiIconPackEntry {
        icon: UiIcon::Database,
        provider_name: "database",
    },
    UiIconPackEntry {
        icon: UiIcon::Eye,
        provider_name: "eye",
    },
    UiIconPackEntry {
        icon: UiIcon::File,
        provider_name: "file",
    },
    UiIconPackEntry {
        icon: UiIcon::Key,
        provider_name: "key",
    },
    UiIconPackEntry {
        icon: UiIcon::Lock,
        provider_name: "lock",
    },
    UiIconPackEntry {
        icon: UiIcon::Menu,
        provider_name: "menu",
    },
    UiIconPackEntry {
        icon: UiIcon::MessagePlus,
        provider_name: "message-circle-plus",
    },
    UiIconPackEntry {
        icon: UiIcon::Network,
        provider_name: "network",
    },
    UiIconPackEntry {
        icon: UiIcon::Route,
        provider_name: "route",
    },
    UiIconPackEntry {
        icon: UiIcon::Search,
        provider_name: "search",
    },
    UiIconPackEntry {
        icon: UiIcon::Send,
        provider_name: "arrow-up",
    },
    UiIconPackEntry {
        icon: UiIcon::Server,
        provider_name: "server",
    },
    UiIconPackEntry {
        icon: UiIcon::Settings,
        provider_name: "settings",
    },
    UiIconPackEntry {
        icon: UiIcon::Shield,
        provider_name: "shield-check",
    },
    UiIconPackEntry {
        icon: UiIcon::Sparkles,
        provider_name: "sparkles",
    },
    UiIconPackEntry {
        icon: UiIcon::Storage,
        provider_name: "database",
    },
    UiIconPackEntry {
        icon: UiIcon::Terminal,
        provider_name: "square-terminal",
    },
    UiIconPackEntry {
        icon: UiIcon::Trust,
        provider_name: "shield-check",
    },
    UiIconPackEntry {
        icon: UiIcon::Trash,
        provider_name: "trash-2",
    },
    UiIconPackEntry {
        icon: UiIcon::User,
        provider_name: "user",
    },
    UiIconPackEntry {
        icon: UiIcon::Wallet,
        provider_name: "wallet",
    },
    UiIconPackEntry {
        icon: UiIcon::Warning,
        provider_name: "triangle-alert",
    },
    UiIconPackEntry {
        icon: UiIcon::X,
        provider_name: "x",
    },
];

pub const EDGERUN_INTER_FONT_FACES: &[UiFontFaceSpec] = &[UiFontFaceSpec {
    name: "Inter",
    default_face: true,
    covered_chars: REQUIRED_FONT_CHARS,
}];

pub const EDGERUN_GEIST_FONT_FACES: &[UiFontFaceSpec] = &[UiFontFaceSpec {
    name: "Geist",
    default_face: true,
    covered_chars: REQUIRED_FONT_CHARS,
}];

pub const EDGERUN_REQUIRED_EMOJI: &[UiEmojiSpec] = &[
    UiEmojiSpec {
        key: "check",
        label: "check",
    },
    UiEmojiSpec {
        key: "warning",
        label: "warning",
    },
    UiEmojiSpec {
        key: "locked",
        label: "locked",
    },
    UiEmojiSpec {
        key: "unlocked",
        label: "unlocked",
    },
    UiEmojiSpec {
        key: "route",
        label: "route",
    },
    UiEmojiSpec {
        key: "storage",
        label: "storage",
    },
    UiEmojiSpec {
        key: "payment",
        label: "payment",
    },
    UiEmojiSpec {
        key: "proof",
        label: "proof",
    },
];

pub const EDGERUN_COMPONENT_PACK_ENTRIES: &[UiComponentPackEntry] = &[
    UiComponentPackEntry {
        name: "shell",
        kind: UiExtractedComponentKind::Shell,
    },
    UiComponentPackEntry {
        name: "layout",
        kind: UiExtractedComponentKind::Layout,
    },
    UiComponentPackEntry {
        name: "navigation",
        kind: UiExtractedComponentKind::Navigation,
    },
    UiComponentPackEntry {
        name: "overlay",
        kind: UiExtractedComponentKind::Overlay,
    },
    UiComponentPackEntry {
        name: "card",
        kind: UiExtractedComponentKind::Card,
    },
    UiComponentPackEntry {
        name: "form",
        kind: UiExtractedComponentKind::Form,
    },
    UiComponentPackEntry {
        name: "input-group",
        kind: UiExtractedComponentKind::InputGroup,
    },
    UiComponentPackEntry {
        name: "chart",
        kind: UiExtractedComponentKind::Chart,
    },
    UiComponentPackEntry {
        name: "data-row",
        kind: UiExtractedComponentKind::DataRow,
    },
    UiComponentPackEntry {
        name: "control",
        kind: UiExtractedComponentKind::Control,
    },
    UiComponentPackEntry {
        name: "selection",
        kind: UiExtractedComponentKind::Selection,
    },
    UiComponentPackEntry {
        name: "feedback",
        kind: UiExtractedComponentKind::Feedback,
    },
    UiComponentPackEntry {
        name: "domain",
        kind: UiExtractedComponentKind::EdgeRunDomain,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiAssetLimits {
    pub max_icon_count: usize,
    pub max_icon_atlas_side: u32,
    pub max_icon_atlas_bytes: usize,
    pub max_font_faces: usize,
    pub max_font_atlas_side: u32,
    pub max_font_atlas_bytes: usize,
    pub max_emoji_count: usize,
    pub max_component_count: usize,
    pub max_name_len: usize,
}

impl UiAssetLimits {
    pub const DEFAULT: Self = Self {
        max_icon_count: 256,
        max_icon_atlas_side: 4096,
        max_icon_atlas_bytes: 16 * 1024 * 1024,
        max_font_faces: 8,
        max_font_atlas_side: 4096,
        max_font_atlas_bytes: 16 * 1024 * 1024,
        max_emoji_count: 256,
        max_component_count: 512,
        max_name_len: 96,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiAssetPackError {
    EmptyName,
    NameTooLong,
    IconCountExceeded,
    MissingRequiredIcon(UiIcon),
    DuplicateIcon(UiIcon),
    IconProviderNameMismatch(UiIcon),
    InvalidIconAtlas,
    FontFaceCountExceeded,
    MissingDefaultFontFace,
    MissingFontChar(char),
    InvalidFontAtlas,
    EmojiCountExceeded,
    MissingRequiredEmoji(&'static str),
    DuplicateEmoji(&'static str),
    ComponentCountExceeded,
    MissingComponentKind(UiExtractedComponentKind),
    DuplicateComponent(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiIconPackEntry {
    pub icon: UiIcon,
    pub provider_name: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiIconPackSpec {
    pub name: &'static str,
    pub provider: UiIconSet,
    pub entries: &'static [UiIconPackEntry],
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub atlas_bytes: usize,
}

impl UiIconPackSpec {
    pub fn validate(self, limits: UiAssetLimits) -> Result<(), UiAssetPackError> {
        validate_name(self.name, limits)?;
        if self.entries.len() > limits.max_icon_count {
            return Err(UiAssetPackError::IconCountExceeded);
        }
        if self.atlas_width == 0
            || self.atlas_height == 0
            || self.atlas_width > limits.max_icon_atlas_side
            || self.atlas_height > limits.max_icon_atlas_side
            || self.atlas_bytes == 0
            || self.atlas_bytes > limits.max_icon_atlas_bytes
        {
            return Err(UiAssetPackError::InvalidIconAtlas);
        }
        for (index, entry) in self.entries.iter().enumerate() {
            for other in self.entries.iter().skip(index + 1) {
                if entry.icon == other.icon {
                    return Err(UiAssetPackError::DuplicateIcon(entry.icon));
                }
            }
        }
        for entry in self.entries {
            if entry.provider_name != entry.icon.provider_name(self.provider) {
                return Err(UiAssetPackError::IconProviderNameMismatch(entry.icon));
            }
        }
        for icon in UiIcon::ALL {
            if !self.entries.iter().any(|entry| entry.icon == *icon) {
                return Err(UiAssetPackError::MissingRequiredIcon(*icon));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiFontFaceSpec {
    pub name: &'static str,
    pub default_face: bool,
    pub covered_chars: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiFontPackSpec {
    pub name: &'static str,
    pub faces: &'static [UiFontFaceSpec],
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub atlas_bytes: usize,
}

impl UiFontPackSpec {
    pub fn validate(self, limits: UiAssetLimits) -> Result<(), UiAssetPackError> {
        validate_name(self.name, limits)?;
        if self.faces.is_empty() || self.faces.len() > limits.max_font_faces {
            return Err(UiAssetPackError::FontFaceCountExceeded);
        }
        if self.atlas_width == 0
            || self.atlas_height == 0
            || self.atlas_width > limits.max_font_atlas_side
            || self.atlas_height > limits.max_font_atlas_side
            || self.atlas_bytes == 0
            || self.atlas_bytes > limits.max_font_atlas_bytes
        {
            return Err(UiAssetPackError::InvalidFontAtlas);
        }
        let Some(default_face) = self.faces.iter().find(|face| face.default_face) else {
            return Err(UiAssetPackError::MissingDefaultFontFace);
        };
        for ch in REQUIRED_FONT_CHARS.chars() {
            if !default_face.covered_chars.contains(ch) {
                return Err(UiAssetPackError::MissingFontChar(ch));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiEmojiSpec {
    pub key: &'static str,
    pub label: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiEmojiPackSpec {
    pub name: &'static str,
    pub emoji: &'static [UiEmojiSpec],
}

impl UiEmojiPackSpec {
    pub fn validate(self, limits: UiAssetLimits) -> Result<(), UiAssetPackError> {
        validate_name(self.name, limits)?;
        if self.emoji.len() > limits.max_emoji_count {
            return Err(UiAssetPackError::EmojiCountExceeded);
        }
        for (index, emoji) in self.emoji.iter().enumerate() {
            for other in self.emoji.iter().skip(index + 1) {
                if emoji.key == other.key {
                    return Err(UiAssetPackError::DuplicateEmoji(emoji.key));
                }
            }
        }
        for required in REQUIRED_UI_EMOJI {
            if !self.emoji.iter().any(|emoji| emoji.key == *required) {
                return Err(UiAssetPackError::MissingRequiredEmoji(required));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiComponentPackEntry {
    pub name: &'static str,
    pub kind: UiExtractedComponentKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiComponentPackSpec {
    pub name: &'static str,
    pub components: &'static [UiComponentPackEntry],
}

impl UiComponentPackSpec {
    pub fn validate(self, limits: UiAssetLimits) -> Result<(), UiAssetPackError> {
        validate_name(self.name, limits)?;
        if self.components.len() > limits.max_component_count {
            return Err(UiAssetPackError::ComponentCountExceeded);
        }
        for (index, component) in self.components.iter().enumerate() {
            for other in self.components.iter().skip(index + 1) {
                if component.name == other.name {
                    return Err(UiAssetPackError::DuplicateComponent(component.name));
                }
            }
        }
        for kind in EXTRACTED_COMPONENT_KINDS {
            if !self
                .components
                .iter()
                .any(|component| component.kind == kind)
            {
                return Err(UiAssetPackError::MissingComponentKind(kind));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiAssetPackSpec {
    pub name: &'static str,
    pub icons: UiIconPackSpec,
    pub fonts: UiFontPackSpec,
    pub emoji: UiEmojiPackSpec,
    pub components: UiComponentPackSpec,
}

impl UiAssetPackSpec {
    pub fn validate(self, limits: UiAssetLimits) -> Result<(), UiAssetPackError> {
        validate_name(self.name, limits)?;
        self.icons.validate(limits)?;
        self.fonts.validate(limits)?;
        self.emoji.validate(limits)?;
        self.components.validate(limits)?;
        Ok(())
    }
}

pub const EDGERUN_TABLER_INTER_ASSET_PACK: UiAssetPackSpec = UiAssetPackSpec {
    name: "edgerun-tabler-inter",
    icons: UiIconPackSpec {
        name: "tabler-svg",
        provider: UiIconSet::Tabler,
        entries: EDGERUN_TABLER_ICON_PACK_ENTRIES,
        atlas_width: 672,
        atlas_height: 560,
        atlas_bytes: 672 * 560,
    },
    fonts: UiFontPackSpec {
        name: "inter",
        faces: EDGERUN_INTER_FONT_FACES,
        atlas_width: 1024,
        atlas_height: 1024,
        atlas_bytes: 1024 * 1024,
    },
    emoji: UiEmojiPackSpec {
        name: "edgerun-semantic-emoji",
        emoji: EDGERUN_REQUIRED_EMOJI,
    },
    components: UiComponentPackSpec {
        name: "edgerun-components",
        components: EDGERUN_COMPONENT_PACK_ENTRIES,
    },
};

pub const EDGERUN_LUCIDE_GEIST_ASSET_PACK: UiAssetPackSpec = UiAssetPackSpec {
    name: "edgerun-lucide-geist",
    icons: UiIconPackSpec {
        name: "lucide-svg",
        provider: UiIconSet::Lucide,
        entries: EDGERUN_LUCIDE_ICON_PACK_ENTRIES,
        atlas_width: 672,
        atlas_height: 560,
        atlas_bytes: 672 * 560,
    },
    fonts: UiFontPackSpec {
        name: "geist",
        faces: EDGERUN_GEIST_FONT_FACES,
        atlas_width: 1024,
        atlas_height: 1024,
        atlas_bytes: 1024 * 1024,
    },
    emoji: UiEmojiPackSpec {
        name: "edgerun-semantic-emoji",
        emoji: EDGERUN_REQUIRED_EMOJI,
    },
    components: UiComponentPackSpec {
        name: "edgerun-components",
        components: EDGERUN_COMPONENT_PACK_ENTRIES,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiAssetPackRuntime {
    active: UiAssetPackSpec,
    limits: UiAssetLimits,
}

impl UiAssetPackRuntime {
    pub fn new(active: UiAssetPackSpec, limits: UiAssetLimits) -> Result<Self, UiAssetPackError> {
        active.validate(limits)?;
        Ok(Self { active, limits })
    }

    pub fn active(&self) -> UiAssetPackSpec {
        self.active
    }

    pub fn limits(&self) -> UiAssetLimits {
        self.limits
    }

    pub fn replace(&mut self, replacement: UiAssetPackSpec) -> Result<(), UiAssetPackError> {
        replacement.validate(self.limits)?;
        self.active = replacement;
        Ok(())
    }
}

fn validate_name(name: &str, limits: UiAssetLimits) -> Result<(), UiAssetPackError> {
    if name.is_empty() {
        return Err(UiAssetPackError::EmptyName);
    }
    if name.len() > limits.max_name_len {
        return Err(UiAssetPackError::NameTooLong);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ICON_ENTRIES: &[UiIconPackEntry] = &[
        UiIconPackEntry {
            icon: UiIcon::Activity,
            provider_name: "activity",
        },
        UiIconPackEntry {
            icon: UiIcon::App,
            provider_name: "apps",
        },
        UiIconPackEntry {
            icon: UiIcon::Bell,
            provider_name: "bell",
        },
        UiIconPackEntry {
            icon: UiIcon::Chat,
            provider_name: "message-circle",
        },
        UiIconPackEntry {
            icon: UiIcon::Check,
            provider_name: "check",
        },
        UiIconPackEntry {
            icon: UiIcon::ChevronRight,
            provider_name: "chevron-right",
        },
        UiIconPackEntry {
            icon: UiIcon::Code,
            provider_name: "code",
        },
        UiIconPackEntry {
            icon: UiIcon::Cpu,
            provider_name: "cpu",
        },
        UiIconPackEntry {
            icon: UiIcon::Database,
            provider_name: "database",
        },
        UiIconPackEntry {
            icon: UiIcon::Eye,
            provider_name: "eye",
        },
        UiIconPackEntry {
            icon: UiIcon::File,
            provider_name: "file",
        },
        UiIconPackEntry {
            icon: UiIcon::Key,
            provider_name: "key",
        },
        UiIconPackEntry {
            icon: UiIcon::Lock,
            provider_name: "lock",
        },
        UiIconPackEntry {
            icon: UiIcon::Menu,
            provider_name: "menu-2",
        },
        UiIconPackEntry {
            icon: UiIcon::MessagePlus,
            provider_name: "message-plus",
        },
        UiIconPackEntry {
            icon: UiIcon::Network,
            provider_name: "network",
        },
        UiIconPackEntry {
            icon: UiIcon::Route,
            provider_name: "route",
        },
        UiIconPackEntry {
            icon: UiIcon::Search,
            provider_name: "search",
        },
        UiIconPackEntry {
            icon: UiIcon::Send,
            provider_name: "arrow-up",
        },
        UiIconPackEntry {
            icon: UiIcon::Server,
            provider_name: "server",
        },
        UiIconPackEntry {
            icon: UiIcon::Settings,
            provider_name: "settings",
        },
        UiIconPackEntry {
            icon: UiIcon::Shield,
            provider_name: "shield-check",
        },
        UiIconPackEntry {
            icon: UiIcon::Sparkles,
            provider_name: "sparkles",
        },
        UiIconPackEntry {
            icon: UiIcon::Storage,
            provider_name: "database",
        },
        UiIconPackEntry {
            icon: UiIcon::Terminal,
            provider_name: "terminal-2",
        },
        UiIconPackEntry {
            icon: UiIcon::Trust,
            provider_name: "shield-check",
        },
        UiIconPackEntry {
            icon: UiIcon::Trash,
            provider_name: "trash",
        },
        UiIconPackEntry {
            icon: UiIcon::User,
            provider_name: "user",
        },
        UiIconPackEntry {
            icon: UiIcon::Wallet,
            provider_name: "wallet",
        },
        UiIconPackEntry {
            icon: UiIcon::Warning,
            provider_name: "alert-triangle",
        },
        UiIconPackEntry {
            icon: UiIcon::X,
            provider_name: "x",
        },
    ];

    const FONT_FACES: &[UiFontFaceSpec] = &[UiFontFaceSpec {
        name: "Inter",
        default_face: true,
        covered_chars: REQUIRED_FONT_CHARS,
    }];
    const BAD_FONT_FACES: &[UiFontFaceSpec] = &[UiFontFaceSpec {
        name: "Inter",
        default_face: true,
        covered_chars: "abc",
    }];

    const EMOJI: &[UiEmojiSpec] = &[
        UiEmojiSpec {
            key: "check",
            label: "check",
        },
        UiEmojiSpec {
            key: "warning",
            label: "warning",
        },
        UiEmojiSpec {
            key: "locked",
            label: "locked",
        },
        UiEmojiSpec {
            key: "unlocked",
            label: "unlocked",
        },
        UiEmojiSpec {
            key: "route",
            label: "route",
        },
        UiEmojiSpec {
            key: "storage",
            label: "storage",
        },
        UiEmojiSpec {
            key: "payment",
            label: "payment",
        },
        UiEmojiSpec {
            key: "proof",
            label: "proof",
        },
    ];

    const COMPONENTS: &[UiComponentPackEntry] = &[
        UiComponentPackEntry {
            name: "shell",
            kind: UiExtractedComponentKind::Shell,
        },
        UiComponentPackEntry {
            name: "layout",
            kind: UiExtractedComponentKind::Layout,
        },
        UiComponentPackEntry {
            name: "navigation",
            kind: UiExtractedComponentKind::Navigation,
        },
        UiComponentPackEntry {
            name: "overlay",
            kind: UiExtractedComponentKind::Overlay,
        },
        UiComponentPackEntry {
            name: "card",
            kind: UiExtractedComponentKind::Card,
        },
        UiComponentPackEntry {
            name: "form",
            kind: UiExtractedComponentKind::Form,
        },
        UiComponentPackEntry {
            name: "input-group",
            kind: UiExtractedComponentKind::InputGroup,
        },
        UiComponentPackEntry {
            name: "chart",
            kind: UiExtractedComponentKind::Chart,
        },
        UiComponentPackEntry {
            name: "data-row",
            kind: UiExtractedComponentKind::DataRow,
        },
        UiComponentPackEntry {
            name: "control",
            kind: UiExtractedComponentKind::Control,
        },
        UiComponentPackEntry {
            name: "selection",
            kind: UiExtractedComponentKind::Selection,
        },
        UiComponentPackEntry {
            name: "feedback",
            kind: UiExtractedComponentKind::Feedback,
        },
        UiComponentPackEntry {
            name: "domain",
            kind: UiExtractedComponentKind::EdgeRunDomain,
        },
    ];

    fn valid_pack() -> UiAssetPackSpec {
        UiAssetPackSpec {
            name: "valid",
            icons: UiIconPackSpec {
                name: "icons",
                provider: UiIconSet::Tabler,
                entries: ICON_ENTRIES,
                atlas_width: 1024,
                atlas_height: 1024,
                atlas_bytes: 1024 * 1024,
            },
            fonts: UiFontPackSpec {
                name: "fonts",
                faces: FONT_FACES,
                atlas_width: 1024,
                atlas_height: 1024,
                atlas_bytes: 1024 * 1024,
            },
            emoji: UiEmojiPackSpec {
                name: "emoji",
                emoji: EMOJI,
            },
            components: UiComponentPackSpec {
                name: "components",
                components: COMPONENTS,
            },
        }
    }

    #[test]
    fn complete_asset_pack_validates_under_deterministic_limits() {
        assert_eq!(valid_pack().validate(UiAssetLimits::DEFAULT), Ok(()));
    }

    #[test]
    fn bundled_tabler_inter_and_lucide_geist_packs_are_valid() {
        assert_eq!(
            EDGERUN_TABLER_INTER_ASSET_PACK.validate(UiAssetLimits::DEFAULT),
            Ok(())
        );
        assert_eq!(
            EDGERUN_LUCIDE_GEIST_ASSET_PACK.validate(UiAssetLimits::DEFAULT),
            Ok(())
        );
        assert_eq!(
            EDGERUN_LUCIDE_GEIST_ASSET_PACK.icons.provider,
            UiIconSet::Lucide
        );
        assert_eq!(EDGERUN_LUCIDE_GEIST_ASSET_PACK.fonts.faces[0].name, "Geist");
    }

    #[test]
    fn runtime_asset_pack_replacement_accepts_only_valid_packs() {
        let mut runtime =
            UiAssetPackRuntime::new(EDGERUN_TABLER_INTER_ASSET_PACK, UiAssetLimits::DEFAULT)
                .expect("valid default pack");
        assert_eq!(runtime.active().name, "edgerun-tabler-inter");

        runtime
            .replace(EDGERUN_LUCIDE_GEIST_ASSET_PACK)
            .expect("valid Lucide/Geist replacement pack");
        assert_eq!(runtime.active().name, "edgerun-lucide-geist");

        let mut invalid = EDGERUN_LUCIDE_GEIST_ASSET_PACK;
        invalid.icons.entries =
            &EDGERUN_LUCIDE_ICON_PACK_ENTRIES[..EDGERUN_LUCIDE_ICON_PACK_ENTRIES.len() - 1];
        assert_eq!(
            runtime.replace(invalid),
            Err(UiAssetPackError::MissingRequiredIcon(UiIcon::X))
        );
        assert_eq!(runtime.active().name, "edgerun-lucide-geist");
    }

    #[test]
    fn icon_pack_rejects_missing_required_icon() {
        let mut pack = valid_pack();
        pack.icons.entries = &ICON_ENTRIES[..ICON_ENTRIES.len() - 1];
        assert_eq!(
            pack.validate(UiAssetLimits::DEFAULT),
            Err(UiAssetPackError::MissingRequiredIcon(UiIcon::X))
        );
    }

    #[test]
    fn icon_pack_rejects_provider_name_mismatch() {
        let entries = &[
            UiIconPackEntry {
                icon: UiIcon::Activity,
                provider_name: "activity",
            },
            UiIconPackEntry {
                icon: UiIcon::App,
                provider_name: "apps",
            },
        ];
        let pack = UiIconPackSpec {
            name: "bad-icons",
            provider: UiIconSet::Lucide,
            entries,
            atlas_width: 64,
            atlas_height: 64,
            atlas_bytes: 4096,
        };
        assert_eq!(
            pack.validate(UiAssetLimits::DEFAULT),
            Err(UiAssetPackError::IconProviderNameMismatch(UiIcon::App))
        );
    }

    #[test]
    fn font_pack_rejects_missing_required_character() {
        let mut pack = valid_pack();
        pack.fonts.faces = BAD_FONT_FACES;
        assert_eq!(
            pack.validate(UiAssetLimits::DEFAULT),
            Err(UiAssetPackError::MissingFontChar('A'))
        );
    }

    #[test]
    fn emoji_pack_rejects_missing_required_semantic_emoji() {
        let mut pack = valid_pack();
        pack.emoji.emoji = &EMOJI[..EMOJI.len() - 1];
        assert_eq!(
            pack.validate(UiAssetLimits::DEFAULT),
            Err(UiAssetPackError::MissingRequiredEmoji("proof"))
        );
    }

    #[test]
    fn component_pack_rejects_missing_required_kind() {
        let mut pack = valid_pack();
        pack.components.components = &COMPONENTS[..COMPONENTS.len() - 1];
        assert_eq!(
            pack.validate(UiAssetLimits::DEFAULT),
            Err(UiAssetPackError::MissingComponentKind(
                UiExtractedComponentKind::EdgeRunDomain
            ))
        );
    }
}
