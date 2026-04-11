//! CSS keyword parsing — common keywords and CSS-wide keywords.

/// CSS-wide keywords that apply to any property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssWideKeyword {
    Inherit,
    Initial,
    Unset,
    Revert,
    RevertLayer,
}

/// Common CSS property value keywords.
/// These are NOT CSS-wide (they don't apply to every property),
/// but are recognized values for specific properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssKeyword {
    // Layout
    Auto, None, Block, Inline, Flex, Grid, Contents, ListItem, TableRow,
    TableCell, TableHeaderGroup, TableFooterGroup, TableCaption,
    Static, Relative, Absolute, Fixed, Sticky,
    Visible, Hidden, Scroll, Clip,
    // Text
    Left, Right, Center, Justify, Start, End,
    Bold, Bolder, Lighter, Normal, Italic, Oblique,
    Uppercase, Lowercase, Capitalize,
    Underline, Overline, LineThrough,
    // Flex/Grid
    Row, RowReverse, Column, ColumnReverse,
    Wrap, WrapWrap, Nowrap,
    FlexStart, FlexEnd, SpaceBetween, SpaceAround, SpaceEvenly,
    Stretch, Baseline,
    // Borders
    Solid, Dashed, Dotted, Double, Groove, Ridge, Inset, Outset,
    // Background
    Cover, Contain,
    BorderBox, PaddingBox, ContentBox,
    // Images
    Repeat, RepeatX, RepeatY, NoRepeat, Round, Space,
    // Transitions
    Linear, Ease, EaseIn, EaseOut, EaseInOut, StepStart, StepEnd,
    // Misc
    Pointer, Help, Move, Text, Wait, Crosshair,
    Default, ContextMenu, Progress, Cell, Alias,
    Grab, Grabbing, ColResize, RowResize,
    AllowDrop, NoDrop, NotAllowed,
    Verbose, Quiet,
    // CSS-wide (also tracked separately)
    Inherit, Initial, Unset, Revert, RevertLayer,
    CurrentColor, Transparent,
}

/// Parse a CSS-wide keyword: `inherit`, `initial`, `unset`, `revert`, `revert-layer`.
pub fn parse_css_wide_keyword(input: &str) -> Option<CssWideKeyword> {
    match input.to_lowercase().as_str() {
        "inherit" => Some(CssWideKeyword::Inherit),
        "initial" => Some(CssWideKeyword::Initial),
        "unset" => Some(CssWideKeyword::Unset),
        "revert" => Some(CssWideKeyword::Revert),
        "revert-layer" => Some(CssWideKeyword::RevertLayer),
        _ => None,
    }
}

/// Parse a CSS keyword value.
/// Returns None if the string doesn't match any known keyword.
pub fn parse_keyword(input: &str) -> Option<CssKeyword> {
    match input.to_lowercase().as_str() {
        "auto" => Some(CssKeyword::Auto),
        "none" => Some(CssKeyword::None),
        "block" => Some(CssKeyword::Block),
        "inline" => Some(CssKeyword::Inline),
        "flex" => Some(CssKeyword::Flex),
        "grid" => Some(CssKeyword::Grid),
        "contents" => Some(CssKeyword::Contents),
        "list-item" => Some(CssKeyword::ListItem),
        "table-row" => Some(CssKeyword::TableRow),
        "table-cell" => Some(CssKeyword::TableCell),
        "table-header-group" => Some(CssKeyword::TableHeaderGroup),
        "table-footer-group" => Some(CssKeyword::TableFooterGroup),
        "table-caption" => Some(CssKeyword::TableCaption),
        "static" => Some(CssKeyword::Static),
        "relative" => Some(CssKeyword::Relative),
        "absolute" => Some(CssKeyword::Absolute),
        "fixed" => Some(CssKeyword::Fixed),
        "sticky" => Some(CssKeyword::Sticky),
        "visible" => Some(CssKeyword::Visible),
        "hidden" => Some(CssKeyword::Hidden),
        "scroll" => Some(CssKeyword::Scroll),
        "clip" => Some(CssKeyword::Clip),
        "left" => Some(CssKeyword::Left),
        "right" => Some(CssKeyword::Right),
        "center" => Some(CssKeyword::Center),
        "justify" => Some(CssKeyword::Justify),
        "start" => Some(CssKeyword::Start),
        "end" => Some(CssKeyword::End),
        "bold" => Some(CssKeyword::Bold),
        "bolder" => Some(CssKeyword::Bolder),
        "lighter" => Some(CssKeyword::Lighter),
        "normal" => Some(CssKeyword::Normal),
        "italic" => Some(CssKeyword::Italic),
        "oblique" => Some(CssKeyword::Oblique),
        "uppercase" => Some(CssKeyword::Uppercase),
        "lowercase" => Some(CssKeyword::Lowercase),
        "capitalize" => Some(CssKeyword::Capitalize),
        "underline" => Some(CssKeyword::Underline),
        "overline" => Some(CssKeyword::Overline),
        "line-through" => Some(CssKeyword::LineThrough),
        "row" => Some(CssKeyword::Row),
        "row-reverse" => Some(CssKeyword::RowReverse),
        "column" => Some(CssKeyword::Column),
        "column-reverse" => Some(CssKeyword::ColumnReverse),
        "wrap" => Some(CssKeyword::Wrap),
        "nowrap" => Some(CssKeyword::Nowrap),
        "flex-start" => Some(CssKeyword::FlexStart),
        "flex-end" => Some(CssKeyword::FlexEnd),
        "space-between" => Some(CssKeyword::SpaceBetween),
        "space-around" => Some(CssKeyword::SpaceAround),
        "space-evenly" => Some(CssKeyword::SpaceEvenly),
        "stretch" => Some(CssKeyword::Stretch),
        "baseline" => Some(CssKeyword::Baseline),
        "solid" => Some(CssKeyword::Solid),
        "dashed" => Some(CssKeyword::Dashed),
        "dotted" => Some(CssKeyword::Dotted),
        "double" => Some(CssKeyword::Double),
        "groove" => Some(CssKeyword::Groove),
        "ridge" => Some(CssKeyword::Ridge),
        "inset" => Some(CssKeyword::Inset),
        "outset" => Some(CssKeyword::Outset),
        "cover" => Some(CssKeyword::Cover),
        "contain" => Some(CssKeyword::Contain),
        "border-box" => Some(CssKeyword::BorderBox),
        "padding-box" => Some(CssKeyword::PaddingBox),
        "content-box" => Some(CssKeyword::ContentBox),
        "repeat" => Some(CssKeyword::Repeat),
        "repeat-x" => Some(CssKeyword::RepeatX),
        "repeat-y" => Some(CssKeyword::RepeatY),
        "no-repeat" => Some(CssKeyword::NoRepeat),
        "linear" => Some(CssKeyword::Linear),
        "ease" => Some(CssKeyword::Ease),
        "ease-in" => Some(CssKeyword::EaseIn),
        "ease-out" => Some(CssKeyword::EaseOut),
        "ease-in-out" => Some(CssKeyword::EaseInOut),
        "step-start" => Some(CssKeyword::StepStart),
        "step-end" => Some(CssKeyword::StepEnd),
        "pointer" => Some(CssKeyword::Pointer),
        "help" => Some(CssKeyword::Help),
        "move" => Some(CssKeyword::Move),
        "text" => Some(CssKeyword::Text),
        "wait" => Some(CssKeyword::Wait),
        "crosshair" => Some(CssKeyword::Crosshair),
        "default" => Some(CssKeyword::Default),
        "context-menu" => Some(CssKeyword::ContextMenu),
        "progress" => Some(CssKeyword::Progress),
        "cell" => Some(CssKeyword::Cell),
        "alias" => Some(CssKeyword::Alias),
        "grab" => Some(CssKeyword::Grab),
        "grabbing" => Some(CssKeyword::Grabbing),
        "col-resize" => Some(CssKeyword::ColResize),
        "row-resize" => Some(CssKeyword::RowResize),
        "not-allowed" => Some(CssKeyword::NotAllowed),
        "no-drop" => Some(CssKeyword::NoDrop),
        // CSS-wide keywords (also parseable here for convenience)
        "inherit" => Some(CssKeyword::Inherit),
        "initial" => Some(CssKeyword::Initial),
        "unset" => Some(CssKeyword::Unset),
        "revert" => Some(CssKeyword::Revert),
        "revert-layer" => Some(CssKeyword::RevertLayer),
        "currentcolor" => Some(CssKeyword::CurrentColor),
        "transparent" => Some(CssKeyword::Transparent),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wide_keyword_inherit() {
        assert_eq!(parse_css_wide_keyword("inherit"), Some(CssWideKeyword::Inherit));
        assert_eq!(parse_css_wide_keyword("Inherit"), Some(CssWideKeyword::Inherit));
    }

    #[test]
    fn test_wide_keyword_initial() {
        assert_eq!(parse_css_wide_keyword("initial"), Some(CssWideKeyword::Initial));
    }

    #[test]
    fn test_wide_keyword_unset() {
        assert_eq!(parse_css_wide_keyword("unset"), Some(CssWideKeyword::Unset));
    }

    #[test]
    fn test_wide_keyword_revert() {
        assert_eq!(parse_css_wide_keyword("revert"), Some(CssWideKeyword::Revert));
        assert_eq!(parse_css_wide_keyword("revert-layer"), Some(CssWideKeyword::RevertLayer));
    }

    #[test]
    fn test_wide_keyword_invalid() {
        assert!(parse_css_wide_keyword("auto").is_none());
        assert!(parse_css_wide_keyword("bold").is_none());
    }

    #[test]
    fn test_keyword_auto() {
        assert_eq!(parse_keyword("auto"), Some(CssKeyword::Auto));
    }

    #[test]
    fn test_keyword_none() {
        assert_eq!(parse_keyword("none"), Some(CssKeyword::None));
    }

    #[test]
    fn test_keyword_bold() {
        assert_eq!(parse_keyword("bold"), Some(CssKeyword::Bold));
    }

    #[test]
    fn test_keyword_case_insensitive() {
        assert_eq!(parse_keyword("Auto"), Some(CssKeyword::Auto));
        assert_eq!(parse_keyword("AUTO"), Some(CssKeyword::Auto));
    }

    #[test]
    fn test_keyword_flex() {
        assert_eq!(parse_keyword("flex"), Some(CssKeyword::Flex));
    }

    #[test]
    fn test_keyword_invalid() {
        assert!(parse_keyword("32px").is_none());
        assert!(parse_keyword("#ff0000").is_none());
    }
}
