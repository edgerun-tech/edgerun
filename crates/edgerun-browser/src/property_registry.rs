//! CSS Property Registry — generated from W3C CSS specifications.
//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssPropertyId {
    Unspecified = 0,
    AnimationName = 1,
    AnimationDuration = 2,
    AnimationTimingFunction = 3,
    AnimationIterationCount = 4,
    AnimationDirection = 5,
    AnimationPlayState = 6,
    AnimationDelay = 7,
    AnimationFillMode = 8,
    Animation = 9,
    BackgroundColor = 10,
    BackgroundImage = 11,
    BackgroundRepeat = 12,
    BackgroundAttachment = 13,
    BackgroundPosition = 14,
    BackgroundClip = 15,
    BackgroundOrigin = 16,
    BackgroundSize = 17,
    Background = 18,
    BorderTopColor = 19,
    BorderColor = 20,
    BorderTopStyle = 21,
    BorderStyle = 22,
    BorderTopWidth = 23,
    BorderWidth = 24,
    BorderTop = 25,
    Border = 26,
    BorderTopLeftRadius = 27,
    BorderRadius = 28,
    BorderImageSource = 29,
    BorderImageSlice = 30,
    BorderImageWidth = 31,
    BorderImageOutset = 32,
    BorderImageRepeat = 33,
    BorderImage = 34,
    BoxShadow = 35,
    MarginTop = 36,
    Margin = 37,
    MarginTrim = 38,
    PaddingTop = 39,
    Padding = 40,
    Opacity = 41,
    Contain = 42,
    ContentVisibility = 43,
    Display = 44,
    Visibility = 45,
    FlexDirection = 46,
    FlexWrap = 47,
    FlexFlow = 48,
    FlexGrow = 49,
    FlexShrink = 50,
    FlexBasis = 51,
    JustifyContent = 52,
    AlignItems = 53,
    AlignSelf = 54,
    AlignContent = 55,
    FontFamily = 56,
    FontWeight = 57,
    FontWidth = 58,
    FontStyle = 59,
    FontSize = 60,
    FontSizeAdjust = 61,
    FontSynthesisWeight = 62,
    FontSynthesisStyle = 63,
    FontSynthesisSmallCaps = 64,
    FontSynthesisPosition = 65,
    FontSynthesis = 66,
    FontKerning = 67,
    FontVariantLigatures = 68,
    FontVariantPosition = 69,
    FontVariantCaps = 70,
    FontVariantNumeric = 71,
    FontVariantAlternates = 72,
    FontVariantEastAsian = 73,
    FontVariant = 74,
    FontFeatureSettings = 75,
    FontLanguageOverride = 76,
    FontOpticalSizing = 77,
    FontVariationSettings = 78,
    FontPalette = 79,
    FontVariantEmoji = 80,
    GridTemplateColumns = 81,
    GridTemplateAreas = 82,
    GridTemplate = 83,
    GridAutoColumns = 84,
    GridAutoFlow = 85,
    GridRowStart = 86,
    GridRow = 87,
    GridArea = 88,
    ObjectFit = 89,
    ObjectPosition = 90,
    ImageOrientation = 91,
    ImageRendering = 92,
    OverflowX = 93,
    Overflow = 94,
    OverflowClipMargin = 95,
    ScrollBehavior = 96,
    ScrollbarGutter = 97,
    TextOverflow = 98,
    MinWidth = 99,
    MaxWidth = 100,
    BoxSizing = 101,
    TextTransform = 102,
    WhiteSpace = 103,
    TabSize = 104,
    WordBreak = 105,
    LineBreak = 106,
    Hyphens = 107,
    OverflowWrap = 108,
    TextAlign = 109,
    TextAlignAll = 110,
    TextAlignLast = 111,
    TextJustify = 112,
    WordSpacing = 113,
    LetterSpacing = 114,
    TextIndent = 115,
    HangingPunctuation = 116,
    Transform = 117,
    TransformOrigin = 118,
    TransformBox = 119,
    TransitionProperty = 120,
    TransitionDuration = 121,
    TransitionDelay = 122,
    Transition = 123,
    Outline = 124,
    OutlineWidth = 125,
    OutlineStyle = 126,
    OutlineColor = 127,
    OutlineOffset = 128,
    Resize = 129,
    Cursor = 130,
    CaretColor = 131,
    CaretAnimation = 132,
    CaretShape = 133,
    NavUp = 134,
    UserSelect = 135,
    PointerEvents = 136,
    Interactivity = 137,
    InterestDelayStart = 138,
    InterestDelay = 139,
    AccentColor = 140,
    Appearance = 141,
    Direction = 142,
    UnicodeBidi = 143,
    WritingMode = 144,
    TextOrientation = 145,
    TextCombineUpright = 146,
    PropertyName = 147,
    MarginRight = 148,
    Position = 149,
    Bottom = 150,
    ZIndex = 151,
    Height = 152,
    MinHeight = 153,
    MaxHeight = 154,
    LineHeight = 155,
    VerticalAlign = 156,
    Content = 157,
    Quotes = 158,
    CounterReset = 159,
    CounterIncrement = 160,
    ListStyleType = 161,
    ListStyleImage = 162,
    ListStylePosition = 163,
    ListStyle = 164,
    PageBreakBefore = 165,
    PageBreakAfter = 166,
    PageBreakInside = 167,
    Orphans = 168,
    Widows = 169,
    TextDecoration = 170,
    CaptionSide = 171,
    TableLayout = 172,
    BorderCollapse = 173,
    BorderSpacing = 174,
    EmptyCells = 175,
}

impl CssPropertyId {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "animation-name" => Some(CssPropertyId::AnimationName),
            "animation-duration" => Some(CssPropertyId::AnimationDuration),
            "animation-timing-function" => Some(CssPropertyId::AnimationTimingFunction),
            "animation-iteration-count" => Some(CssPropertyId::AnimationIterationCount),
            "animation-direction" => Some(CssPropertyId::AnimationDirection),
            "animation-play-state" => Some(CssPropertyId::AnimationPlayState),
            "animation-delay" => Some(CssPropertyId::AnimationDelay),
            "animation-fill-mode" => Some(CssPropertyId::AnimationFillMode),
            "animation" => Some(CssPropertyId::Animation),
            "background-color" => Some(CssPropertyId::BackgroundColor),
            "background-image" => Some(CssPropertyId::BackgroundImage),
            "background-repeat" => Some(CssPropertyId::BackgroundRepeat),
            "background-attachment" => Some(CssPropertyId::BackgroundAttachment),
            "background-position" => Some(CssPropertyId::BackgroundPosition),
            "background-clip" => Some(CssPropertyId::BackgroundClip),
            "background-origin" => Some(CssPropertyId::BackgroundOrigin),
            "background-size" => Some(CssPropertyId::BackgroundSize),
            "background" => Some(CssPropertyId::Background),
            "border-top-color" => Some(CssPropertyId::BorderTopColor),
            "border-color" => Some(CssPropertyId::BorderColor),
            "border-top-style" => Some(CssPropertyId::BorderTopStyle),
            "border-style" => Some(CssPropertyId::BorderStyle),
            "border-top-width" => Some(CssPropertyId::BorderTopWidth),
            "border-width" => Some(CssPropertyId::BorderWidth),
            "border-top" => Some(CssPropertyId::BorderTop),
            "border" => Some(CssPropertyId::Border),
            "border-top-left-radius" => Some(CssPropertyId::BorderTopLeftRadius),
            "border-radius" => Some(CssPropertyId::BorderRadius),
            "border-image-source" => Some(CssPropertyId::BorderImageSource),
            "border-image-slice" => Some(CssPropertyId::BorderImageSlice),
            "border-image-width" => Some(CssPropertyId::BorderImageWidth),
            "border-image-outset" => Some(CssPropertyId::BorderImageOutset),
            "border-image-repeat" => Some(CssPropertyId::BorderImageRepeat),
            "border-image" => Some(CssPropertyId::BorderImage),
            "box-shadow" => Some(CssPropertyId::BoxShadow),
            "margin-top" => Some(CssPropertyId::MarginTop),
            "margin" => Some(CssPropertyId::Margin),
            "margin-trim" => Some(CssPropertyId::MarginTrim),
            "padding-top" => Some(CssPropertyId::PaddingTop),
            "padding" => Some(CssPropertyId::Padding),
            "opacity" => Some(CssPropertyId::Opacity),
            "contain" => Some(CssPropertyId::Contain),
            "content-visibility" => Some(CssPropertyId::ContentVisibility),
            "display" => Some(CssPropertyId::Display),
            "visibility" => Some(CssPropertyId::Visibility),
            "flex-direction" => Some(CssPropertyId::FlexDirection),
            "flex-wrap" => Some(CssPropertyId::FlexWrap),
            "flex-flow" => Some(CssPropertyId::FlexFlow),
            "flex-grow" => Some(CssPropertyId::FlexGrow),
            "flex-shrink" => Some(CssPropertyId::FlexShrink),
            "flex-basis" => Some(CssPropertyId::FlexBasis),
            "justify-content" => Some(CssPropertyId::JustifyContent),
            "align-items" => Some(CssPropertyId::AlignItems),
            "align-self" => Some(CssPropertyId::AlignSelf),
            "align-content" => Some(CssPropertyId::AlignContent),
            "font-family" => Some(CssPropertyId::FontFamily),
            "font-weight" => Some(CssPropertyId::FontWeight),
            "font-width" => Some(CssPropertyId::FontWidth),
            "font-style" => Some(CssPropertyId::FontStyle),
            "font-size" => Some(CssPropertyId::FontSize),
            "font-size-adjust" => Some(CssPropertyId::FontSizeAdjust),
            "font-synthesis-weight" => Some(CssPropertyId::FontSynthesisWeight),
            "font-synthesis-style" => Some(CssPropertyId::FontSynthesisStyle),
            "font-synthesis-small-caps" => Some(CssPropertyId::FontSynthesisSmallCaps),
            "font-synthesis-position" => Some(CssPropertyId::FontSynthesisPosition),
            "font-synthesis" => Some(CssPropertyId::FontSynthesis),
            "font-kerning" => Some(CssPropertyId::FontKerning),
            "font-variant-ligatures" => Some(CssPropertyId::FontVariantLigatures),
            "font-variant-position" => Some(CssPropertyId::FontVariantPosition),
            "font-variant-caps" => Some(CssPropertyId::FontVariantCaps),
            "font-variant-numeric" => Some(CssPropertyId::FontVariantNumeric),
            "font-variant-alternates" => Some(CssPropertyId::FontVariantAlternates),
            "font-variant-east-asian" => Some(CssPropertyId::FontVariantEastAsian),
            "font-variant" => Some(CssPropertyId::FontVariant),
            "font-feature-settings" => Some(CssPropertyId::FontFeatureSettings),
            "font-language-override" => Some(CssPropertyId::FontLanguageOverride),
            "font-optical-sizing" => Some(CssPropertyId::FontOpticalSizing),
            "font-variation-settings" => Some(CssPropertyId::FontVariationSettings),
            "font-palette" => Some(CssPropertyId::FontPalette),
            "font-variant-emoji" => Some(CssPropertyId::FontVariantEmoji),
            "grid-template-columns" => Some(CssPropertyId::GridTemplateColumns),
            "grid-template-areas" => Some(CssPropertyId::GridTemplateAreas),
            "grid-template" => Some(CssPropertyId::GridTemplate),
            "grid-auto-columns" => Some(CssPropertyId::GridAutoColumns),
            "grid-auto-flow" => Some(CssPropertyId::GridAutoFlow),
            "grid-row-start" => Some(CssPropertyId::GridRowStart),
            "grid-row" => Some(CssPropertyId::GridRow),
            "grid-area" => Some(CssPropertyId::GridArea),
            "object-fit" => Some(CssPropertyId::ObjectFit),
            "object-position" => Some(CssPropertyId::ObjectPosition),
            "image-orientation" => Some(CssPropertyId::ImageOrientation),
            "image-rendering" => Some(CssPropertyId::ImageRendering),
            "overflow-x" => Some(CssPropertyId::OverflowX),
            "overflow" => Some(CssPropertyId::Overflow),
            "overflow-clip-margin" => Some(CssPropertyId::OverflowClipMargin),
            "scroll-behavior" => Some(CssPropertyId::ScrollBehavior),
            "scrollbar-gutter" => Some(CssPropertyId::ScrollbarGutter),
            "text-overflow" => Some(CssPropertyId::TextOverflow),
            "min-width" => Some(CssPropertyId::MinWidth),
            "max-width" => Some(CssPropertyId::MaxWidth),
            "box-sizing" => Some(CssPropertyId::BoxSizing),
            "text-transform" => Some(CssPropertyId::TextTransform),
            "white-space" => Some(CssPropertyId::WhiteSpace),
            "tab-size" => Some(CssPropertyId::TabSize),
            "word-break" => Some(CssPropertyId::WordBreak),
            "line-break" => Some(CssPropertyId::LineBreak),
            "hyphens" => Some(CssPropertyId::Hyphens),
            "overflow-wrap" => Some(CssPropertyId::OverflowWrap),
            "text-align" => Some(CssPropertyId::TextAlign),
            "text-align-all" => Some(CssPropertyId::TextAlignAll),
            "text-align-last" => Some(CssPropertyId::TextAlignLast),
            "text-justify" => Some(CssPropertyId::TextJustify),
            "word-spacing" => Some(CssPropertyId::WordSpacing),
            "letter-spacing" => Some(CssPropertyId::LetterSpacing),
            "text-indent" => Some(CssPropertyId::TextIndent),
            "hanging-punctuation" => Some(CssPropertyId::HangingPunctuation),
            "transform" => Some(CssPropertyId::Transform),
            "transform-origin" => Some(CssPropertyId::TransformOrigin),
            "transform-box" => Some(CssPropertyId::TransformBox),
            "transition-property" => Some(CssPropertyId::TransitionProperty),
            "transition-duration" => Some(CssPropertyId::TransitionDuration),
            "transition-delay" => Some(CssPropertyId::TransitionDelay),
            "transition" => Some(CssPropertyId::Transition),
            "outline" => Some(CssPropertyId::Outline),
            "outline-width" => Some(CssPropertyId::OutlineWidth),
            "outline-style" => Some(CssPropertyId::OutlineStyle),
            "outline-color" => Some(CssPropertyId::OutlineColor),
            "outline-offset" => Some(CssPropertyId::OutlineOffset),
            "resize" => Some(CssPropertyId::Resize),
            "cursor" => Some(CssPropertyId::Cursor),
            "caret-color" => Some(CssPropertyId::CaretColor),
            "caret-animation" => Some(CssPropertyId::CaretAnimation),
            "caret-shape" => Some(CssPropertyId::CaretShape),
            "nav-up" => Some(CssPropertyId::NavUp),
            "user-select" => Some(CssPropertyId::UserSelect),
            "pointer-events" => Some(CssPropertyId::PointerEvents),
            "interactivity" => Some(CssPropertyId::Interactivity),
            "interest-delay-start" => Some(CssPropertyId::InterestDelayStart),
            "interest-delay" => Some(CssPropertyId::InterestDelay),
            "accent-color" => Some(CssPropertyId::AccentColor),
            "appearance" => Some(CssPropertyId::Appearance),
            "direction" => Some(CssPropertyId::Direction),
            "unicode-bidi" => Some(CssPropertyId::UnicodeBidi),
            "writing-mode" => Some(CssPropertyId::WritingMode),
            "text-orientation" => Some(CssPropertyId::TextOrientation),
            "text-combine-upright" => Some(CssPropertyId::TextCombineUpright),
            "property-name" => Some(CssPropertyId::PropertyName),
            "margin-right" => Some(CssPropertyId::MarginRight),
            "position" => Some(CssPropertyId::Position),
            "bottom" => Some(CssPropertyId::Bottom),
            "z-index" => Some(CssPropertyId::ZIndex),
            "height" => Some(CssPropertyId::Height),
            "min-height" => Some(CssPropertyId::MinHeight),
            "max-height" => Some(CssPropertyId::MaxHeight),
            "line-height" => Some(CssPropertyId::LineHeight),
            "vertical-align" => Some(CssPropertyId::VerticalAlign),
            "content" => Some(CssPropertyId::Content),
            "quotes" => Some(CssPropertyId::Quotes),
            "counter-reset" => Some(CssPropertyId::CounterReset),
            "counter-increment" => Some(CssPropertyId::CounterIncrement),
            "list-style-type" => Some(CssPropertyId::ListStyleType),
            "list-style-image" => Some(CssPropertyId::ListStyleImage),
            "list-style-position" => Some(CssPropertyId::ListStylePosition),
            "list-style" => Some(CssPropertyId::ListStyle),
            "page-break-before" => Some(CssPropertyId::PageBreakBefore),
            "page-break-after" => Some(CssPropertyId::PageBreakAfter),
            "page-break-inside" => Some(CssPropertyId::PageBreakInside),
            "orphans" => Some(CssPropertyId::Orphans),
            "widows" => Some(CssPropertyId::Widows),
            "text-decoration" => Some(CssPropertyId::TextDecoration),
            "caption-side" => Some(CssPropertyId::CaptionSide),
            "table-layout" => Some(CssPropertyId::TableLayout),
            "border-collapse" => Some(CssPropertyId::BorderCollapse),
            "border-spacing" => Some(CssPropertyId::BorderSpacing),
            "empty-cells" => Some(CssPropertyId::EmptyCells),
            _ => None,
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            CssPropertyId::AnimationName => "animation-name",
            CssPropertyId::AnimationDuration => "animation-duration",
            CssPropertyId::AnimationTimingFunction => "animation-timing-function",
            CssPropertyId::AnimationIterationCount => "animation-iteration-count",
            CssPropertyId::AnimationDirection => "animation-direction",
            CssPropertyId::AnimationPlayState => "animation-play-state",
            CssPropertyId::AnimationDelay => "animation-delay",
            CssPropertyId::AnimationFillMode => "animation-fill-mode",
            CssPropertyId::Animation => "animation",
            CssPropertyId::BackgroundColor => "background-color",
            CssPropertyId::BackgroundImage => "background-image",
            CssPropertyId::BackgroundRepeat => "background-repeat",
            CssPropertyId::BackgroundAttachment => "background-attachment",
            CssPropertyId::BackgroundPosition => "background-position",
            CssPropertyId::BackgroundClip => "background-clip",
            CssPropertyId::BackgroundOrigin => "background-origin",
            CssPropertyId::BackgroundSize => "background-size",
            CssPropertyId::Background => "background",
            CssPropertyId::BorderTopColor => "border-top-color",
            CssPropertyId::BorderColor => "border-color",
            CssPropertyId::BorderTopStyle => "border-top-style",
            CssPropertyId::BorderStyle => "border-style",
            CssPropertyId::BorderTopWidth => "border-top-width",
            CssPropertyId::BorderWidth => "border-width",
            CssPropertyId::BorderTop => "border-top",
            CssPropertyId::Border => "border",
            CssPropertyId::BorderTopLeftRadius => "border-top-left-radius",
            CssPropertyId::BorderRadius => "border-radius",
            CssPropertyId::BorderImageSource => "border-image-source",
            CssPropertyId::BorderImageSlice => "border-image-slice",
            CssPropertyId::BorderImageWidth => "border-image-width",
            CssPropertyId::BorderImageOutset => "border-image-outset",
            CssPropertyId::BorderImageRepeat => "border-image-repeat",
            CssPropertyId::BorderImage => "border-image",
            CssPropertyId::BoxShadow => "box-shadow",
            CssPropertyId::MarginTop => "margin-top",
            CssPropertyId::Margin => "margin",
            CssPropertyId::MarginTrim => "margin-trim",
            CssPropertyId::PaddingTop => "padding-top",
            CssPropertyId::Padding => "padding",
            CssPropertyId::Opacity => "opacity",
            CssPropertyId::Contain => "contain",
            CssPropertyId::ContentVisibility => "content-visibility",
            CssPropertyId::Display => "display",
            CssPropertyId::Visibility => "visibility",
            CssPropertyId::FlexDirection => "flex-direction",
            CssPropertyId::FlexWrap => "flex-wrap",
            CssPropertyId::FlexFlow => "flex-flow",
            CssPropertyId::FlexGrow => "flex-grow",
            CssPropertyId::FlexShrink => "flex-shrink",
            CssPropertyId::FlexBasis => "flex-basis",
            CssPropertyId::JustifyContent => "justify-content",
            CssPropertyId::AlignItems => "align-items",
            CssPropertyId::AlignSelf => "align-self",
            CssPropertyId::AlignContent => "align-content",
            CssPropertyId::FontFamily => "font-family",
            CssPropertyId::FontWeight => "font-weight",
            CssPropertyId::FontWidth => "font-width",
            CssPropertyId::FontStyle => "font-style",
            CssPropertyId::FontSize => "font-size",
            CssPropertyId::FontSizeAdjust => "font-size-adjust",
            CssPropertyId::FontSynthesisWeight => "font-synthesis-weight",
            CssPropertyId::FontSynthesisStyle => "font-synthesis-style",
            CssPropertyId::FontSynthesisSmallCaps => "font-synthesis-small-caps",
            CssPropertyId::FontSynthesisPosition => "font-synthesis-position",
            CssPropertyId::FontSynthesis => "font-synthesis",
            CssPropertyId::FontKerning => "font-kerning",
            CssPropertyId::FontVariantLigatures => "font-variant-ligatures",
            CssPropertyId::FontVariantPosition => "font-variant-position",
            CssPropertyId::FontVariantCaps => "font-variant-caps",
            CssPropertyId::FontVariantNumeric => "font-variant-numeric",
            CssPropertyId::FontVariantAlternates => "font-variant-alternates",
            CssPropertyId::FontVariantEastAsian => "font-variant-east-asian",
            CssPropertyId::FontVariant => "font-variant",
            CssPropertyId::FontFeatureSettings => "font-feature-settings",
            CssPropertyId::FontLanguageOverride => "font-language-override",
            CssPropertyId::FontOpticalSizing => "font-optical-sizing",
            CssPropertyId::FontVariationSettings => "font-variation-settings",
            CssPropertyId::FontPalette => "font-palette",
            CssPropertyId::FontVariantEmoji => "font-variant-emoji",
            CssPropertyId::GridTemplateColumns => "grid-template-columns",
            CssPropertyId::GridTemplateAreas => "grid-template-areas",
            CssPropertyId::GridTemplate => "grid-template",
            CssPropertyId::GridAutoColumns => "grid-auto-columns",
            CssPropertyId::GridAutoFlow => "grid-auto-flow",
            CssPropertyId::GridRowStart => "grid-row-start",
            CssPropertyId::GridRow => "grid-row",
            CssPropertyId::GridArea => "grid-area",
            CssPropertyId::ObjectFit => "object-fit",
            CssPropertyId::ObjectPosition => "object-position",
            CssPropertyId::ImageOrientation => "image-orientation",
            CssPropertyId::ImageRendering => "image-rendering",
            CssPropertyId::OverflowX => "overflow-x",
            CssPropertyId::Overflow => "overflow",
            CssPropertyId::OverflowClipMargin => "overflow-clip-margin",
            CssPropertyId::ScrollBehavior => "scroll-behavior",
            CssPropertyId::ScrollbarGutter => "scrollbar-gutter",
            CssPropertyId::TextOverflow => "text-overflow",
            CssPropertyId::MinWidth => "min-width",
            CssPropertyId::MaxWidth => "max-width",
            CssPropertyId::BoxSizing => "box-sizing",
            CssPropertyId::TextTransform => "text-transform",
            CssPropertyId::WhiteSpace => "white-space",
            CssPropertyId::TabSize => "tab-size",
            CssPropertyId::WordBreak => "word-break",
            CssPropertyId::LineBreak => "line-break",
            CssPropertyId::Hyphens => "hyphens",
            CssPropertyId::OverflowWrap => "overflow-wrap",
            CssPropertyId::TextAlign => "text-align",
            CssPropertyId::TextAlignAll => "text-align-all",
            CssPropertyId::TextAlignLast => "text-align-last",
            CssPropertyId::TextJustify => "text-justify",
            CssPropertyId::WordSpacing => "word-spacing",
            CssPropertyId::LetterSpacing => "letter-spacing",
            CssPropertyId::TextIndent => "text-indent",
            CssPropertyId::HangingPunctuation => "hanging-punctuation",
            CssPropertyId::Transform => "transform",
            CssPropertyId::TransformOrigin => "transform-origin",
            CssPropertyId::TransformBox => "transform-box",
            CssPropertyId::TransitionProperty => "transition-property",
            CssPropertyId::TransitionDuration => "transition-duration",
            CssPropertyId::TransitionDelay => "transition-delay",
            CssPropertyId::Transition => "transition",
            CssPropertyId::Outline => "outline",
            CssPropertyId::OutlineWidth => "outline-width",
            CssPropertyId::OutlineStyle => "outline-style",
            CssPropertyId::OutlineColor => "outline-color",
            CssPropertyId::OutlineOffset => "outline-offset",
            CssPropertyId::Resize => "resize",
            CssPropertyId::Cursor => "cursor",
            CssPropertyId::CaretColor => "caret-color",
            CssPropertyId::CaretAnimation => "caret-animation",
            CssPropertyId::CaretShape => "caret-shape",
            CssPropertyId::NavUp => "nav-up",
            CssPropertyId::UserSelect => "user-select",
            CssPropertyId::PointerEvents => "pointer-events",
            CssPropertyId::Interactivity => "interactivity",
            CssPropertyId::InterestDelayStart => "interest-delay-start",
            CssPropertyId::InterestDelay => "interest-delay",
            CssPropertyId::AccentColor => "accent-color",
            CssPropertyId::Appearance => "appearance",
            CssPropertyId::Direction => "direction",
            CssPropertyId::UnicodeBidi => "unicode-bidi",
            CssPropertyId::WritingMode => "writing-mode",
            CssPropertyId::TextOrientation => "text-orientation",
            CssPropertyId::TextCombineUpright => "text-combine-upright",
            CssPropertyId::PropertyName => "property-name",
            CssPropertyId::MarginRight => "margin-right",
            CssPropertyId::Position => "position",
            CssPropertyId::Bottom => "bottom",
            CssPropertyId::ZIndex => "z-index",
            CssPropertyId::Height => "height",
            CssPropertyId::MinHeight => "min-height",
            CssPropertyId::MaxHeight => "max-height",
            CssPropertyId::LineHeight => "line-height",
            CssPropertyId::VerticalAlign => "vertical-align",
            CssPropertyId::Content => "content",
            CssPropertyId::Quotes => "quotes",
            CssPropertyId::CounterReset => "counter-reset",
            CssPropertyId::CounterIncrement => "counter-increment",
            CssPropertyId::ListStyleType => "list-style-type",
            CssPropertyId::ListStyleImage => "list-style-image",
            CssPropertyId::ListStylePosition => "list-style-position",
            CssPropertyId::ListStyle => "list-style",
            CssPropertyId::PageBreakBefore => "page-break-before",
            CssPropertyId::PageBreakAfter => "page-break-after",
            CssPropertyId::PageBreakInside => "page-break-inside",
            CssPropertyId::Orphans => "orphans",
            CssPropertyId::Widows => "widows",
            CssPropertyId::TextDecoration => "text-decoration",
            CssPropertyId::CaptionSide => "caption-side",
            CssPropertyId::TableLayout => "table-layout",
            CssPropertyId::BorderCollapse => "border-collapse",
            CssPropertyId::BorderSpacing => "border-spacing",
            CssPropertyId::EmptyCells => "empty-cells",
            CssPropertyId::Unspecified => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PropertyDef {
    pub id: CssPropertyId,
    pub name: &'static str,
    pub value_syntax: &'static str,
    pub initial_value: &'static str,
    pub inherited: bool,
    pub animates: bool,
}

pub struct PropertyRegistry;

impl PropertyRegistry {
    pub fn by_name(name: &str) -> Option<&'static PropertyDef> {
        CssPropertyId::from_name(name).and_then(Self::by_id)
    }
    pub fn by_id(id: CssPropertyId) -> Option<&'static PropertyDef> {
        match id {
            CssPropertyId::AnimationName => Some(&P_ANIMATION_NAME),
            CssPropertyId::AnimationDuration => Some(&P_ANIMATION_DURATION),
            CssPropertyId::AnimationTimingFunction => Some(&P_ANIMATION_TIMING_FUNCTION),
            CssPropertyId::AnimationIterationCount => Some(&P_ANIMATION_ITERATION_COUNT),
            CssPropertyId::AnimationDirection => Some(&P_ANIMATION_DIRECTION),
            CssPropertyId::AnimationPlayState => Some(&P_ANIMATION_PLAY_STATE),
            CssPropertyId::AnimationDelay => Some(&P_ANIMATION_DELAY),
            CssPropertyId::AnimationFillMode => Some(&P_ANIMATION_FILL_MODE),
            CssPropertyId::Animation => Some(&P_ANIMATION),
            CssPropertyId::BackgroundColor => Some(&P_BACKGROUND_COLOR),
            CssPropertyId::BackgroundImage => Some(&P_BACKGROUND_IMAGE),
            CssPropertyId::BackgroundRepeat => Some(&P_BACKGROUND_REPEAT),
            CssPropertyId::BackgroundAttachment => Some(&P_BACKGROUND_ATTACHMENT),
            CssPropertyId::BackgroundPosition => Some(&P_BACKGROUND_POSITION),
            CssPropertyId::BackgroundClip => Some(&P_BACKGROUND_CLIP),
            CssPropertyId::BackgroundOrigin => Some(&P_BACKGROUND_ORIGIN),
            CssPropertyId::BackgroundSize => Some(&P_BACKGROUND_SIZE),
            CssPropertyId::Background => Some(&P_BACKGROUND),
            CssPropertyId::BorderTopColor => Some(&P_BORDER_TOP_COLOR),
            CssPropertyId::BorderColor => Some(&P_BORDER_COLOR),
            CssPropertyId::BorderTopStyle => Some(&P_BORDER_TOP_STYLE),
            CssPropertyId::BorderStyle => Some(&P_BORDER_STYLE),
            CssPropertyId::BorderTopWidth => Some(&P_BORDER_TOP_WIDTH),
            CssPropertyId::BorderWidth => Some(&P_BORDER_WIDTH),
            CssPropertyId::BorderTop => Some(&P_BORDER_TOP),
            CssPropertyId::Border => Some(&P_BORDER),
            CssPropertyId::BorderTopLeftRadius => Some(&P_BORDER_TOP_LEFT_RADIUS),
            CssPropertyId::BorderRadius => Some(&P_BORDER_RADIUS),
            CssPropertyId::BorderImageSource => Some(&P_BORDER_IMAGE_SOURCE),
            CssPropertyId::BorderImageSlice => Some(&P_BORDER_IMAGE_SLICE),
            CssPropertyId::BorderImageWidth => Some(&P_BORDER_IMAGE_WIDTH),
            CssPropertyId::BorderImageOutset => Some(&P_BORDER_IMAGE_OUTSET),
            CssPropertyId::BorderImageRepeat => Some(&P_BORDER_IMAGE_REPEAT),
            CssPropertyId::BorderImage => Some(&P_BORDER_IMAGE),
            CssPropertyId::BoxShadow => Some(&P_BOX_SHADOW),
            CssPropertyId::MarginTop => Some(&P_MARGIN_TOP),
            CssPropertyId::Margin => Some(&P_MARGIN),
            CssPropertyId::MarginTrim => Some(&P_MARGIN_TRIM),
            CssPropertyId::PaddingTop => Some(&P_PADDING_TOP),
            CssPropertyId::Padding => Some(&P_PADDING),
            CssPropertyId::Opacity => Some(&P_OPACITY),
            CssPropertyId::Contain => Some(&P_CONTAIN),
            CssPropertyId::ContentVisibility => Some(&P_CONTENT_VISIBILITY),
            CssPropertyId::Display => Some(&P_DISPLAY),
            CssPropertyId::Visibility => Some(&P_VISIBILITY),
            CssPropertyId::FlexDirection => Some(&P_FLEX_DIRECTION),
            CssPropertyId::FlexWrap => Some(&P_FLEX_WRAP),
            CssPropertyId::FlexFlow => Some(&P_FLEX_FLOW),
            CssPropertyId::FlexGrow => Some(&P_FLEX_GROW),
            CssPropertyId::FlexShrink => Some(&P_FLEX_SHRINK),
            CssPropertyId::FlexBasis => Some(&P_FLEX_BASIS),
            CssPropertyId::JustifyContent => Some(&P_JUSTIFY_CONTENT),
            CssPropertyId::AlignItems => Some(&P_ALIGN_ITEMS),
            CssPropertyId::AlignSelf => Some(&P_ALIGN_SELF),
            CssPropertyId::AlignContent => Some(&P_ALIGN_CONTENT),
            CssPropertyId::FontFamily => Some(&P_FONT_FAMILY),
            CssPropertyId::FontWeight => Some(&P_FONT_WEIGHT),
            CssPropertyId::FontWidth => Some(&P_FONT_WIDTH),
            CssPropertyId::FontStyle => Some(&P_FONT_STYLE),
            CssPropertyId::FontSize => Some(&P_FONT_SIZE),
            CssPropertyId::FontSizeAdjust => Some(&P_FONT_SIZE_ADJUST),
            CssPropertyId::FontSynthesisWeight => Some(&P_FONT_SYNTHESIS_WEIGHT),
            CssPropertyId::FontSynthesisStyle => Some(&P_FONT_SYNTHESIS_STYLE),
            CssPropertyId::FontSynthesisSmallCaps => Some(&P_FONT_SYNTHESIS_SMALL_CAPS),
            CssPropertyId::FontSynthesisPosition => Some(&P_FONT_SYNTHESIS_POSITION),
            CssPropertyId::FontSynthesis => Some(&P_FONT_SYNTHESIS),
            CssPropertyId::FontKerning => Some(&P_FONT_KERNING),
            CssPropertyId::FontVariantLigatures => Some(&P_FONT_VARIANT_LIGATURES),
            CssPropertyId::FontVariantPosition => Some(&P_FONT_VARIANT_POSITION),
            CssPropertyId::FontVariantCaps => Some(&P_FONT_VARIANT_CAPS),
            CssPropertyId::FontVariantNumeric => Some(&P_FONT_VARIANT_NUMERIC),
            CssPropertyId::FontVariantAlternates => Some(&P_FONT_VARIANT_ALTERNATES),
            CssPropertyId::FontVariantEastAsian => Some(&P_FONT_VARIANT_EAST_ASIAN),
            CssPropertyId::FontVariant => Some(&P_FONT_VARIANT),
            CssPropertyId::FontFeatureSettings => Some(&P_FONT_FEATURE_SETTINGS),
            CssPropertyId::FontLanguageOverride => Some(&P_FONT_LANGUAGE_OVERRIDE),
            CssPropertyId::FontOpticalSizing => Some(&P_FONT_OPTICAL_SIZING),
            CssPropertyId::FontVariationSettings => Some(&P_FONT_VARIATION_SETTINGS),
            CssPropertyId::FontPalette => Some(&P_FONT_PALETTE),
            CssPropertyId::FontVariantEmoji => Some(&P_FONT_VARIANT_EMOJI),
            CssPropertyId::GridTemplateColumns => Some(&P_GRID_TEMPLATE_COLUMNS),
            CssPropertyId::GridTemplateAreas => Some(&P_GRID_TEMPLATE_AREAS),
            CssPropertyId::GridTemplate => Some(&P_GRID_TEMPLATE),
            CssPropertyId::GridAutoColumns => Some(&P_GRID_AUTO_COLUMNS),
            CssPropertyId::GridAutoFlow => Some(&P_GRID_AUTO_FLOW),
            CssPropertyId::GridRowStart => Some(&P_GRID_ROW_START),
            CssPropertyId::GridRow => Some(&P_GRID_ROW),
            CssPropertyId::GridArea => Some(&P_GRID_AREA),
            CssPropertyId::ObjectFit => Some(&P_OBJECT_FIT),
            CssPropertyId::ObjectPosition => Some(&P_OBJECT_POSITION),
            CssPropertyId::ImageOrientation => Some(&P_IMAGE_ORIENTATION),
            CssPropertyId::ImageRendering => Some(&P_IMAGE_RENDERING),
            CssPropertyId::OverflowX => Some(&P_OVERFLOW_X),
            CssPropertyId::Overflow => Some(&P_OVERFLOW),
            CssPropertyId::OverflowClipMargin => Some(&P_OVERFLOW_CLIP_MARGIN),
            CssPropertyId::ScrollBehavior => Some(&P_SCROLL_BEHAVIOR),
            CssPropertyId::ScrollbarGutter => Some(&P_SCROLLBAR_GUTTER),
            CssPropertyId::TextOverflow => Some(&P_TEXT_OVERFLOW),
            CssPropertyId::MinWidth => Some(&P_MIN_WIDTH),
            CssPropertyId::MaxWidth => Some(&P_MAX_WIDTH),
            CssPropertyId::BoxSizing => Some(&P_BOX_SIZING),
            CssPropertyId::TextTransform => Some(&P_TEXT_TRANSFORM),
            CssPropertyId::WhiteSpace => Some(&P_WHITE_SPACE),
            CssPropertyId::TabSize => Some(&P_TAB_SIZE),
            CssPropertyId::WordBreak => Some(&P_WORD_BREAK),
            CssPropertyId::LineBreak => Some(&P_LINE_BREAK),
            CssPropertyId::Hyphens => Some(&P_HYPHENS),
            CssPropertyId::OverflowWrap => Some(&P_OVERFLOW_WRAP),
            CssPropertyId::TextAlign => Some(&P_TEXT_ALIGN),
            CssPropertyId::TextAlignAll => Some(&P_TEXT_ALIGN_ALL),
            CssPropertyId::TextAlignLast => Some(&P_TEXT_ALIGN_LAST),
            CssPropertyId::TextJustify => Some(&P_TEXT_JUSTIFY),
            CssPropertyId::WordSpacing => Some(&P_WORD_SPACING),
            CssPropertyId::LetterSpacing => Some(&P_LETTER_SPACING),
            CssPropertyId::TextIndent => Some(&P_TEXT_INDENT),
            CssPropertyId::HangingPunctuation => Some(&P_HANGING_PUNCTUATION),
            CssPropertyId::Transform => Some(&P_TRANSFORM),
            CssPropertyId::TransformOrigin => Some(&P_TRANSFORM_ORIGIN),
            CssPropertyId::TransformBox => Some(&P_TRANSFORM_BOX),
            CssPropertyId::TransitionProperty => Some(&P_TRANSITION_PROPERTY),
            CssPropertyId::TransitionDuration => Some(&P_TRANSITION_DURATION),
            CssPropertyId::TransitionDelay => Some(&P_TRANSITION_DELAY),
            CssPropertyId::Transition => Some(&P_TRANSITION),
            CssPropertyId::Outline => Some(&P_OUTLINE),
            CssPropertyId::OutlineWidth => Some(&P_OUTLINE_WIDTH),
            CssPropertyId::OutlineStyle => Some(&P_OUTLINE_STYLE),
            CssPropertyId::OutlineColor => Some(&P_OUTLINE_COLOR),
            CssPropertyId::OutlineOffset => Some(&P_OUTLINE_OFFSET),
            CssPropertyId::Resize => Some(&P_RESIZE),
            CssPropertyId::Cursor => Some(&P_CURSOR),
            CssPropertyId::CaretColor => Some(&P_CARET_COLOR),
            CssPropertyId::CaretAnimation => Some(&P_CARET_ANIMATION),
            CssPropertyId::CaretShape => Some(&P_CARET_SHAPE),
            CssPropertyId::NavUp => Some(&P_NAV_UP),
            CssPropertyId::UserSelect => Some(&P_USER_SELECT),
            CssPropertyId::PointerEvents => Some(&P_POINTER_EVENTS),
            CssPropertyId::Interactivity => Some(&P_INTERACTIVITY),
            CssPropertyId::InterestDelayStart => Some(&P_INTEREST_DELAY_START),
            CssPropertyId::InterestDelay => Some(&P_INTEREST_DELAY),
            CssPropertyId::AccentColor => Some(&P_ACCENT_COLOR),
            CssPropertyId::Appearance => Some(&P_APPEARANCE),
            CssPropertyId::Direction => Some(&P_DIRECTION),
            CssPropertyId::UnicodeBidi => Some(&P_UNICODE_BIDI),
            CssPropertyId::WritingMode => Some(&P_WRITING_MODE),
            CssPropertyId::TextOrientation => Some(&P_TEXT_ORIENTATION),
            CssPropertyId::TextCombineUpright => Some(&P_TEXT_COMBINE_UPRIGHT),
            CssPropertyId::PropertyName => Some(&P_PROPERTY_NAME),
            CssPropertyId::MarginRight => Some(&P_MARGIN_RIGHT),
            CssPropertyId::Position => Some(&P_POSITION),
            CssPropertyId::Bottom => Some(&P_BOTTOM),
            CssPropertyId::ZIndex => Some(&P_Z_INDEX),
            CssPropertyId::Height => Some(&P_HEIGHT),
            CssPropertyId::MinHeight => Some(&P_MIN_HEIGHT),
            CssPropertyId::MaxHeight => Some(&P_MAX_HEIGHT),
            CssPropertyId::LineHeight => Some(&P_LINE_HEIGHT),
            CssPropertyId::VerticalAlign => Some(&P_VERTICAL_ALIGN),
            CssPropertyId::Content => Some(&P_CONTENT),
            CssPropertyId::Quotes => Some(&P_QUOTES),
            CssPropertyId::CounterReset => Some(&P_COUNTER_RESET),
            CssPropertyId::CounterIncrement => Some(&P_COUNTER_INCREMENT),
            CssPropertyId::ListStyleType => Some(&P_LIST_STYLE_TYPE),
            CssPropertyId::ListStyleImage => Some(&P_LIST_STYLE_IMAGE),
            CssPropertyId::ListStylePosition => Some(&P_LIST_STYLE_POSITION),
            CssPropertyId::ListStyle => Some(&P_LIST_STYLE),
            CssPropertyId::PageBreakBefore => Some(&P_PAGE_BREAK_BEFORE),
            CssPropertyId::PageBreakAfter => Some(&P_PAGE_BREAK_AFTER),
            CssPropertyId::PageBreakInside => Some(&P_PAGE_BREAK_INSIDE),
            CssPropertyId::Orphans => Some(&P_ORPHANS),
            CssPropertyId::Widows => Some(&P_WIDOWS),
            CssPropertyId::TextDecoration => Some(&P_TEXT_DECORATION),
            CssPropertyId::CaptionSide => Some(&P_CAPTION_SIDE),
            CssPropertyId::TableLayout => Some(&P_TABLE_LAYOUT),
            CssPropertyId::BorderCollapse => Some(&P_BORDER_COLLAPSE),
            CssPropertyId::BorderSpacing => Some(&P_BORDER_SPACING),
            CssPropertyId::EmptyCells => Some(&P_EMPTY_CELLS),
            CssPropertyId::Unspecified => None,
        }
    }
}

pub const P_ANIMATION_NAME: PropertyDef = PropertyDef {
    id: CssPropertyId::AnimationName,
    name: "animation-name",
    value_syntax: "\\[ none | <keyframes-name> \\]#",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_ANIMATION_DURATION: PropertyDef = PropertyDef {
    id: CssPropertyId::AnimationDuration,
    name: "animation-duration",
    value_syntax: "<time [0s,∞]>#",
    initial_value: "0s",
    inherited: false,
    animates: false,
};

pub const P_ANIMATION_TIMING_FUNCTION: PropertyDef = PropertyDef {
    id: CssPropertyId::AnimationTimingFunction,
    name: "animation-timing-function",
    value_syntax: "<easing-function>#",
    initial_value: "ease",
    inherited: false,
    animates: false,
};

pub const P_ANIMATION_ITERATION_COUNT: PropertyDef = PropertyDef {
    id: CssPropertyId::AnimationIterationCount,
    name: "animation-iteration-count",
    value_syntax: "<single-animation-iteration-count>#",
    initial_value: "1",
    inherited: false,
    animates: false,
};

pub const P_ANIMATION_DIRECTION: PropertyDef = PropertyDef {
    id: CssPropertyId::AnimationDirection,
    name: "animation-direction",
    value_syntax: "<single-animation-direction>#",
    initial_value: "normal",
    inherited: false,
    animates: false,
};

pub const P_ANIMATION_PLAY_STATE: PropertyDef = PropertyDef {
    id: CssPropertyId::AnimationPlayState,
    name: "animation-play-state",
    value_syntax: "<single-animation-play-state>#",
    initial_value: "running",
    inherited: false,
    animates: false,
};

pub const P_ANIMATION_DELAY: PropertyDef = PropertyDef {
    id: CssPropertyId::AnimationDelay,
    name: "animation-delay",
    value_syntax: "<time>#",
    initial_value: "0s",
    inherited: false,
    animates: false,
};

pub const P_ANIMATION_FILL_MODE: PropertyDef = PropertyDef {
    id: CssPropertyId::AnimationFillMode,
    name: "animation-fill-mode",
    value_syntax: "<single-animation-fill-mode>#",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_ANIMATION: PropertyDef = PropertyDef {
    id: CssPropertyId::Animation,
    name: "animation",
    value_syntax: "<single-animation>#",
    initial_value: "see individual properties",
    inherited: false,
    animates: false,
};

pub const P_BACKGROUND_COLOR: PropertyDef = PropertyDef {
    id: CssPropertyId::BackgroundColor,
    name: "background-color",
    value_syntax: "<color>",
    initial_value: "transparent",
    inherited: false,
    animates: false,
};

pub const P_BACKGROUND_IMAGE: PropertyDef = PropertyDef {
    id: CssPropertyId::BackgroundImage,
    name: "background-image",
    value_syntax: "<bg-image>#",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_BACKGROUND_REPEAT: PropertyDef = PropertyDef {
    id: CssPropertyId::BackgroundRepeat,
    name: "background-repeat",
    value_syntax: "<repeat-style>#",
    initial_value: "repeat",
    inherited: false,
    animates: false,
};

pub const P_BACKGROUND_ATTACHMENT: PropertyDef = PropertyDef {
    id: CssPropertyId::BackgroundAttachment,
    name: "background-attachment",
    value_syntax: "<attachment>#",
    initial_value: "scroll",
    inherited: false,
    animates: false,
};

pub const P_BACKGROUND_POSITION: PropertyDef = PropertyDef {
    id: CssPropertyId::BackgroundPosition,
    name: "background-position",
    value_syntax: "<bg-position>#",
    initial_value: "0% 0%",
    inherited: false,
    animates: false,
};

pub const P_BACKGROUND_CLIP: PropertyDef = PropertyDef {
    id: CssPropertyId::BackgroundClip,
    name: "background-clip",
    value_syntax: "<visual-box>#",
    initial_value: "border-box",
    inherited: false,
    animates: false,
};

pub const P_BACKGROUND_ORIGIN: PropertyDef = PropertyDef {
    id: CssPropertyId::BackgroundOrigin,
    name: "background-origin",
    value_syntax: "<visual-box>#",
    initial_value: "padding-box",
    inherited: false,
    animates: false,
};

pub const P_BACKGROUND_SIZE: PropertyDef = PropertyDef {
    id: CssPropertyId::BackgroundSize,
    name: "background-size",
    value_syntax: "<bg-size>#",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_BACKGROUND: PropertyDef = PropertyDef {
    id: CssPropertyId::Background,
    name: "background",
    value_syntax: "<bg-layer>#? , <final-bg-layer>",
    initial_value: "see individual properties",
    inherited: false,
    animates: false,
};

pub const P_BORDER_TOP_COLOR: PropertyDef = PropertyDef {
    id: CssPropertyId::BorderTopColor,
    name: "border-top-color",
    value_syntax: "<color>",
    initial_value: "[currentColor](https://www.w3.org/TR/css-color-4/#currentcolor-color)",
    inherited: false,
    animates: false,
};

pub const P_BORDER_COLOR: PropertyDef = PropertyDef {
    id: CssPropertyId::BorderColor,
    name: "border-color",
    value_syntax: "<color>{1,4}",
    initial_value: "see individual properties",
    inherited: false,
    animates: false,
};

pub const P_BORDER_TOP_STYLE: PropertyDef = PropertyDef {
    id: CssPropertyId::BorderTopStyle,
    name: "border-top-style",
    value_syntax: "<line-style>",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_BORDER_STYLE: PropertyDef = PropertyDef {
    id: CssPropertyId::BorderStyle,
    name: "border-style",
    value_syntax: "<line-style>{1,4}",
    initial_value: "see individual properties",
    inherited: false,
    animates: false,
};

pub const P_BORDER_TOP_WIDTH: PropertyDef = PropertyDef {
    id: CssPropertyId::BorderTopWidth,
    name: "border-top-width",
    value_syntax: "<line-width>",
    initial_value: "medium",
    inherited: false,
    animates: false,
};

pub const P_BORDER_WIDTH: PropertyDef = PropertyDef {
    id: CssPropertyId::BorderWidth,
    name: "border-width",
    value_syntax: "<line-width>{1,4}",
    initial_value: "see individual properties",
    inherited: false,
    animates: false,
};

pub const P_BORDER_TOP: PropertyDef = PropertyDef {
    id: CssPropertyId::BorderTop,
    name: "border-top",
    value_syntax: "<line-width> || <line-style> \\|\\| <color>",
    initial_value: "See individual properties",
    inherited: false,
    animates: false,
};

pub const P_BORDER: PropertyDef = PropertyDef {
    id: CssPropertyId::Border,
    name: "border",
    value_syntax: "<line-width> || <line-style> \\|\\| <color>",
    initial_value: "See individual properties",
    inherited: false,
    animates: false,
};

pub const P_BORDER_TOP_LEFT_RADIUS: PropertyDef = PropertyDef {
    id: CssPropertyId::BorderTopLeftRadius,
    name: "border-top-left-radius",
    value_syntax: "<length-percentage [0,∞]>{1,2}",
    initial_value: "0",
    inherited: false,
    animates: false,
};

pub const P_BORDER_RADIUS: PropertyDef = PropertyDef {
    id: CssPropertyId::BorderRadius,
    name: "border-radius",
    value_syntax: "<length-percentage [0,∞]>{1,4} \\[ / \\{1,4} \\]?",
    initial_value: "see individual properties",
    inherited: false,
    animates: false,
};

pub const P_BORDER_IMAGE_SOURCE: PropertyDef = PropertyDef {
    id: CssPropertyId::BorderImageSource,
    name: "border-image-source",
    value_syntax: "none | <image>",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_BORDER_IMAGE_SLICE: PropertyDef = PropertyDef {
    id: CssPropertyId::BorderImageSlice,
    name: "border-image-slice",
    value_syntax: "\\[<number [0,∞]> | <percentage [0,∞]>\\]{1,4} && fill?",
    initial_value: "100%",
    inherited: false,
    animates: false,
};

pub const P_BORDER_IMAGE_WIDTH: PropertyDef = PropertyDef {
    id: CssPropertyId::BorderImageWidth,
    name: "border-image-width",
    value_syntax: "\\[ <length-percentage [0,∞]> | <number [0,∞]> \\| auto \\]{1,4}",
    initial_value: "1",
    inherited: false,
    animates: false,
};

pub const P_BORDER_IMAGE_OUTSET: PropertyDef = PropertyDef {
    id: CssPropertyId::BorderImageOutset,
    name: "border-image-outset",
    value_syntax: "\\[ <length [0,∞]> | <number [0,∞]> \\]{1,4}",
    initial_value: "0",
    inherited: false,
    animates: false,
};

pub const P_BORDER_IMAGE_REPEAT: PropertyDef = PropertyDef {
    id: CssPropertyId::BorderImageRepeat,
    name: "border-image-repeat",
    value_syntax: "\\[ stretch | repeat \\| round \\| space \\]{1,2}",
    initial_value: "stretch",
    inherited: false,
    animates: false,
};

pub const P_BORDER_IMAGE: PropertyDef = PropertyDef {
    id: CssPropertyId::BorderImage,
    name: "border-image",
    value_syntax: "<'border-image-source'> || <'border-image-slice'> \\[ / <'border-image-width'> | / \\? / <'border-image-outset'> \\]? \\|\\| <'border-image-repeat'>",
    initial_value: "See individual properties",
    inherited: false,
    animates: false,
};

pub const P_BOX_SHADOW: PropertyDef = PropertyDef {
    id: CssPropertyId::BoxShadow,
    name: "box-shadow",
    value_syntax: "none | <shadow>#",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_MARGIN_TOP: PropertyDef = PropertyDef {
    id: CssPropertyId::MarginTop,
    name: "margin-top",
    value_syntax: "<length-percentage> | auto",
    initial_value: "0",
    inherited: false,
    animates: false,
};

pub const P_MARGIN: PropertyDef = PropertyDef {
    id: CssPropertyId::Margin,
    name: "margin",
    value_syntax: "<'margin-top'>{1,4}",
    initial_value: "0",
    inherited: false,
    animates: false,
};

pub const P_MARGIN_TRIM: PropertyDef = PropertyDef {
    id: CssPropertyId::MarginTrim,
    name: "margin-trim",
    value_syntax: "none | \\[ block || inline \\] \\| \\[ block-start \\|\\| inline-start \\|\\| block-end \\|\\| inline-end \\]",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_PADDING_TOP: PropertyDef = PropertyDef {
    id: CssPropertyId::PaddingTop,
    name: "padding-top",
    value_syntax: "<length-percentage [0,∞]>",
    initial_value: "0",
    inherited: false,
    animates: false,
};

pub const P_PADDING: PropertyDef = PropertyDef {
    id: CssPropertyId::Padding,
    name: "padding",
    value_syntax: "<'padding-top'>{1,4}",
    initial_value: "0",
    inherited: false,
    animates: false,
};

pub const P_OPACITY: PropertyDef = PropertyDef {
    id: CssPropertyId::Opacity,
    name: "opacity",
    value_syntax: "<color>",
    initial_value: "CanvasText",
    inherited: true,
    animates: false,
};

pub const P_CONTAIN: PropertyDef = PropertyDef {
    id: CssPropertyId::Contain,
    name: "contain",
    value_syntax: "none | strict \\| content \\| \\[ \\[size \\| inline-size\\] || layout \\|\\| style \\|\\| paint \\]",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_CONTENT_VISIBILITY: PropertyDef = PropertyDef {
    id: CssPropertyId::ContentVisibility,
    name: "content-visibility",
    value_syntax: "visible | auto \\| hidden",
    initial_value: "visible",
    inherited: false,
    animates: false,
};

pub const P_DISPLAY: PropertyDef = PropertyDef {
    id: CssPropertyId::Display,
    name: "display",
    value_syntax: "\\[ <display-outside> || <display-inside> \\] | <display-listitem> \\| <display-internal> \\| <display-box> \\| <display-legacy>",
    initial_value: "inline",
    inherited: false,
    animates: false,
};

pub const P_VISIBILITY: PropertyDef = PropertyDef {
    id: CssPropertyId::Visibility,
    name: "visibility",
    value_syntax: "<integer>",
    initial_value: "0",
    inherited: false,
    animates: false,
};

pub const P_FLEX_DIRECTION: PropertyDef = PropertyDef {
    id: CssPropertyId::FlexDirection,
    name: "flex-direction",
    value_syntax: "row | row-reverse \\| column \\| column-reverse",
    initial_value: "row",
    inherited: false,
    animates: false,
};

pub const P_FLEX_WRAP: PropertyDef = PropertyDef {
    id: CssPropertyId::FlexWrap,
    name: "flex-wrap",
    value_syntax: "nowrap | wrap \\| wrap-reverse",
    initial_value: "nowrap",
    inherited: false,
    animates: false,
};

pub const P_FLEX_FLOW: PropertyDef = PropertyDef {
    id: CssPropertyId::FlexFlow,
    name: "flex-flow",
    value_syntax: "<'flex-direction'> || <'flex-wrap'>",
    initial_value: "see individual properties",
    inherited: false,
    animates: false,
};

pub const P_FLEX_GROW: PropertyDef = PropertyDef {
    id: CssPropertyId::FlexGrow,
    name: "flex-grow",
    value_syntax: "none | \\[ <'flex-grow'> <'flex-shrink'>? || <'flex-basis'> \\]",
    initial_value: "0 1 auto",
    inherited: false,
    animates: false,
};

pub const P_FLEX_SHRINK: PropertyDef = PropertyDef {
    id: CssPropertyId::FlexShrink,
    name: "flex-shrink",
    value_syntax: "<number [0,∞]>",
    initial_value: "1",
    inherited: false,
    animates: false,
};

pub const P_FLEX_BASIS: PropertyDef = PropertyDef {
    id: CssPropertyId::FlexBasis,
    name: "flex-basis",
    value_syntax: "content | <'width'>",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_JUSTIFY_CONTENT: PropertyDef = PropertyDef {
    id: CssPropertyId::JustifyContent,
    name: "justify-content",
    value_syntax: "flex-start | flex-end \\| center \\| space-between \\| space-around",
    initial_value: "flex-start",
    inherited: false,
    animates: false,
};

pub const P_ALIGN_ITEMS: PropertyDef = PropertyDef {
    id: CssPropertyId::AlignItems,
    name: "align-items",
    value_syntax: "flex-start | flex-end \\| center \\| baseline \\| stretch",
    initial_value: "stretch",
    inherited: false,
    animates: false,
};

pub const P_ALIGN_SELF: PropertyDef = PropertyDef {
    id: CssPropertyId::AlignSelf,
    name: "align-self",
    value_syntax: "auto | flex-start \\| flex-end \\| center \\| baseline \\| stretch",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_ALIGN_CONTENT: PropertyDef = PropertyDef {
    id: CssPropertyId::AlignContent,
    name: "align-content",
    value_syntax: "flex-start | flex-end \\| center \\| space-between \\| space-around \\| stretch",
    initial_value: "stretch",
    inherited: false,
    animates: false,
};

pub const P_FONT_FAMILY: PropertyDef = PropertyDef {
    id: CssPropertyId::FontFamily,
    name: "font-family",
    value_syntax: "\\[ <family-name> | <generic-family> \\]#",
    initial_value: "depends on user agent",
    inherited: true,
    animates: false,
};

pub const P_FONT_WEIGHT: PropertyDef = PropertyDef {
    id: CssPropertyId::FontWeight,
    name: "font-weight",
    value_syntax: "<font-weight-absolute> | bolder \\| lighter",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_FONT_WIDTH: PropertyDef = PropertyDef {
    id: CssPropertyId::FontWidth,
    name: "font-width",
    value_syntax: "normal | <percentage [0,∞]> \\| ultra-condensed \\| extra-condensed \\| condensed \\| semi-condensed \\| semi-expanded \\| expanded \\| extra-expanded \\| ultra-expanded",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_FONT_STYLE: PropertyDef = PropertyDef {
    id: CssPropertyId::FontStyle,
    name: "font-style",
    value_syntax: "normal | italic \\| left \\| right \\| oblique <angle [-90deg,90deg]>?",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_FONT_SIZE: PropertyDef = PropertyDef {
    id: CssPropertyId::FontSize,
    name: "font-size",
    value_syntax: "<absolute-size> | <relative-size> \\| <length-percentage [0,∞]> \\| math",
    initial_value: "medium",
    inherited: true,
    animates: false,
};

pub const P_FONT_SIZE_ADJUST: PropertyDef = PropertyDef {
    id: CssPropertyId::FontSizeAdjust,
    name: "font-size-adjust",
    value_syntax: "none | <number [0,∞]>",
    initial_value: "none",
    inherited: true,
    animates: false,
};

pub const P_FONT_SYNTHESIS_WEIGHT: PropertyDef = PropertyDef {
    id: CssPropertyId::FontSynthesisWeight,
    name: "font-synthesis-weight",
    value_syntax: "\\[ \\[ <'font-style'> || <font-variant-css2> \\|\\| <'font-weight'> \\|\\| <font-width-css3> \\]? <'font-size'> \\[ / <'line-height'> \\]? <'font-family'># \\] | <system-family-name>",
    initial_value: "see individual properties",
    inherited: true,
    animates: false,
};

pub const P_FONT_SYNTHESIS_STYLE: PropertyDef = PropertyDef {
    id: CssPropertyId::FontSynthesisStyle,
    name: "font-synthesis-style",
    value_syntax: "auto | none \\| oblique-only",
    initial_value: "auto",
    inherited: true,
    animates: false,
};

pub const P_FONT_SYNTHESIS_SMALL_CAPS: PropertyDef = PropertyDef {
    id: CssPropertyId::FontSynthesisSmallCaps,
    name: "font-synthesis-small-caps",
    value_syntax: "auto | none",
    initial_value: "auto",
    inherited: true,
    animates: false,
};

pub const P_FONT_SYNTHESIS_POSITION: PropertyDef = PropertyDef {
    id: CssPropertyId::FontSynthesisPosition,
    name: "font-synthesis-position",
    value_syntax: "auto | none",
    initial_value: "auto",
    inherited: true,
    animates: false,
};

pub const P_FONT_SYNTHESIS: PropertyDef = PropertyDef {
    id: CssPropertyId::FontSynthesis,
    name: "font-synthesis",
    value_syntax: "none | \\[ weight || style \\|\\| small-caps \\|\\| position\\]",
    initial_value: "weight style small-caps position",
    inherited: true,
    animates: false,
};

pub const P_FONT_KERNING: PropertyDef = PropertyDef {
    id: CssPropertyId::FontKerning,
    name: "font-kerning",
    value_syntax: "<family-name>",
    initial_value: "N/A This descriptor defines the font family name that will be used in all CSS font family name matching. It overrides the font family names contained in the underlying font data. If the font family name is the same as a font family available in a given user’s environment, it effectively hides the underlying font for documents that use the stylesheet. This permits a web author to freely choose font-family names without worrying about conflicts with font family names present in a given user’s environment. Likewise, platform substitutions for a given font family name must not be used. ### 4.3. Font reference: the src descriptor",
    inherited: true,
    animates: false,
};

pub const P_FONT_VARIANT_LIGATURES: PropertyDef = PropertyDef {
    id: CssPropertyId::FontVariantLigatures,
    name: "font-variant-ligatures",
    value_syntax: "normal | none \\| \\[ `` || `` \\|\\| `` \\|\\| `` \\]",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_FONT_VARIANT_POSITION: PropertyDef = PropertyDef {
    id: CssPropertyId::FontVariantPosition,
    name: "font-variant-position",
    value_syntax: "normal | sub \\| super",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_FONT_VARIANT_CAPS: PropertyDef = PropertyDef {
    id: CssPropertyId::FontVariantCaps,
    name: "font-variant-caps",
    value_syntax: "normal | small-caps \\| all-small-caps \\| petite-caps \\| all-petite-caps \\| unicase \\| titling-caps",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_FONT_VARIANT_NUMERIC: PropertyDef = PropertyDef {
    id: CssPropertyId::FontVariantNumeric,
    name: "font-variant-numeric",
    value_syntax: "normal | \\[ <numeric-figure-values> || <numeric-spacing-values> \\|\\| <numeric-fraction-values> \\|\\| ordinal \\|\\| slashed-zero \\]",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_FONT_VARIANT_ALTERNATES: PropertyDef = PropertyDef {
    id: CssPropertyId::FontVariantAlternates,
    name: "font-variant-alternates",
    value_syntax: "normal | \\[ stylistic(<feature-value-name>) || historical-forms \\|\\| styleset(\\#) \\|\\| character-variant(\\\\#) \\|\\| swash(\\) \\|\\| ornaments(\\) \\|\\| annotation(\\) \\]",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_FONT_VARIANT_EAST_ASIAN: PropertyDef = PropertyDef {
    id: CssPropertyId::FontVariantEastAsian,
    name: "font-variant-east-asian",
    value_syntax: "normal | \\[ <east-asian-variant-values> || <east-asian-width-values> \\|\\| ruby \\]",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_FONT_VARIANT: PropertyDef = PropertyDef {
    id: CssPropertyId::FontVariant,
    name: "font-variant",
    value_syntax: "normal | none \\| \\[ \\[ <common-lig-values> || <discretionary-lig-values> \\|\\| <historical-lig-values> \\|\\| <contextual-alt-values> \\] \\|\\| \\[ small-caps \\| all-small-caps \\| petite-caps \\| all-petite-caps \\| unicase \\| titling-caps \\] \\|\\| \\[ stylistic(<feature-value-name>) \\|\\| historical-forms \\|\\| styleset(\\#) \\|\\| character-variant(\\\\#) \\|\\| swash(\\) \\|\\| ornaments(\\) \\|\\| annotation(\\) \\] \\|\\| \\[ <numeric-figure-values> \\|\\| <numeric-spacing-values> \\|\\| <numeric-fraction-values> \\|\\| ordinal \\|\\| slashed-zero \\] \\|\\| \\[ <east-asian-variant-values> \\|\\| <east-asian-width-values> \\|\\| ruby \\] \\|\\| \\[ sub \\| super \\] \\|\\| \\[ text \\| emoji \\| unicode \\] \\]",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_FONT_FEATURE_SETTINGS: PropertyDef = PropertyDef {
    id: CssPropertyId::FontFeatureSettings,
    name: "font-feature-settings",
    value_syntax: "normal | <feature-tag-value>#",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_FONT_LANGUAGE_OVERRIDE: PropertyDef = PropertyDef {
    id: CssPropertyId::FontLanguageOverride,
    name: "font-language-override",
    value_syntax: "normal | <string>",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_FONT_OPTICAL_SIZING: PropertyDef = PropertyDef {
    id: CssPropertyId::FontOpticalSizing,
    name: "font-optical-sizing",
    value_syntax: "auto | none",
    initial_value: "auto",
    inherited: true,
    animates: false,
};

pub const P_FONT_VARIATION_SETTINGS: PropertyDef = PropertyDef {
    id: CssPropertyId::FontVariationSettings,
    name: "font-variation-settings",
    value_syntax: "normal | \\[ <opentype-tag> <number> \\]#",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_FONT_PALETTE: PropertyDef = PropertyDef {
    id: CssPropertyId::FontPalette,
    name: "font-palette",
    value_syntax: "normal | light \\| dark \\| <palette-identifier> \\| <palette-mix()>",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_FONT_VARIANT_EMOJI: PropertyDef = PropertyDef {
    id: CssPropertyId::FontVariantEmoji,
    name: "font-variant-emoji",
    value_syntax: "<family-name>#",
    initial_value: "N/A This descriptor defines the font families that this palette applies to, using the same list of font families as [§ 5 Font Matching Algorithm](#font-matching-algorithm). This palette will only ever be applied to the fonts with these family names. The value of this descriptor means that only named font families are allowed and rules that include generic fonts in the list of font families are syntax errors. If syntax errors occur within the font family list, the descriptor must be ignored (will still be in the CSS OM, but will not match any font families). #### 9.2.2. Specifying the base palette: the base-palette descriptor",
    inherited: true,
    animates: false,
};

pub const P_GRID_TEMPLATE_COLUMNS: PropertyDef = PropertyDef {
    id: CssPropertyId::GridTemplateColumns,
    name: "grid-template-columns",
    value_syntax: "none | <track-list> \\| <auto-track-list> \\| subgrid <line-name-list>?",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_GRID_TEMPLATE_AREAS: PropertyDef = PropertyDef {
    id: CssPropertyId::GridTemplateAreas,
    name: "grid-template-areas",
    value_syntax: "none | <string>+",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_GRID_TEMPLATE: PropertyDef = PropertyDef {
    id: CssPropertyId::GridTemplate,
    name: "grid-template",
    value_syntax: "none | \\[ <'grid-template-rows'> / <'grid-template-columns'> \\] \\| \\[ <line-names>? <string> <track-size>? \\? \\]+ \\[ / <explicit-track-list> \\]?",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_GRID_AUTO_COLUMNS: PropertyDef = PropertyDef {
    id: CssPropertyId::GridAutoColumns,
    name: "grid-auto-columns",
    value_syntax: "<track-size>+",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_GRID_AUTO_FLOW: PropertyDef = PropertyDef {
    id: CssPropertyId::GridAutoFlow,
    name: "grid-auto-flow",
    value_syntax: "\\[ row | column \\] || dense",
    initial_value: "row",
    inherited: false,
    animates: false,
};

pub const P_GRID_ROW_START: PropertyDef = PropertyDef {
    id: CssPropertyId::GridRowStart,
    name: "grid-row-start",
    value_syntax: "<'grid-template'> | <'grid-template-rows'> / \\[ auto-flow && dense? \\] <'grid-auto-columns'>? \\| \\[ auto-flow && dense? \\] <'grid-auto-rows'>? / <'grid-template-columns'>",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_GRID_ROW: PropertyDef = PropertyDef {
    id: CssPropertyId::GridRow,
    name: "grid-row",
    value_syntax: "<grid-line> \\[ / \\ \\]?",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_GRID_AREA: PropertyDef = PropertyDef {
    id: CssPropertyId::GridArea,
    name: "grid-area",
    value_syntax: "<grid-line> \\[ / \\ \\]{0,3}",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_OBJECT_FIT: PropertyDef = PropertyDef {
    id: CssPropertyId::ObjectFit,
    name: "object-fit",
    value_syntax: "fill | contain \\| cover \\| none \\| scale-down",
    initial_value: "fill",
    inherited: false,
    animates: false,
};

pub const P_OBJECT_POSITION: PropertyDef = PropertyDef {
    id: CssPropertyId::ObjectPosition,
    name: "object-position",
    value_syntax: "<position>",
    initial_value: "50% 50%",
    inherited: false,
    animates: false,
};

pub const P_IMAGE_ORIENTATION: PropertyDef = PropertyDef {
    id: CssPropertyId::ImageOrientation,
    name: "image-orientation",
    value_syntax: "from-image | none \\| \\[ <angle> || flip \\]",
    initial_value: "from-image",
    inherited: true,
    animates: false,
};

pub const P_IMAGE_RENDERING: PropertyDef = PropertyDef {
    id: CssPropertyId::ImageRendering,
    name: "image-rendering",
    value_syntax: "auto | smooth \\| high-quality \\| pixelated \\| crisp-edges",
    initial_value: "auto",
    inherited: true,
    animates: false,
};

pub const P_OVERFLOW_X: PropertyDef = PropertyDef {
    id: CssPropertyId::OverflowX,
    name: "overflow-x",
    value_syntax: "visible | hidden \\| clip \\| scroll \\| auto",
    initial_value: "visible",
    inherited: false,
    animates: false,
};

pub const P_OVERFLOW: PropertyDef = PropertyDef {
    id: CssPropertyId::Overflow,
    name: "overflow",
    value_syntax: "<'overflow-block'>{1,2}",
    initial_value: "visible",
    inherited: false,
    animates: false,
};

pub const P_OVERFLOW_CLIP_MARGIN: PropertyDef = PropertyDef {
    id: CssPropertyId::OverflowClipMargin,
    name: "overflow-clip-margin",
    value_syntax: "<visual-box> || <length>",
    initial_value: "0px",
    inherited: false,
    animates: false,
};

pub const P_SCROLL_BEHAVIOR: PropertyDef = PropertyDef {
    id: CssPropertyId::ScrollBehavior,
    name: "scroll-behavior",
    value_syntax: "auto | smooth",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_SCROLLBAR_GUTTER: PropertyDef = PropertyDef {
    id: CssPropertyId::ScrollbarGutter,
    name: "scrollbar-gutter",
    value_syntax: "auto | stable && both-edges?",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_TEXT_OVERFLOW: PropertyDef = PropertyDef {
    id: CssPropertyId::TextOverflow,
    name: "text-overflow",
    value_syntax: "clip | ellipsis",
    initial_value: "clip",
    inherited: false,
    animates: false,
};

pub const P_MIN_WIDTH: PropertyDef = PropertyDef {
    id: CssPropertyId::MinWidth,
    name: "min-width",
    value_syntax: "auto | <length-percentage [0,∞]> \\| min-content \\| max-content \\| fit-content(<length-percentage [0,∞]>) \\| <calc-size()>",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_MAX_WIDTH: PropertyDef = PropertyDef {
    id: CssPropertyId::MaxWidth,
    name: "max-width",
    value_syntax: "none | <length-percentage [0,∞]> \\| min-content \\| max-content \\| fit-content(<length-percentage [0,∞]>) \\| <calc-size()>",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_BOX_SIZING: PropertyDef = PropertyDef {
    id: CssPropertyId::BoxSizing,
    name: "box-sizing",
    value_syntax: "content-box | border-box",
    initial_value: "content-box",
    inherited: false,
    animates: false,
};

pub const P_TEXT_TRANSFORM: PropertyDef = PropertyDef {
    id: CssPropertyId::TextTransform,
    name: "text-transform",
    value_syntax: "none | \\[capitalize \\| uppercase \\| lowercase \\] || full-width \\|\\| full-size-kana",
    initial_value: "none",
    inherited: true,
    animates: false,
};

pub const P_WHITE_SPACE: PropertyDef = PropertyDef {
    id: CssPropertyId::WhiteSpace,
    name: "white-space",
    value_syntax: "normal | pre \\| nowrap \\| pre-wrap \\| break-spaces \\| pre-line",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_TAB_SIZE: PropertyDef = PropertyDef {
    id: CssPropertyId::TabSize,
    name: "tab-size",
    value_syntax: "<number [0,∞]> | <length [0,∞]>",
    initial_value: "8",
    inherited: true,
    animates: false,
};

pub const P_WORD_BREAK: PropertyDef = PropertyDef {
    id: CssPropertyId::WordBreak,
    name: "word-break",
    value_syntax: "normal | keep-all \\| break-all \\| break-word",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_LINE_BREAK: PropertyDef = PropertyDef {
    id: CssPropertyId::LineBreak,
    name: "line-break",
    value_syntax: "auto | loose \\| normal \\| strict \\| anywhere",
    initial_value: "auto",
    inherited: true,
    animates: false,
};

pub const P_HYPHENS: PropertyDef = PropertyDef {
    id: CssPropertyId::Hyphens,
    name: "hyphens",
    value_syntax: "none | manual \\| auto",
    initial_value: "manual",
    inherited: true,
    animates: false,
};

pub const P_OVERFLOW_WRAP: PropertyDef = PropertyDef {
    id: CssPropertyId::OverflowWrap,
    name: "overflow-wrap",
    value_syntax: "normal | break-word \\| anywhere",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_TEXT_ALIGN: PropertyDef = PropertyDef {
    id: CssPropertyId::TextAlign,
    name: "text-align",
    value_syntax: "start | end \\| left \\| right \\| center \\| justify \\| match-parent \\| justify-all",
    initial_value: "start",
    inherited: true,
    animates: false,
};

pub const P_TEXT_ALIGN_ALL: PropertyDef = PropertyDef {
    id: CssPropertyId::TextAlignAll,
    name: "text-align-all",
    value_syntax: "start | end \\| left \\| right \\| center \\| justify \\| match-parent",
    initial_value: "start",
    inherited: true,
    animates: false,
};

pub const P_TEXT_ALIGN_LAST: PropertyDef = PropertyDef {
    id: CssPropertyId::TextAlignLast,
    name: "text-align-last",
    value_syntax: "auto | start \\| end \\| left \\| right \\| center \\| justify \\| match-parent",
    initial_value: "auto",
    inherited: true,
    animates: false,
};

pub const P_TEXT_JUSTIFY: PropertyDef = PropertyDef {
    id: CssPropertyId::TextJustify,
    name: "text-justify",
    value_syntax: "auto | none \\| inter-word \\| inter-character",
    initial_value: "auto",
    inherited: true,
    animates: false,
};

pub const P_WORD_SPACING: PropertyDef = PropertyDef {
    id: CssPropertyId::WordSpacing,
    name: "word-spacing",
    value_syntax: "normal | <length>",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_LETTER_SPACING: PropertyDef = PropertyDef {
    id: CssPropertyId::LetterSpacing,
    name: "letter-spacing",
    value_syntax: "normal | <length>",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_TEXT_INDENT: PropertyDef = PropertyDef {
    id: CssPropertyId::TextIndent,
    name: "text-indent",
    value_syntax: "\\[ <length-percentage> \\] && hanging? && each-line?",
    initial_value: "0",
    inherited: true,
    animates: false,
};

pub const P_HANGING_PUNCTUATION: PropertyDef = PropertyDef {
    id: CssPropertyId::HangingPunctuation,
    name: "hanging-punctuation",
    value_syntax: "none | \\[ first || \\[ force-end \\| allow-end \\] \\|\\| last \\]",
    initial_value: "none",
    inherited: true,
    animates: false,
};

pub const P_TRANSFORM: PropertyDef = PropertyDef {
    id: CssPropertyId::Transform,
    name: "transform",
    value_syntax: "none | <transform-list>",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_TRANSFORM_ORIGIN: PropertyDef = PropertyDef {
    id: CssPropertyId::TransformOrigin,
    name: "transform-origin",
    value_syntax: "\\[ left | center \\| right \\| top \\| bottom \\| <length-percentage> \\] \\| \\[ left \\| center \\| right \\| \\ \\] \\[ top \\| center \\| bottom \\| \\ \\] <length>? \\| \\[ \\[ center \\| left \\| right \\] && \\[ center \\| top \\| bottom \\] \\] \\?",
    initial_value: "50% 50%",
    inherited: false,
    animates: false,
};

pub const P_TRANSFORM_BOX: PropertyDef = PropertyDef {
    id: CssPropertyId::TransformBox,
    name: "transform-box",
    value_syntax: "content-box | border-box \\| fill-box \\| stroke-box \\| view-box",
    initial_value: "view-box",
    inherited: false,
    animates: false,
};

pub const P_TRANSITION_PROPERTY: PropertyDef = PropertyDef {
    id: CssPropertyId::TransitionProperty,
    name: "transition-property",
    value_syntax: "none | <single-transition-property>#",
    initial_value: "all",
    inherited: false,
    animates: false,
};

pub const P_TRANSITION_DURATION: PropertyDef = PropertyDef {
    id: CssPropertyId::TransitionDuration,
    name: "transition-duration",
    value_syntax: "<time [0s,∞]>#",
    initial_value: "0s",
    inherited: false,
    animates: false,
};

pub const P_TRANSITION_DELAY: PropertyDef = PropertyDef {
    id: CssPropertyId::TransitionDelay,
    name: "transition-delay",
    value_syntax: "<easing-function>#",
    initial_value: "ease",
    inherited: false,
    animates: false,
};

pub const P_TRANSITION: PropertyDef = PropertyDef {
    id: CssPropertyId::Transition,
    name: "transition",
    value_syntax: "<single-transition>#",
    initial_value: "see individual properties",
    inherited: false,
    animates: false,
};

pub const P_OUTLINE: PropertyDef = PropertyDef {
    id: CssPropertyId::Outline,
    name: "outline",
    value_syntax: "<'outline-width'> || <'outline-style'> \\|\\| <'outline-color'>",
    initial_value: "see individual properties",
    inherited: false,
    animates: false,
};

pub const P_OUTLINE_WIDTH: PropertyDef = PropertyDef {
    id: CssPropertyId::OutlineWidth,
    name: "outline-width",
    value_syntax: "<line-width>",
    initial_value: "medium",
    inherited: false,
    animates: false,
};

pub const P_OUTLINE_STYLE: PropertyDef = PropertyDef {
    id: CssPropertyId::OutlineStyle,
    name: "outline-style",
    value_syntax: "auto | <outline-line-style>",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_OUTLINE_COLOR: PropertyDef = PropertyDef {
    id: CssPropertyId::OutlineColor,
    name: "outline-color",
    value_syntax: "auto | <'border-top-color'>",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_OUTLINE_OFFSET: PropertyDef = PropertyDef {
    id: CssPropertyId::OutlineOffset,
    name: "outline-offset",
    value_syntax: "<length>",
    initial_value: "0",
    inherited: false,
    animates: false,
};

pub const P_RESIZE: PropertyDef = PropertyDef {
    id: CssPropertyId::Resize,
    name: "resize",
    value_syntax: "none | both \\| horizontal \\| vertical \\| block \\| inline",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_CURSOR: PropertyDef = PropertyDef {
    id: CssPropertyId::Cursor,
    name: "cursor",
    value_syntax: "\\[<cursor-image>,\\]* <cursor-predefined>",
    initial_value: "auto",
    inherited: true,
    animates: false,
};

pub const P_CARET_COLOR: PropertyDef = PropertyDef {
    id: CssPropertyId::CaretColor,
    name: "caret-color",
    value_syntax: "auto | <color> \\[auto \\| \\\\]?",
    initial_value: "auto",
    inherited: true,
    animates: false,
};

pub const P_CARET_ANIMATION: PropertyDef = PropertyDef {
    id: CssPropertyId::CaretAnimation,
    name: "caret-animation",
    value_syntax: "auto | manual",
    initial_value: "auto",
    inherited: true,
    animates: false,
};

pub const P_CARET_SHAPE: PropertyDef = PropertyDef {
    id: CssPropertyId::CaretShape,
    name: "caret-shape",
    value_syntax: "auto | bar \\| block \\| underscore",
    initial_value: "auto",
    inherited: true,
    animates: false,
};

pub const P_NAV_UP: PropertyDef = PropertyDef {
    id: CssPropertyId::NavUp,
    name: "nav-up",
    value_syntax: "<'caret-color'> || <'caret-animation'> \\|\\| <'caret-shape'>",
    initial_value: "auto",
    inherited: true,
    animates: false,
};

pub const P_USER_SELECT: PropertyDef = PropertyDef {
    id: CssPropertyId::UserSelect,
    name: "user-select",
    value_syntax: "auto | text \\| none \\| contain \\| all",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_POINTER_EVENTS: PropertyDef = PropertyDef {
    id: CssPropertyId::PointerEvents,
    name: "pointer-events",
    value_syntax: "auto | none",
    initial_value: "auto",
    inherited: true,
    animates: false,
};

pub const P_INTERACTIVITY: PropertyDef = PropertyDef {
    id: CssPropertyId::Interactivity,
    name: "interactivity",
    value_syntax: "auto | inert",
    initial_value: "auto",
    inherited: true,
    animates: false,
};

pub const P_INTEREST_DELAY_START: PropertyDef = PropertyDef {
    id: CssPropertyId::InterestDelayStart,
    name: "interest-delay-start",
    value_syntax: "normal | <time>",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_INTEREST_DELAY: PropertyDef = PropertyDef {
    id: CssPropertyId::InterestDelay,
    name: "interest-delay",
    value_syntax: "<'interest-delay-start'>{1,2}",
    initial_value: "see individual properties",
    inherited: false,
    animates: false,
};

pub const P_ACCENT_COLOR: PropertyDef = PropertyDef {
    id: CssPropertyId::AccentColor,
    name: "accent-color",
    value_syntax: "auto | <color>",
    initial_value: "auto",
    inherited: true,
    animates: false,
};

pub const P_APPEARANCE: PropertyDef = PropertyDef {
    id: CssPropertyId::Appearance,
    name: "appearance",
    value_syntax: "none | auto \\| base \\| base-select \\| <compat-auto> \\| <compat-special>",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_DIRECTION: PropertyDef = PropertyDef {
    id: CssPropertyId::Direction,
    name: "direction",
    value_syntax: "ltr | rtl",
    initial_value: "ltr",
    inherited: true,
    animates: false,
};

pub const P_UNICODE_BIDI: PropertyDef = PropertyDef {
    id: CssPropertyId::UnicodeBidi,
    name: "unicode-bidi",
    value_syntax: "normal | embed \\| isolate \\| bidi-override \\| isolate-override \\| plaintext",
    initial_value: "normal",
    inherited: false,
    animates: false,
};

pub const P_WRITING_MODE: PropertyDef = PropertyDef {
    id: CssPropertyId::WritingMode,
    name: "writing-mode",
    value_syntax: "horizontal-tb | vertical-rl \\| vertical-lr \\| sideways-rl \\| sideways-lr",
    initial_value: "horizontal-tb",
    inherited: true,
    animates: false,
};

pub const P_TEXT_ORIENTATION: PropertyDef = PropertyDef {
    id: CssPropertyId::TextOrientation,
    name: "text-orientation",
    value_syntax: "mixed | upright \\| sideways",
    initial_value: "mixed",
    inherited: true,
    animates: false,
};

pub const P_TEXT_COMBINE_UPRIGHT: PropertyDef = PropertyDef {
    id: CssPropertyId::TextCombineUpright,
    name: "text-combine-upright",
    value_syntax: "auto | 0deg \\| 90deg \\| 0 \\| 90",
    initial_value: "n/a",
    inherited: false,
    animates: false,
};

pub const P_PROPERTY_NAME: PropertyDef = PropertyDef {
    id: CssPropertyId::PropertyName,
    name: "property-name",
    value_syntax: "legal values & syntax",
    initial_value: "initial value",
    inherited: false,
    animates: false,
};

pub const P_MARGIN_RIGHT: PropertyDef = PropertyDef {
    id: CssPropertyId::MarginRight,
    name: "margin-right",
    value_syntax: "<margin-width> | inherit",
    initial_value: "0",
    inherited: false,
    animates: false,
};

pub const P_POSITION: PropertyDef = PropertyDef {
    id: CssPropertyId::Position,
    name: "position",
    value_syntax: "static | relative \\| absolute \\| fixed \\| inherit",
    initial_value: "static",
    inherited: false,
    animates: false,
};

pub const P_BOTTOM: PropertyDef = PropertyDef {
    id: CssPropertyId::Bottom,
    name: "bottom",
    value_syntax: "<length> | <percentage> \\| auto \\| inherit",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_Z_INDEX: PropertyDef = PropertyDef {
    id: CssPropertyId::ZIndex,
    name: "z-index",
    value_syntax: "<length> | <percentage> \\| auto \\| inherit",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_HEIGHT: PropertyDef = PropertyDef {
    id: CssPropertyId::Height,
    name: "height",
    value_syntax: "<length> | <percentage> \\| auto \\| inherit",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_MIN_HEIGHT: PropertyDef = PropertyDef {
    id: CssPropertyId::MinHeight,
    name: "min-height",
    value_syntax: "<length> | <percentage> \\| inherit",
    initial_value: "0",
    inherited: false,
    animates: false,
};

pub const P_MAX_HEIGHT: PropertyDef = PropertyDef {
    id: CssPropertyId::MaxHeight,
    name: "max-height",
    value_syntax: "<length> | <percentage> \\| none \\| inherit",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_LINE_HEIGHT: PropertyDef = PropertyDef {
    id: CssPropertyId::LineHeight,
    name: "line-height",
    value_syntax: "normal | <number> \\| <length> \\| <percentage> \\| inherit",
    initial_value: "normal",
    inherited: true,
    animates: false,
};

pub const P_VERTICAL_ALIGN: PropertyDef = PropertyDef {
    id: CssPropertyId::VerticalAlign,
    name: "vertical-align",
    value_syntax: "baseline | sub \\| super \\| top \\| text-top \\| middle \\| bottom \\| text-bottom \\| <percentage> \\| <length> \\| inherit",
    initial_value: "baseline",
    inherited: false,
    animates: false,
};

pub const P_CONTENT: PropertyDef = PropertyDef {
    id: CssPropertyId::Content,
    name: "content",
    value_syntax: "normal | none \\| \\[ <string> \\| <uri> \\| <counter> \\| attr(<identifier>) \\| open-quote \\| close-quote \\| no-open-quote \\| no-close-quote \\]+ \\| inherit",
    initial_value: "normal",
    inherited: false,
    animates: false,
};

pub const P_QUOTES: PropertyDef = PropertyDef {
    id: CssPropertyId::Quotes,
    name: "quotes",
    value_syntax: "\\[<string> \\\\]+ | none \\| inherit",
    initial_value: "depends on user agent",
    inherited: true,
    animates: false,
};

pub const P_COUNTER_RESET: PropertyDef = PropertyDef {
    id: CssPropertyId::CounterReset,
    name: "counter-reset",
    value_syntax: "\\[ <identifier> <integer>? \\]+ | none \\| inherit",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_COUNTER_INCREMENT: PropertyDef = PropertyDef {
    id: CssPropertyId::CounterIncrement,
    name: "counter-increment",
    value_syntax: "\\[ <identifier> <integer>? \\]+ | none \\| inherit",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_LIST_STYLE_TYPE: PropertyDef = PropertyDef {
    id: CssPropertyId::ListStyleType,
    name: "list-style-type",
    value_syntax: "disc | circle \\| square \\| decimal \\| decimal-leading-zero \\| lower-roman \\| upper-roman \\| lower-greek \\| lower-latin \\| upper-latin \\| armenian \\| georgian \\| lower-alpha \\| upper-alpha \\| none \\| inherit",
    initial_value: "disc",
    inherited: true,
    animates: false,
};

pub const P_LIST_STYLE_IMAGE: PropertyDef = PropertyDef {
    id: CssPropertyId::ListStyleImage,
    name: "list-style-image",
    value_syntax: "<uri> | none \\| inherit",
    initial_value: "none",
    inherited: true,
    animates: false,
};

pub const P_LIST_STYLE_POSITION: PropertyDef = PropertyDef {
    id: CssPropertyId::ListStylePosition,
    name: "list-style-position",
    value_syntax: "inside | outside \\| inherit",
    initial_value: "outside",
    inherited: true,
    animates: false,
};

pub const P_LIST_STYLE: PropertyDef = PropertyDef {
    id: CssPropertyId::ListStyle,
    name: "list-style",
    value_syntax: "\\[ <'list-style-type'> || <'list-style-position'> \\|\\| <'list-style-image'> \\] | inherit",
    initial_value: "see individual properties",
    inherited: true,
    animates: false,
};

pub const P_PAGE_BREAK_BEFORE: PropertyDef = PropertyDef {
    id: CssPropertyId::PageBreakBefore,
    name: "page-break-before",
    value_syntax: "auto | always \\| avoid \\| left \\| right \\| inherit",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_PAGE_BREAK_AFTER: PropertyDef = PropertyDef {
    id: CssPropertyId::PageBreakAfter,
    name: "page-break-after",
    value_syntax: "auto | always \\| avoid \\| left \\| right \\| inherit",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_PAGE_BREAK_INSIDE: PropertyDef = PropertyDef {
    id: CssPropertyId::PageBreakInside,
    name: "page-break-inside",
    value_syntax: "avoid | auto \\| inherit",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_ORPHANS: PropertyDef = PropertyDef {
    id: CssPropertyId::Orphans,
    name: "orphans",
    value_syntax: "<integer> | inherit",
    initial_value: "2",
    inherited: true,
    animates: false,
};

pub const P_WIDOWS: PropertyDef = PropertyDef {
    id: CssPropertyId::Widows,
    name: "widows",
    value_syntax: "<integer> | inherit",
    initial_value: "2",
    inherited: true,
    animates: false,
};

pub const P_TEXT_DECORATION: PropertyDef = PropertyDef {
    id: CssPropertyId::TextDecoration,
    name: "text-decoration",
    value_syntax: "none | \\[ underline || overline \\|\\| line-through \\|\\| blink \\] \\| inherit",
    initial_value: "none",
    inherited: false,
    animates: false,
};

pub const P_CAPTION_SIDE: PropertyDef = PropertyDef {
    id: CssPropertyId::CaptionSide,
    name: "caption-side",
    value_syntax: "top | bottom \\| inherit",
    initial_value: "top",
    inherited: true,
    animates: false,
};

pub const P_TABLE_LAYOUT: PropertyDef = PropertyDef {
    id: CssPropertyId::TableLayout,
    name: "table-layout",
    value_syntax: "auto | fixed \\| inherit",
    initial_value: "auto",
    inherited: false,
    animates: false,
};

pub const P_BORDER_COLLAPSE: PropertyDef = PropertyDef {
    id: CssPropertyId::BorderCollapse,
    name: "border-collapse",
    value_syntax: "collapse | separate \\| inherit",
    initial_value: "separate",
    inherited: true,
    animates: false,
};

pub const P_BORDER_SPACING: PropertyDef = PropertyDef {
    id: CssPropertyId::BorderSpacing,
    name: "border-spacing",
    value_syntax: "<length> \\? | inherit",
    initial_value: "0",
    inherited: true,
    animates: false,
};

pub const P_EMPTY_CELLS: PropertyDef = PropertyDef {
    id: CssPropertyId::EmptyCells,
    name: "empty-cells",
    value_syntax: "show | hide \\| inherit",
    initial_value: "show",
    inherited: true,
    animates: false,
};
