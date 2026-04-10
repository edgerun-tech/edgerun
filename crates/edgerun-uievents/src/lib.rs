//! UI Events types — from W3C UI Events specification.
//! DO NOT EDIT. Regenerate with: scripts/generate_web_platform.py
#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::{string::String, vec::Vec};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FunctionalKey {
    /// Alt
    Alt,
    /// AltGraph
    Altgraph,
    /// Control
    Control,
    /// Fn
    Fn,
    /// FnLock
    Fnlock,
    /// Meta
    Meta,
    /// NumLock
    Numlock,
    /// ScrollLock
    Scrolllock,
    /// Shift
    Shift,
    /// Super
    Super,
    /// Symbol
    Symbol,
    /// SymbolLock
    Symbollock,
    /// Hyper
    Hyper,
    /// Down
    Down,
    /// Left
    Left,
    /// Right
    Right,
    /// Up
    Up,
    /// End
    End,
    /// Home
    Home,
    /// PageDown
    Pagedown,
    /// PageUp
    Pageup,
    /// ArrowDown
    Arrowdown,
    /// ArrowLeft
    Arrowleft,
    /// ArrowRight
    Arrowright,
    /// ArrowUp
    Arrowup,
    /// Backspace
    Backspace,
    /// Clear
    Clear,
    /// Copy
    Copy,
    /// CrSel
    Crsel,
    /// Cut
    Cut,
    /// Delete
    Delete,
    /// EraseEof
    Eraseeof,
    /// ExSel
    Exsel,
    /// Insert
    Insert,
    /// Paste
    Paste,
    /// Redo
    Redo,
    /// Undo
    Undo,
    /// Accept
    Accept,
    /// Again
    Again,
    /// Attn
    Attn,
    /// Cancel
    Cancel,
    /// ContextMenu
    Contextmenu,
    /// Escape
    Escape,
    /// Execute
    Execute,
    /// Find
    Find,
    /// Finish
    Finish,
    /// Help
    Help,
    /// Pause
    Pause,
    /// Play
    Play,
    /// Props
    Props,
    /// Select
    Select,
    /// ZoomIn
    Zoomin,
    /// ZoomOut
    Zoomout,
    /// BrightnessDown
    Brightnessdown,
    /// BrightnessUp
    Brightnessup,
    /// Eject
    Eject,
    /// LogOff
    Logoff,
    /// Power
    Power,
    /// PowerOff
    Poweroff,
    /// PrintScreen
    Printscreen,
    /// Hibernate
    Hibernate,
    /// Standby
    Standby,
    /// WakeUp
    Wakeup,
    /// AllCandidates
    Allcandidates,
    /// Alphanumeric
    Alphanumeric,
    /// CodeInput
    Codeinput,
    /// Compose
    Compose,
    /// Convert
    Convert,
    /// Dead
    Dead,
    /// FinalMode
    Finalmode,
    /// GroupFirst
    Groupfirst,
    /// GroupLast
    Grouplast,
    /// GroupNext
    Groupnext,
    /// GroupPrevious
    Groupprevious,
    /// ModeChange
    Modechange,
    /// NextCandidate
    Nextcandidate,
    /// NonConvert
    Nonconvert,
    /// PreviousCandidate
    Previouscandidate,
    /// Process
    Process,
    /// SingleCandidate
    Singlecandidate,
    /// HangulMode
    Hangulmode,
    /// HanjaMode
    Hanjamode,
    /// JunjaMode
    Junjamode,
    /// Eisu
    Eisu,
    /// Hankaku
    Hankaku,
    /// Hiragana
    Hiragana,
    /// HiraganaKatakana
    Hiraganakatakana,
    /// KanaMode
    Kanamode,
    /// KanjiMode
    Kanjimode,
    /// Katakana
    Katakana,
    /// Romaji
    Romaji,
    /// Zenkaku
    Zenkaku,
    /// ZenkakuHankaku
    Zenkakuhankaku,
    /// F1
    F1,
    /// F2
    F2,
    /// F3
    F3,
    /// F4
    F4,
    /// F5
    F5,
    /// F6
    F6,
    /// F7
    F7,
    /// F8
    F8,
    /// F9
    F9,
    /// F10
    F10,
    /// F11
    F11,
    /// F12
    F12,
    /// F13
    F13,
    /// F14
    F14,
    /// F15
    F15,
    /// F16
    F16,
    /// F17
    F17,
    /// F18
    F18,
    /// F19
    F19,
    /// F20
    F20,
    /// F21
    F21,
    /// F22
    F22,
    /// F23
    F23,
    /// F24
    F24,
    /// Tab
    Tab,
    /// Enter
    Enter,
    /// Space
    Space,
    /// CapsLock
    Capslock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// 0: Left button / touch
    Primary = 0,
    /// 1: Middle button
    Auxiliary = 1,
    /// 2: Right button
    Secondary = 2,
    /// 3: Browser back
    Fourth = 3,
    /// 4: Browser forward
    Fifth = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaMode {
    /// 0: Delta values in pixels
    Pixel = 0,
    /// 1: Delta values in lines
    Line = 1,
    /// 2: Delta values in pages
    Page = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModifierKey {
    Alt,
    Ctrl,
    Meta,
    Shift,
    Altgraph,
    Capslock,
    Fn,
    Fnlock,
    Numlock,
    Scrolllock,
    Super,
    Symbol,
    Symbollock,
    Hyper,
}
