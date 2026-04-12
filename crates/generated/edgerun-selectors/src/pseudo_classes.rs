//! Pseudo-class selectors — generated from Selectors Level 4.
//! DO NOT EDIT. Regenerate with: scripts/generate_selector_dom.py
extern crate alloc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PseudoClass {
    /// :root
    Root,
    /// :empty
    Empty,
    /// :nth-child
    NthChild,
    /// :nth-last-child
    NthLastChild,
    /// :first-child
    FirstChild,
    /// :last-child
    LastChild,
    /// :only-child
    OnlyChild,
    /// :nth-of-type
    NthOfType,
    /// :nth-last-of-type
    NthLastOfType,
    /// :first-of-type
    FirstOfType,
    /// :last-of-type
    LastOfType,
    /// :only-of-type
    OnlyOfType,
    /// :is (forgiving selector list)
    Is,
    /// :where (forgiving selector list)
    Where,
    /// :not (forgiving selector list)
    Not,
    /// :has (forgiving selector list)
    Has,
    /// :hover
    Hover,
    /// :active
    Active,
    /// :focus
    Focus,
    /// :focus-within
    FocusWithin,
    /// :focus-visible
    FocusVisible,
    /// :visited
    Visited,
    /// :link
    Link,
    /// :target
    Target,
    /// :target-within
    TargetWithin,
    /// :lang (language list)
    Lang,
    /// :dir (ltr or rtl)
    Dir,
    /// :any-link
    AnyLink,
    /// :local-link
    LocalLink,
    /// :scope
    Scope,
    /// :defined
    Defined,
    /// :checked
    Checked,
    /// :indeterminate
    Indeterminate,
    /// :default
    Default,
    /// :valid
    Valid,
    /// :invalid
    Invalid,
    /// :in-range
    InRange,
    /// :out-of-range
    OutOfRange,
    /// :required
    Required,
    /// :optional
    Optional,
    /// :blank
    Blank,
    /// :placeholder-shown
    PlaceholderShown,
    /// :read-only
    ReadOnly,
    /// :read-write
    ReadWrite,
    /// :disabled
    Disabled,
    /// :enabled
    Enabled,
    /// :playing
    Playing,
    /// :paused
    Paused,
    /// :current (selector list)
    Current,
    /// :past (selector list)
    Past,
    /// :future (selector list)
    Future,
    /// :host (selector list)
    Host,
    /// :host-context (selector list)
    HostContext,
    /// :popover-open
    PopoverOpen,
    /// :modal
    Modal,
    /// :fullscreen
    Fullscreen,
    /// :picture-in-picture
    PictureInPicture,
    /// :autofill
    Autofill,
    /// :user-invalid
    UserInvalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PseudoClassSelector {
    Root,
    Empty,
    NthChild,
    NthLastChild,
    FirstChild,
    LastChild,
    OnlyChild,
    NthOfType,
    NthLastOfType,
    FirstOfType,
    LastOfType,
    OnlyOfType,
    Is,
    Where,
    Not,
    Has,
    Hover,
    Active,
    Focus,
    FocusWithin,
    FocusVisible,
    Visited,
    Link,
    Target,
    TargetWithin,
    Lang,
    Dir,
    AnyLink,
    LocalLink,
    Scope,
    Defined,
    Checked,
    Indeterminate,
    Default,
    Valid,
    Invalid,
    InRange,
    OutOfRange,
    Required,
    Optional,
    Blank,
    PlaceholderShown,
    ReadOnly,
    ReadWrite,
    Disabled,
    Enabled,
    Playing,
    Paused,
    Current,
    Past,
    Future,
    Host,
    HostContext,
    PopoverOpen,
    Modal,
    Fullscreen,
    PictureInPicture,
    Autofill,
    UserInvalid,
}

impl PseudoClassSelector {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            ":root" => Some(PseudoClassSelector::Root),
            ":empty" => Some(PseudoClassSelector::Empty),
            ":nth-child" => Some(PseudoClassSelector::NthChild),
            ":nth-last-child" => Some(PseudoClassSelector::NthLastChild),
            ":first-child" => Some(PseudoClassSelector::FirstChild),
            ":last-child" => Some(PseudoClassSelector::LastChild),
            ":only-child" => Some(PseudoClassSelector::OnlyChild),
            ":nth-of-type" => Some(PseudoClassSelector::NthOfType),
            ":nth-last-of-type" => Some(PseudoClassSelector::NthLastOfType),
            ":first-of-type" => Some(PseudoClassSelector::FirstOfType),
            ":last-of-type" => Some(PseudoClassSelector::LastOfType),
            ":only-of-type" => Some(PseudoClassSelector::OnlyOfType),
            ":is" => Some(PseudoClassSelector::Is),
            ":where" => Some(PseudoClassSelector::Where),
            ":not" => Some(PseudoClassSelector::Not),
            ":has" => Some(PseudoClassSelector::Has),
            ":hover" => Some(PseudoClassSelector::Hover),
            ":active" => Some(PseudoClassSelector::Active),
            ":focus" => Some(PseudoClassSelector::Focus),
            ":focus-within" => Some(PseudoClassSelector::FocusWithin),
            ":focus-visible" => Some(PseudoClassSelector::FocusVisible),
            ":visited" => Some(PseudoClassSelector::Visited),
            ":link" => Some(PseudoClassSelector::Link),
            ":target" => Some(PseudoClassSelector::Target),
            ":target-within" => Some(PseudoClassSelector::TargetWithin),
            ":lang" => Some(PseudoClassSelector::Lang),
            ":dir" => Some(PseudoClassSelector::Dir),
            ":any-link" => Some(PseudoClassSelector::AnyLink),
            ":local-link" => Some(PseudoClassSelector::LocalLink),
            ":scope" => Some(PseudoClassSelector::Scope),
            ":defined" => Some(PseudoClassSelector::Defined),
            ":checked" => Some(PseudoClassSelector::Checked),
            ":indeterminate" => Some(PseudoClassSelector::Indeterminate),
            ":default" => Some(PseudoClassSelector::Default),
            ":valid" => Some(PseudoClassSelector::Valid),
            ":invalid" => Some(PseudoClassSelector::Invalid),
            ":in-range" => Some(PseudoClassSelector::InRange),
            ":out-of-range" => Some(PseudoClassSelector::OutOfRange),
            ":required" => Some(PseudoClassSelector::Required),
            ":optional" => Some(PseudoClassSelector::Optional),
            ":blank" => Some(PseudoClassSelector::Blank),
            ":placeholder-shown" => Some(PseudoClassSelector::PlaceholderShown),
            ":read-only" => Some(PseudoClassSelector::ReadOnly),
            ":read-write" => Some(PseudoClassSelector::ReadWrite),
            ":disabled" => Some(PseudoClassSelector::Disabled),
            ":enabled" => Some(PseudoClassSelector::Enabled),
            ":playing" => Some(PseudoClassSelector::Playing),
            ":paused" => Some(PseudoClassSelector::Paused),
            ":current" => Some(PseudoClassSelector::Current),
            ":past" => Some(PseudoClassSelector::Past),
            ":future" => Some(PseudoClassSelector::Future),
            ":host" => Some(PseudoClassSelector::Host),
            ":host-context" => Some(PseudoClassSelector::HostContext),
            ":popover-open" => Some(PseudoClassSelector::PopoverOpen),
            ":modal" => Some(PseudoClassSelector::Modal),
            ":fullscreen" => Some(PseudoClassSelector::Fullscreen),
            ":picture-in-picture" => Some(PseudoClassSelector::PictureInPicture),
            ":autofill" => Some(PseudoClassSelector::Autofill),
            ":user-invalid" => Some(PseudoClassSelector::UserInvalid),
            _ => None,
        }
    }
}