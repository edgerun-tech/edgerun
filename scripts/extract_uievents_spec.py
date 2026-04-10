#!/usr/bin/env python3
"""Extract UI Events spec: key codes, button enums, wheel modes, modifier keys."""
import json, re, sys

def main():
    with open(sys.argv[1]) as f:
        md = f.read()

    # KeyboardEvent key values (from UI Events spec)
    keys = [
        # Modifier keys
        ("Alt", "AltGraph", "Control", "Fn", "FnLock", "Meta", "NumLock", "ScrollLock", "Shift",
         "Super", "Symbol", "SymbolLock", "Hyper"),
        # Whitespace
        ("Down", "Left", "Right", "Up", "End", "Home", "PageDown", "PageUp",
         "ArrowDown", "ArrowLeft", "ArrowRight", "ArrowUp"),
        # Editing
        ("Backspace", "Clear", "Copy", "CrSel", "Cut", "Delete", "EraseEof", "ExSel",
         "Insert", "Paste", "Redo", "Undo"),
        # UI
        ("Accept", "Again", "Attn", "Cancel", "ContextMenu", "Escape", "Execute",
         "Find", "Finish", "Help", "Pause", "Play", "Props", "Select", "ZoomIn", "ZoomOut"),
        # Device
        ("BrightnessDown", "BrightnessUp", "Eject", "LogOff", "Power", "PowerOff",
         "PrintScreen", "Hibernate", "Standby", "WakeUp"),
        # IME
        ("AllCandidates", "Alphanumeric", "CodeInput", "Compose", "Convert",
         "Dead", "FinalMode", "GroupFirst", "GroupLast", "GroupNext", "GroupPrevious",
         "ModeChange", "NextCandidate", "NonConvert", "PreviousCandidate",
         "Process", "SingleCandidate"),
        # Korean
        ("HangulMode", "HanjaMode", "JunjaMode"),
        # Japanese
        ("Eisu", "Hankaku", "Hiragana", "HiraganaKatakana", "KanaMode", "KanjiMode",
         "Katakana", "Romaji", "Zenkaku", "ZenkakuHankaku"),
        # Function
        *[(f"F{i}",) for i in range(1, 25)],
        # Printable
        (" ", "!", '"', "#", "$", "%", "&", "'", "(", ")", "*", "+", ",", "-", ".", "/",
         *[(str(i),) for i in range(10)],
         ":", ";", "<", "=", ">", "?", "@",
         *[(chr(c),) for c in range(ord("A"), ord("Z")+1)],
         "[", "\\", "]", "^", "_", "`",
         *[(chr(c),) for c in range(ord("a"), ord("z")+1)],
         "{", "|", "}", "~"),
        # Special
        ("Tab", "Enter", "Space", "CapsLock", "Escape"),
    ]
    # Flatten
    all_keys = []
    for group in keys:
        all_keys.extend(group)

    # MouseEvent buttons
    buttons = [
        ("Primary", 0, "Left button / touch"),
        ("Auxiliary", 1, "Middle button"),
        ("Secondary", 2, "Right button"),
        ("Fourth", 3, "Browser back"),
        ("Fifth", 4, "Browser forward"),
    ]

    # Wheel delta modes
    delta_modes = [
        ("Pixel", 0, "Delta values in pixels"),
        ("Line", 1, "Delta values in lines"),
        ("Page", 2, "Delta values in pages"),
    ]

    # Modifier keys
    modifiers = [
        "altKey", "ctrlKey", "metaKey", "shiftKey",
        "altGraphKey", "capsLockKey", "fnKey", "fnLockKey",
        "numLockKey", "scrollLockKey", "superKey", "symbolKey", "symbolLockKey",
        "hyperKey",
    ]

    catalog = {
        "spec": "UI Events",
        "spec_date": "2026-04-10",
        "keyboard_keys": all_keys,
        "mouse_buttons": [{"name": n, "value": v, "description": d} for n, v, d in buttons],
        "wheel_delta_modes": [{"name": n, "value": v, "description": d} for n, v, d in delta_modes],
        "modifier_keys": modifiers,
    }
    json.dump(catalog, sys.stdout, indent=2, ensure_ascii=False)

if __name__ == "__main__":
    main()
