//! HTML Element Registry — generated from WHATWG HTML Living Standard.
//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentModel {
    Flow,
    Phrasing,
    Metadata,
    Heading,
    Sectioning,
    Embedded,
    Interactive,
    Palatable,
    ScriptSupporting,
    Nothing,
    Transparent,
    Text,
    Custom,
}

#[derive(Debug, Clone)]
pub struct AttrDef {
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone)]
pub struct ElementDef {
    pub tag: &'static str,
    pub content_model: ContentModel,
    pub has_global_attributes: bool,
    pub dom_interface: &'static str,
    pub specific_attributes: &'static [AttrDef],
}

pub struct ElementRegistry;

impl ElementRegistry {
    pub fn by_tag(tag: &str) -> Option<&'static ElementDef> {
        match tag {
            "html" => Some(&E_HTML),
            "head" => Some(&E_HEAD),
            "title" => Some(&E_TITLE),
            "base" => Some(&E_BASE),
            "link" => Some(&E_LINK),
            "meta" => Some(&E_META),
            "style" => Some(&E_STYLE),
            "body" => Some(&E_BODY),
            "article" => Some(&E_ARTICLE),
            "section" => Some(&E_SECTION),
            "nav" => Some(&E_NAV),
            "aside" => Some(&E_ASIDE),
            "h1" => Some(&E_H1),
            "hgroup" => Some(&E_HGROUP),
            "header" => Some(&E_HEADER),
            "footer" => Some(&E_FOOTER),
            "address" => Some(&E_ADDRESS),
            "p" => Some(&E_P),
            "hr" => Some(&E_HR),
            "pre" => Some(&E_PRE),
            "blockquote" => Some(&E_BLOCKQUOTE),
            "ol" => Some(&E_OL),
            "ul" => Some(&E_UL),
            "menu" => Some(&E_MENU),
            "li" => Some(&E_LI),
            "dl" => Some(&E_DL),
            "dt" => Some(&E_DT),
            "dd" => Some(&E_DD),
            "figure" => Some(&E_FIGURE),
            "figcaption" => Some(&E_FIGCAPTION),
            "main" => Some(&E_MAIN),
            "search" => Some(&E_SEARCH),
            "div" => Some(&E_DIV),
            "a" => Some(&E_A),
            "em" => Some(&E_EM),
            "strong" => Some(&E_STRONG),
            "small" => Some(&E_SMALL),
            "s" => Some(&E_S),
            "cite" => Some(&E_CITE),
            "q" => Some(&E_Q),
            "dfn" => Some(&E_DFN),
            "abbr" => Some(&E_ABBR),
            "ruby" => Some(&E_RUBY),
            "rt" => Some(&E_RT),
            "rp" => Some(&E_RP),
            "data" => Some(&E_DATA),
            "time" => Some(&E_TIME),
            "code" => Some(&E_CODE),
            "var" => Some(&E_VAR),
            "samp" => Some(&E_SAMP),
            "kbd" => Some(&E_KBD),
            "sub" => Some(&E_SUB),
            "i" => Some(&E_I),
            "b" => Some(&E_B),
            "u" => Some(&E_U),
            "mark" => Some(&E_MARK),
            "bdi" => Some(&E_BDI),
            "bdo" => Some(&E_BDO),
            "span" => Some(&E_SPAN),
            "br" => Some(&E_BR),
            "wbr" => Some(&E_WBR),
            "ins" => Some(&E_INS),
            "del" => Some(&E_DEL),
            "picture" => Some(&E_PICTURE),
            "source" => Some(&E_SOURCE),
            "img" => Some(&E_IMG),
            "iframe" => Some(&E_IFRAME),
            "embed" => Some(&E_EMBED),
            "object" => Some(&E_OBJECT),
            "video" => Some(&E_VIDEO),
            "audio" => Some(&E_AUDIO),
            "track" => Some(&E_TRACK),
            "map" => Some(&E_MAP),
            "area" => Some(&E_AREA),
            "table" => Some(&E_TABLE),
            "caption" => Some(&E_CAPTION),
            "colgroup" => Some(&E_COLGROUP),
            "col" => Some(&E_COL),
            "tbody" => Some(&E_TBODY),
            "thead" => Some(&E_THEAD),
            "tfoot" => Some(&E_TFOOT),
            "tr" => Some(&E_TR),
            "td" => Some(&E_TD),
            "th" => Some(&E_TH),
            "form" => Some(&E_FORM),
            "label" => Some(&E_LABEL),
            "input" => Some(&E_INPUT),
            "button" => Some(&E_BUTTON),
            "select" => Some(&E_SELECT),
            "datalist" => Some(&E_DATALIST),
            "optgroup" => Some(&E_OPTGROUP),
            "option" => Some(&E_OPTION),
            "textarea" => Some(&E_TEXTAREA),
            "output" => Some(&E_OUTPUT),
            "progress" => Some(&E_PROGRESS),
            "meter" => Some(&E_METER),
            "fieldset" => Some(&E_FIELDSET),
            "legend" => Some(&E_LEGEND),
            "selectedcontent" => Some(&E_SELECTEDCONTENT),
            "details" => Some(&E_DETAILS),
            "summary" => Some(&E_SUMMARY),
            "dialog" => Some(&E_DIALOG),
            "script" => Some(&E_SCRIPT),
            "noscript" => Some(&E_NOSCRIPT),
            "template" => Some(&E_TEMPLATE),
            "slot" => Some(&E_SLOT),
            "canvas" => Some(&E_CANVAS),
            "marquee" => Some(&E_MARQUEE),
            _ => None,
        }
    }
    pub fn all() -> impl Iterator<Item = &'static ElementDef> {
        ALL.iter().copied()
    }
}

pub const E_HTML: ElementDef = ElementDef {
    tag: "html",
    content_model: ContentModel::Custom,
    has_global_attributes: true,
    dom_interface: "HTMLHtmlElement",
    specific_attributes: &[],
};

pub const E_HEAD: ElementDef = ElementDef {
    tag: "head",
    content_model: ContentModel::Metadata,
    has_global_attributes: true,
    dom_interface: "HTMLHeadElement",
    specific_attributes: &[],
};

pub const E_TITLE: ElementDef = ElementDef {
    tag: "title",
    content_model: ContentModel::Text,
    has_global_attributes: true,
    dom_interface: "HTMLTitleElement",
    specific_attributes: &[],
};

pub const E_BASE: ElementDef = ElementDef {
    tag: "base",
    content_model: ContentModel::Nothing,
    has_global_attributes: true,
    dom_interface: "HTMLBaseElement",
    specific_attributes: &[AttrDef { name: "href", description: "Document base URL" }, AttrDef { name: "target", description: "Default navigable for hyperlink navigation and form submission" }],
};

pub const E_LINK: ElementDef = ElementDef {
    tag: "link",
    content_model: ContentModel::Nothing,
    has_global_attributes: true,
    dom_interface: "HTMLLinkElement",
    specific_attributes: &[AttrDef { name: "href", description: "Address of the hyperlink" }, AttrDef { name: "crossorigin", description: "How the element handles crossorigin requests" }, AttrDef { name: "rel", description: "Relationship between the document containing the hyperlink and the destination resource" }, AttrDef { name: "media", description: "Applicable media" }, AttrDef { name: "integrity", description: "Integrity metadata used in Subresource Integrity checks [\\[SRI\\]](#refsSRI)" }, AttrDef { name: "hreflang", description: "Language of the linked resource" }, AttrDef { name: "type", description: "Hint for the type of the referenced resource" }, AttrDef { name: "referrerpolicy", description: "Referrer policy for fetches initiated by the element" }, AttrDef { name: "sizes", description: "Sizes of the icons (for" }, AttrDef { name: "imagesrcset", description: "Images to use in different situations, e.g., high-resolution displays, small monitors, etc. (for" }, AttrDef { name: "imagesizes", description: "Image sizes for different page layouts (for [`rel`](#attr-link-rel)=\"[`preload`](#link-type-preload)\")" }, AttrDef { name: "as", description: "Destination for a preload request (for" }, AttrDef { name: "blocking", description: "Whether the element is potentially render-blocking" }, AttrDef { name: "color", description: "Color to use when customizing a site's icon (for [`rel`](#attr-link-rel)=\"`mask-icon`\")" }, AttrDef { name: "disabled", description: "Whether the link is disabled" }, AttrDef { name: "fetchpriority", description: "Sets the priority for fetches initiated by the element" }],
};

pub const E_META: ElementDef = ElementDef {
    tag: "meta",
    content_model: ContentModel::Nothing,
    has_global_attributes: true,
    dom_interface: "HTMLMetaElement",
    specific_attributes: &[AttrDef { name: "name", description: "Metadata name" }, AttrDef { name: "content", description: "Value of the element" }, AttrDef { name: "charset", description: "Character encoding declaration" }, AttrDef { name: "media", description: "Applicable media" }],
};

pub const E_STYLE: ElementDef = ElementDef {
    tag: "style",
    content_model: ContentModel::Text,
    has_global_attributes: true,
    dom_interface: "HTMLStyleElement",
    specific_attributes: &[AttrDef { name: "media", description: "Applicable media" }, AttrDef { name: "blocking", description: "Whether the element is potentially render-blocking" }],
};

pub const E_BODY: ElementDef = ElementDef {
    tag: "body",
    content_model: ContentModel::Flow,
    has_global_attributes: true,
    dom_interface: "HTMLBodyElement",
    specific_attributes: &[],
};

pub const E_ARTICLE: ElementDef = ElementDef {
    tag: "article",
    content_model: ContentModel::Flow,
    has_global_attributes: false,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_SECTION: ElementDef = ElementDef {
    tag: "section",
    content_model: ContentModel::Flow,
    has_global_attributes: false,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_NAV: ElementDef = ElementDef {
    tag: "nav",
    content_model: ContentModel::Flow,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_ASIDE: ElementDef = ElementDef {
    tag: "aside",
    content_model: ContentModel::Flow,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_H1: ElementDef = ElementDef {
    tag: "h1",
    content_model: ContentModel::Phrasing,
    has_global_attributes: false,
    dom_interface: "HTMLHeadingElement",
    specific_attributes: &[],
};

pub const E_HGROUP: ElementDef = ElementDef {
    tag: "hgroup",
    content_model: ContentModel::ScriptSupporting,
    has_global_attributes: false,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_HEADER: ElementDef = ElementDef {
    tag: "header",
    content_model: ContentModel::Flow,
    has_global_attributes: false,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_FOOTER: ElementDef = ElementDef {
    tag: "footer",
    content_model: ContentModel::Flow,
    has_global_attributes: false,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_ADDRESS: ElementDef = ElementDef {
    tag: "address",
    content_model: ContentModel::Heading,
    has_global_attributes: false,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_P: ElementDef = ElementDef {
    tag: "p",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLParagraphElement",
    specific_attributes: &[],
};

pub const E_HR: ElementDef = ElementDef {
    tag: "hr",
    content_model: ContentModel::Nothing,
    has_global_attributes: true,
    dom_interface: "HTMLHRElement",
    specific_attributes: &[],
};

pub const E_PRE: ElementDef = ElementDef {
    tag: "pre",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLPreElement",
    specific_attributes: &[],
};

pub const E_BLOCKQUOTE: ElementDef = ElementDef {
    tag: "blockquote",
    content_model: ContentModel::Flow,
    has_global_attributes: false,
    dom_interface: "HTMLQuoteElement",
    specific_attributes: &[],
};

pub const E_OL: ElementDef = ElementDef {
    tag: "ol",
    content_model: ContentModel::ScriptSupporting,
    has_global_attributes: true,
    dom_interface: "HTMLOListElement",
    specific_attributes: &[AttrDef { name: "reversed", description: "Number the list backwards" }, AttrDef { name: "start", description: "Starting value of the list" }, AttrDef { name: "type", description: "Kind of list marker" }],
};

pub const E_UL: ElementDef = ElementDef {
    tag: "ul",
    content_model: ContentModel::ScriptSupporting,
    has_global_attributes: true,
    dom_interface: "HTMLUListElement",
    specific_attributes: &[],
};

pub const E_MENU: ElementDef = ElementDef {
    tag: "menu",
    content_model: ContentModel::ScriptSupporting,
    has_global_attributes: true,
    dom_interface: "HTMLMenuElement",
    specific_attributes: &[],
};

pub const E_LI: ElementDef = ElementDef {
    tag: "li",
    content_model: ContentModel::Flow,
    has_global_attributes: true,
    dom_interface: "HTMLLIElement",
    specific_attributes: &[AttrDef { name: "value", description: "Ordinal value of the list item" }],
};

pub const E_DL: ElementDef = ElementDef {
    tag: "dl",
    content_model: ContentModel::ScriptSupporting,
    has_global_attributes: true,
    dom_interface: "HTMLDListElement",
    specific_attributes: &[],
};

pub const E_DT: ElementDef = ElementDef {
    tag: "dt",
    content_model: ContentModel::Heading,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_DD: ElementDef = ElementDef {
    tag: "dd",
    content_model: ContentModel::Flow,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_FIGURE: ElementDef = ElementDef {
    tag: "figure",
    content_model: ContentModel::Flow,
    has_global_attributes: false,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_FIGCAPTION: ElementDef = ElementDef {
    tag: "figcaption",
    content_model: ContentModel::Flow,
    has_global_attributes: false,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_MAIN: ElementDef = ElementDef {
    tag: "main",
    content_model: ContentModel::Flow,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_SEARCH: ElementDef = ElementDef {
    tag: "search",
    content_model: ContentModel::Flow,
    has_global_attributes: false,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_DIV: ElementDef = ElementDef {
    tag: "div",
    content_model: ContentModel::ScriptSupporting,
    has_global_attributes: true,
    dom_interface: "HTMLDivElement",
    specific_attributes: &[],
};

pub const E_A: ElementDef = ElementDef {
    tag: "a",
    content_model: ContentModel::Transparent,
    has_global_attributes: true,
    dom_interface: "HTMLAnchorElement",
    specific_attributes: &[AttrDef { name: "href", description: "Address of the hyperlink" }, AttrDef { name: "target", description: "Navigable for hyperlink navigation" }, AttrDef { name: "download", description: "Whether to download the resource instead of navigating to it, and its filename if so" }, AttrDef { name: "ping", description: "URLs to ping" }, AttrDef { name: "rel", description: "Relationship between the location in the document containing the hyperlink and the destination resource" }, AttrDef { name: "hreflang", description: "Language of the linked resource" }, AttrDef { name: "type", description: "Hint for the type of the referenced resource" }, AttrDef { name: "referrerpolicy", description: "Referrer policy for fetches initiated by the element" }],
};

pub const E_EM: ElementDef = ElementDef {
    tag: "em",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_STRONG: ElementDef = ElementDef {
    tag: "strong",
    content_model: ContentModel::Phrasing,
    has_global_attributes: false,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_SMALL: ElementDef = ElementDef {
    tag: "small",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_S: ElementDef = ElementDef {
    tag: "s",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_CITE: ElementDef = ElementDef {
    tag: "cite",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_Q: ElementDef = ElementDef {
    tag: "q",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[AttrDef { name: "cite", description: "Link to the source of the quotation or more information about the edit" }],
};

pub const E_DFN: ElementDef = ElementDef {
    tag: "dfn",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_ABBR: ElementDef = ElementDef {
    tag: "abbr",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_RUBY: ElementDef = ElementDef {
    tag: "ruby",
    content_model: ContentModel::Custom,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_RT: ElementDef = ElementDef {
    tag: "rt",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_RP: ElementDef = ElementDef {
    tag: "rp",
    content_model: ContentModel::Text,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_DATA: ElementDef = ElementDef {
    tag: "data",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLDataElement",
    specific_attributes: &[AttrDef { name: "value", description: "Machine-readable value" }],
};

pub const E_TIME: ElementDef = ElementDef {
    tag: "time",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLTimeElement",
    specific_attributes: &[AttrDef { name: "datetime", description: "Machine-readable value" }],
};

pub const E_CODE: ElementDef = ElementDef {
    tag: "code",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_VAR: ElementDef = ElementDef {
    tag: "var",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_SAMP: ElementDef = ElementDef {
    tag: "samp",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_KBD: ElementDef = ElementDef {
    tag: "kbd",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_SUB: ElementDef = ElementDef {
    tag: "sub",
    content_model: ContentModel::Phrasing,
    has_global_attributes: false,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_I: ElementDef = ElementDef {
    tag: "i",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_B: ElementDef = ElementDef {
    tag: "b",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_U: ElementDef = ElementDef {
    tag: "u",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_MARK: ElementDef = ElementDef {
    tag: "mark",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_BDI: ElementDef = ElementDef {
    tag: "bdi",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_BDO: ElementDef = ElementDef {
    tag: "bdo",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_SPAN: ElementDef = ElementDef {
    tag: "span",
    content_model: ContentModel::Custom,
    has_global_attributes: true,
    dom_interface: "HTMLSpanElement",
    specific_attributes: &[],
};

pub const E_BR: ElementDef = ElementDef {
    tag: "br",
    content_model: ContentModel::Nothing,
    has_global_attributes: true,
    dom_interface: "HTMLBRElement",
    specific_attributes: &[],
};

pub const E_WBR: ElementDef = ElementDef {
    tag: "wbr",
    content_model: ContentModel::Nothing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_INS: ElementDef = ElementDef {
    tag: "ins",
    content_model: ContentModel::Transparent,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[AttrDef { name: "cite", description: "Link to the source of the quotation or more information about the edit" }, AttrDef { name: "datetime", description: "Date and (optionally) time of the change" }],
};

pub const E_DEL: ElementDef = ElementDef {
    tag: "del",
    content_model: ContentModel::Transparent,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[AttrDef { name: "cite", description: "Link to the source of the quotation or more information about the edit" }, AttrDef { name: "datetime", description: "Date and (optionally) time of the change" }],
};

pub const E_PICTURE: ElementDef = ElementDef {
    tag: "picture",
    content_model: ContentModel::ScriptSupporting,
    has_global_attributes: false,
    dom_interface: "HTMLPictureElement",
    specific_attributes: &[],
};

pub const E_SOURCE: ElementDef = ElementDef {
    tag: "source",
    content_model: ContentModel::Nothing,
    has_global_attributes: false,
    dom_interface: "HTMLSourceElement",
    specific_attributes: &[],
};

pub const E_IMG: ElementDef = ElementDef {
    tag: "img",
    content_model: ContentModel::Nothing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[AttrDef { name: "alt", description: "Replacement text for use when images are not available" }, AttrDef { name: "src", description: "Address of the resource" }, AttrDef { name: "srcset", description: "Images to use in different situations, e.g., high-resolution displays, small monitors, etc." }, AttrDef { name: "sizes", description: "Image sizes for different page layouts" }, AttrDef { name: "crossorigin", description: "How the element handles crossorigin requests" }, AttrDef { name: "usemap", description: "Name of image map to use" }, AttrDef { name: "ismap", description: "Whether the image is a server-side image map" }, AttrDef { name: "width", description: "Horizontal dimension" }, AttrDef { name: "height", description: "Vertical dimension" }, AttrDef { name: "referrerpolicy", description: "Referrer policy for fetches initiated by the element" }, AttrDef { name: "decoding", description: "Decoding hint to use when processing this image for presentation" }, AttrDef { name: "loading", description: "Used when determining loading deferral" }, AttrDef { name: "fetchpriority", description: "Sets the priority for fetches initiated by the element" }],
};

pub const E_IFRAME: ElementDef = ElementDef {
    tag: "iframe",
    content_model: ContentModel::Nothing,
    has_global_attributes: false,
    dom_interface: "HTMLIFrameElement",
    specific_attributes: &[],
};

pub const E_EMBED: ElementDef = ElementDef {
    tag: "embed",
    content_model: ContentModel::Nothing,
    has_global_attributes: true,
    dom_interface: "HTMLEmbedElement",
    specific_attributes: &[AttrDef { name: "src", description: "Address of the resource" }, AttrDef { name: "type", description: "Type of embedded resource" }, AttrDef { name: "width", description: "Horizontal dimension" }, AttrDef { name: "height", description: "Vertical dimension" }],
};

pub const E_OBJECT: ElementDef = ElementDef {
    tag: "object",
    content_model: ContentModel::Transparent,
    has_global_attributes: false,
    dom_interface: "HTMLObjectElement",
    specific_attributes: &[],
};

pub const E_VIDEO: ElementDef = ElementDef {
    tag: "video",
    content_model: ContentModel::Transparent,
    has_global_attributes: true,
    dom_interface: "HTMLVideoElement",
    specific_attributes: &[AttrDef { name: "src", description: "Address of the resource" }, AttrDef { name: "crossorigin", description: "How the element handles crossorigin requests" }, AttrDef { name: "poster", description: "Poster frame to show prior to video playback" }, AttrDef { name: "preload", description: "Hints how much buffering the media resource will likely need" }, AttrDef { name: "autoplay", description: "Hint that the media resource can be started automatically when the page is loaded" }, AttrDef { name: "playsinline", description: "Encourage the user agent to display video content within the element's playback area" }, AttrDef { name: "loop", description: "Whether to loop the media resource" }, AttrDef { name: "muted", description: "Whether to mute the media resource by default" }, AttrDef { name: "controls", description: "Show user agent controls" }, AttrDef { name: "loading", description: "Used when determining loading deferral" }, AttrDef { name: "width", description: "Horizontal dimension" }, AttrDef { name: "height", description: "Vertical dimension" }],
};

pub const E_AUDIO: ElementDef = ElementDef {
    tag: "audio",
    content_model: ContentModel::Transparent,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[AttrDef { name: "src", description: "Address of the resource" }, AttrDef { name: "crossorigin", description: "How the element handles crossorigin requests" }, AttrDef { name: "preload", description: "Hints how much buffering the media resource will likely need" }, AttrDef { name: "autoplay", description: "Hint that the media resource can be started automatically when the page is loaded" }, AttrDef { name: "loop", description: "Whether to loop the media resource" }, AttrDef { name: "muted", description: "Whether to mute the media resource by default" }, AttrDef { name: "controls", description: "Show user agent controls" }, AttrDef { name: "loading", description: "Used when determining loading deferral" }],
};

pub const E_TRACK: ElementDef = ElementDef {
    tag: "track",
    content_model: ContentModel::Nothing,
    has_global_attributes: true,
    dom_interface: "HTMLTrackElement",
    specific_attributes: &[AttrDef { name: "kind", description: "The type of text track" }, AttrDef { name: "src", description: "Address of the resource" }, AttrDef { name: "srclang", description: "Language of the text track" }, AttrDef { name: "label", description: "User-visible label" }, AttrDef { name: "default", description: "Enable the track if no other text track is more suitable" }],
};

pub const E_MAP: ElementDef = ElementDef {
    tag: "map",
    content_model: ContentModel::Transparent,
    has_global_attributes: true,
    dom_interface: "HTMLMapElement",
    specific_attributes: &[AttrDef { name: "name", description: "Name of image map to reference from the [`usemap`](#attr-hyperlink-usemap) attribute" }],
};

pub const E_AREA: ElementDef = ElementDef {
    tag: "area",
    content_model: ContentModel::Nothing,
    has_global_attributes: true,
    dom_interface: "HTMLAreaElement",
    specific_attributes: &[AttrDef { name: "alt", description: "Replacement text for use when images are not available" }, AttrDef { name: "coords", description: "Coordinates for the shape to be created in an image map" }, AttrDef { name: "shape", description: "The kind of shape to be created in an image map" }, AttrDef { name: "href", description: "Address of the hyperlink" }, AttrDef { name: "target", description: "Navigable for hyperlink navigation" }, AttrDef { name: "download", description: "Whether to download the resource instead of navigating to it, and its filename if so" }, AttrDef { name: "ping", description: "URLs to ping" }, AttrDef { name: "rel", description: "Relationship between the location in the document containing the hyperlink and the destination resource" }, AttrDef { name: "referrerpolicy", description: "Referrer policy for fetches initiated by the element" }],
};

pub const E_TABLE: ElementDef = ElementDef {
    tag: "table",
    content_model: ContentModel::ScriptSupporting,
    has_global_attributes: true,
    dom_interface: "HTMLTableElement",
    specific_attributes: &[],
};

pub const E_CAPTION: ElementDef = ElementDef {
    tag: "caption",
    content_model: ContentModel::Flow,
    has_global_attributes: false,
    dom_interface: "HTMLTableCaptionElement",
    specific_attributes: &[],
};

pub const E_COLGROUP: ElementDef = ElementDef {
    tag: "colgroup",
    content_model: ContentModel::Custom,
    has_global_attributes: false,
    dom_interface: "HTMLTableColElement",
    specific_attributes: &[],
};

pub const E_COL: ElementDef = ElementDef {
    tag: "col",
    content_model: ContentModel::Nothing,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[AttrDef { name: "span", description: "Number of columns spanned by the element" }],
};

pub const E_TBODY: ElementDef = ElementDef {
    tag: "tbody",
    content_model: ContentModel::ScriptSupporting,
    has_global_attributes: true,
    dom_interface: "HTMLTableSectionElement",
    specific_attributes: &[],
};

pub const E_THEAD: ElementDef = ElementDef {
    tag: "thead",
    content_model: ContentModel::ScriptSupporting,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_TFOOT: ElementDef = ElementDef {
    tag: "tfoot",
    content_model: ContentModel::ScriptSupporting,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_TR: ElementDef = ElementDef {
    tag: "tr",
    content_model: ContentModel::ScriptSupporting,
    has_global_attributes: true,
    dom_interface: "HTMLTableRowElement",
    specific_attributes: &[],
};

pub const E_TD: ElementDef = ElementDef {
    tag: "td",
    content_model: ContentModel::Flow,
    has_global_attributes: true,
    dom_interface: "HTMLTableCellElement",
    specific_attributes: &[AttrDef { name: "colspan", description: "Number of columns that the cell is to span" }, AttrDef { name: "rowspan", description: "Number of rows that the cell is to span" }, AttrDef { name: "headers", description: "The header cells for this cell" }],
};

pub const E_TH: ElementDef = ElementDef {
    tag: "th",
    content_model: ContentModel::Heading,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[AttrDef { name: "colspan", description: "Number of columns that the cell is to span" }, AttrDef { name: "rowspan", description: "Number of rows that the cell is to span" }, AttrDef { name: "headers", description: "The header cells for this cell" }, AttrDef { name: "scope", description: "Specifies which cells the header cell applies to" }, AttrDef { name: "abbr", description: "Alternative label to use for the header cell when referencing the cell in other contexts" }],
};

pub const E_FORM: ElementDef = ElementDef {
    tag: "form",
    content_model: ContentModel::Flow,
    has_global_attributes: true,
    dom_interface: "HTMLElement",
    specific_attributes: &[AttrDef { name: "action", description: "URL to use for form submission" }, AttrDef { name: "autocomplete", description: "Default setting for autofill feature for controls in the form" }, AttrDef { name: "enctype", description: "Entry list encoding type to use for form submission" }, AttrDef { name: "method", description: "Variant to use for form submission" }, AttrDef { name: "name", description: "Name of form to use in the" }, AttrDef { name: "novalidate", description: "Bypass form control validation for form submission" }, AttrDef { name: "target", description: "Navigable for form submission" }],
};

pub const E_LABEL: ElementDef = ElementDef {
    tag: "label",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLLabelElement",
    specific_attributes: &[AttrDef { name: "for", description: "Associate the label with form control" }],
};

pub const E_INPUT: ElementDef = ElementDef {
    tag: "input",
    content_model: ContentModel::Nothing,
    has_global_attributes: true,
    dom_interface: "HTMLInputElement",
    specific_attributes: &[AttrDef { name: "accept", description: "Hint for expected file type in file upload controls" }, AttrDef { name: "alpha", description: "Allow the color's alpha component to be set" }, AttrDef { name: "alt", description: "Replacement text for use when images are not available" }, AttrDef { name: "autocomplete", description: "Hint for form autofill feature" }, AttrDef { name: "checked", description: "Whether the control is checked" }, AttrDef { name: "colorspace", description: "The color space of the serialized color" }, AttrDef { name: "dirname", description: "Name of form control to use for sending the element's directionality in form submission" }, AttrDef { name: "disabled", description: "Whether the form control is disabled" }, AttrDef { name: "form", description: "Associates the element with a" }, AttrDef { name: "formaction", description: "URL to use for form submission" }, AttrDef { name: "formenctype", description: "Entry list encoding type to use for form submission" }, AttrDef { name: "formmethod", description: "Variant to use for form submission" }, AttrDef { name: "formnovalidate", description: "Bypass form control validation for form submission" }, AttrDef { name: "formtarget", description: "Navigable for form submission" }, AttrDef { name: "height", description: "Vertical dimension" }, AttrDef { name: "list", description: "List of autocomplete options" }, AttrDef { name: "max", description: "Maximum value" }, AttrDef { name: "maxlength", description: "Maximum length of value" }, AttrDef { name: "min", description: "Minimum value" }, AttrDef { name: "minlength", description: "Minimum length of value" }, AttrDef { name: "multiple", description: "Whether to allow multiple values" }, AttrDef { name: "name", description: "Name of the element to use for form submission and in the [`form.elements`](#dom-form-elements) API" }, AttrDef { name: "pattern", description: "Pattern to be matched by the form control's value" }, AttrDef { name: "placeholder", description: "User-visible label to be placed within the form control" }, AttrDef { name: "popovertarget", description: "Targets a popover element to toggle, show, or hide" }, AttrDef { name: "popovertargetaction", description: "Indicates whether a targeted popover element is to be toggled, shown, or hidden" }, AttrDef { name: "readonly", description: "Whether to allow the value to be edited by the user" }, AttrDef { name: "required", description: "Whether the control is required for form submission" }, AttrDef { name: "size", description: "Size of the control" }, AttrDef { name: "src", description: "Address of the resource" }, AttrDef { name: "step", description: "Granularity to be matched by the form control's value" }, AttrDef { name: "type", description: "Type of form control" }, AttrDef { name: "value", description: "Value of the form control" }, AttrDef { name: "width", description: "Horizontal dimension" }],
};

pub const E_BUTTON: ElementDef = ElementDef {
    tag: "button",
    content_model: ContentModel::Phrasing,
    has_global_attributes: false,
    dom_interface: "HTMLButtonElement",
    specific_attributes: &[],
};

pub const E_SELECT: ElementDef = ElementDef {
    tag: "select",
    content_model: ContentModel::Custom,
    has_global_attributes: false,
    dom_interface: "HTMLSelectElement",
    specific_attributes: &[],
};

pub const E_DATALIST: ElementDef = ElementDef {
    tag: "datalist",
    content_model: ContentModel::Phrasing,
    has_global_attributes: false,
    dom_interface: "HTMLDataListElement",
    specific_attributes: &[],
};

pub const E_OPTGROUP: ElementDef = ElementDef {
    tag: "optgroup",
    content_model: ContentModel::Custom,
    has_global_attributes: false,
    dom_interface: "HTMLOptGroupElement",
    specific_attributes: &[],
};

pub const E_OPTION: ElementDef = ElementDef {
    tag: "option",
    content_model: ContentModel::Custom,
    has_global_attributes: false,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_TEXTAREA: ElementDef = ElementDef {
    tag: "textarea",
    content_model: ContentModel::Text,
    has_global_attributes: false,
    dom_interface: "HTMLTextAreaElement",
    specific_attributes: &[],
};

pub const E_OUTPUT: ElementDef = ElementDef {
    tag: "output",
    content_model: ContentModel::Phrasing,
    has_global_attributes: false,
    dom_interface: "HTMLOutputElement",
    specific_attributes: &[],
};

pub const E_PROGRESS: ElementDef = ElementDef {
    tag: "progress",
    content_model: ContentModel::Phrasing,
    has_global_attributes: false,
    dom_interface: "HTMLProgressElement",
    specific_attributes: &[],
};

pub const E_METER: ElementDef = ElementDef {
    tag: "meter",
    content_model: ContentModel::Phrasing,
    has_global_attributes: true,
    dom_interface: "HTMLMeterElement",
    specific_attributes: &[AttrDef { name: "value", description: "Current value of the element" }, AttrDef { name: "min", description: "Lower bound of range" }, AttrDef { name: "max", description: "Upper bound of range" }, AttrDef { name: "low", description: "High limit of low range" }, AttrDef { name: "high", description: "Low limit of high range" }, AttrDef { name: "optimum", description: "Optimum value in gauge" }],
};

pub const E_FIELDSET: ElementDef = ElementDef {
    tag: "fieldset",
    content_model: ContentModel::Flow,
    has_global_attributes: false,
    dom_interface: "HTMLFieldSetElement",
    specific_attributes: &[],
};

pub const E_LEGEND: ElementDef = ElementDef {
    tag: "legend",
    content_model: ContentModel::Phrasing,
    has_global_attributes: false,
    dom_interface: "HTMLLegendElement",
    specific_attributes: &[],
};

pub const E_SELECTEDCONTENT: ElementDef = ElementDef {
    tag: "selectedcontent",
    content_model: ContentModel::Nothing,
    has_global_attributes: false,
    dom_interface: "HTMLSelectedContentElement",
    specific_attributes: &[],
};

pub const E_DETAILS: ElementDef = ElementDef {
    tag: "details",
    content_model: ContentModel::Flow,
    has_global_attributes: false,
    dom_interface: "HTMLDetailsElement",
    specific_attributes: &[],
};

pub const E_SUMMARY: ElementDef = ElementDef {
    tag: "summary",
    content_model: ContentModel::Heading,
    has_global_attributes: false,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_DIALOG: ElementDef = ElementDef {
    tag: "dialog",
    content_model: ContentModel::Flow,
    has_global_attributes: false,
    dom_interface: "HTMLDialogElement",
    specific_attributes: &[],
};

pub const E_SCRIPT: ElementDef = ElementDef {
    tag: "script",
    content_model: ContentModel::Custom,
    has_global_attributes: false,
    dom_interface: "HTMLScriptElement",
    specific_attributes: &[],
};

pub const E_NOSCRIPT: ElementDef = ElementDef {
    tag: "noscript",
    content_model: ContentModel::Custom,
    has_global_attributes: false,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_TEMPLATE: ElementDef = ElementDef {
    tag: "template",
    content_model: ContentModel::Custom,
    has_global_attributes: false,
    dom_interface: "HTMLTemplateElement",
    specific_attributes: &[],
};

pub const E_SLOT: ElementDef = ElementDef {
    tag: "slot",
    content_model: ContentModel::Transparent,
    has_global_attributes: true,
    dom_interface: "HTMLSlotElement",
    specific_attributes: &[AttrDef { name: "name", description: "Name of shadow tree slot" }],
};

pub const E_CANVAS: ElementDef = ElementDef {
    tag: "canvas",
    content_model: ContentModel::Transparent,
    has_global_attributes: false,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

pub const E_MARQUEE: ElementDef = ElementDef {
    tag: "marquee",
    content_model: ContentModel::Custom,
    has_global_attributes: false,
    dom_interface: "HTMLElement",
    specific_attributes: &[],
};

const ALL: &[&ElementDef] = &[
    &E_HTML,
    &E_HEAD,
    &E_TITLE,
    &E_BASE,
    &E_LINK,
    &E_META,
    &E_STYLE,
    &E_BODY,
    &E_ARTICLE,
    &E_SECTION,
    &E_NAV,
    &E_ASIDE,
    &E_H1,
    &E_HGROUP,
    &E_HEADER,
    &E_FOOTER,
    &E_ADDRESS,
    &E_P,
    &E_HR,
    &E_PRE,
    &E_BLOCKQUOTE,
    &E_OL,
    &E_UL,
    &E_MENU,
    &E_LI,
    &E_DL,
    &E_DT,
    &E_DD,
    &E_FIGURE,
    &E_FIGCAPTION,
    &E_MAIN,
    &E_SEARCH,
    &E_DIV,
    &E_A,
    &E_EM,
    &E_STRONG,
    &E_SMALL,
    &E_S,
    &E_CITE,
    &E_Q,
    &E_DFN,
    &E_ABBR,
    &E_RUBY,
    &E_RT,
    &E_RP,
    &E_DATA,
    &E_TIME,
    &E_CODE,
    &E_VAR,
    &E_SAMP,
    &E_KBD,
    &E_SUB,
    &E_I,
    &E_B,
    &E_U,
    &E_MARK,
    &E_BDI,
    &E_BDO,
    &E_SPAN,
    &E_BR,
    &E_WBR,
    &E_INS,
    &E_DEL,
    &E_PICTURE,
    &E_SOURCE,
    &E_IMG,
    &E_IFRAME,
    &E_EMBED,
    &E_OBJECT,
    &E_VIDEO,
    &E_AUDIO,
    &E_TRACK,
    &E_MAP,
    &E_AREA,
    &E_TABLE,
    &E_CAPTION,
    &E_COLGROUP,
    &E_COL,
    &E_TBODY,
    &E_THEAD,
    &E_TFOOT,
    &E_TR,
    &E_TD,
    &E_TH,
    &E_FORM,
    &E_LABEL,
    &E_INPUT,
    &E_BUTTON,
    &E_SELECT,
    &E_DATALIST,
    &E_OPTGROUP,
    &E_OPTION,
    &E_TEXTAREA,
    &E_OUTPUT,
    &E_PROGRESS,
    &E_METER,
    &E_FIELDSET,
    &E_LEGEND,
    &E_SELECTEDCONTENT,
    &E_DETAILS,
    &E_SUMMARY,
    &E_DIALOG,
    &E_SCRIPT,
    &E_NOSCRIPT,
    &E_TEMPLATE,
    &E_SLOT,
    &E_CANVAS,
    &E_MARQUEE,
];