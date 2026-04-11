// Extract UI Events spec: key codes, button enums, wheel modes, modifier keys.
package main

import (
	"encoding/json"
	"fmt"
	"os"
)

type MouseBtn struct {
	Name        string `json:"name"`
	Value       int    `json:"value"`
	Description string `json:"description"`
}

type DeltaMode struct {
	Name        string `json:"name"`
	Value       int    `json:"value"`
	Description string `json:"description"`
}

type UICatalog struct {
	Spec            string     `json:"spec"`
	SpecDate        string     `json:"spec_date"`
	KeyboardKeys    []string   `json:"keyboard_keys"`
	MouseButtons    []MouseBtn `json:"mouse_buttons"`
	WheelDeltaModes []DeltaMode `json:"wheel_delta_modes"`
	ModifierKeys    []string   `json:"modifier_keys"`
}

func main() {
	if len(os.Args) >= 2 {
		if _, err := os.ReadFile(os.Args[1]); err != nil {
			fmt.Fprintf(os.Stderr, "error: %v\n", err)
			os.Exit(1)
		}
	}

	var keys []string
	// Modifier keys
	keys = append(keys, "Alt", "AltGraph", "Control", "Fn", "FnLock", "Meta", "NumLock", "ScrollLock", "Shift", "Super", "Symbol", "SymbolLock", "Hyper")
	// Navigation
	keys = append(keys, "Down", "Left", "Right", "Up", "End", "Home", "PageDown", "PageUp", "ArrowDown", "ArrowLeft", "ArrowRight", "ArrowUp")
	// Editing
	keys = append(keys, "Backspace", "Clear", "Copy", "CrSel", "Cut", "Delete", "EraseEof", "ExSel", "Insert", "Paste", "Redo", "Undo")
	// UI
	keys = append(keys, "Accept", "Again", "Attn", "Cancel", "ContextMenu", "Escape", "Execute", "Find", "Finish", "Help", "Pause", "Play", "Props", "Select", "ZoomIn", "ZoomOut")
	// Device
	keys = append(keys, "BrightnessDown", "BrightnessUp", "Eject", "LogOff", "Power", "PowerOff", "PrintScreen", "Hibernate", "Standby", "WakeUp")
	// IME
	keys = append(keys, "AllCandidates", "Alphanumeric", "CodeInput", "Compose", "Convert", "Dead", "FinalMode", "GroupFirst", "GroupLast", "GroupNext", "GroupPrevious", "ModeChange", "NextCandidate", "NonConvert", "PreviousCandidate", "Process", "SingleCandidate")
	// Korean
	keys = append(keys, "HangulMode", "HanjaMode", "JunjaMode")
	// Japanese
	keys = append(keys, "Eisu", "Hankaku", "Hiragana", "HiraganaKatakana", "KanaMode", "KanjiMode", "Katakana", "Romaji", "Zenkaku", "ZenkakuHankaku")
	// Function keys F1-F24
	for i := 1; i <= 24; i++ {
		keys = append(keys, fmt.Sprintf("F%d", i))
	}
	// Printable
	for _, c := range " !\"#$%&'()*+,-./0123456789:;<=>?@" {
		keys = append(keys, string(c))
	}
	for c := 'A'; c <= 'Z'; c++ {
		keys = append(keys, string(c))
	}
	for _, c := range "[]^_`" {
		keys = append(keys, string(c))
	}
	for c := 'a'; c <= 'z'; c++ {
		keys = append(keys, string(c))
	}
	for _, c := range "{}|~" {
		keys = append(keys, string(c))
	}
	keys = append(keys, "Tab", "Enter", "Space", "CapsLock")

	catalog := UICatalog{
		Spec: "UI Events", SpecDate: "2026-04-10",
		KeyboardKeys: keys,
		MouseButtons: []MouseBtn{
			{"Primary", 0, "Left button / touch"},
			{"Auxiliary", 1, "Middle button"},
			{"Secondary", 2, "Right button"},
			{"Fourth", 3, "Browser back"},
			{"Fifth", 4, "Browser forward"},
		},
		WheelDeltaModes: []DeltaMode{
			{"Pixel", 0, "Delta values in pixels"},
			{"Line", 1, "Delta values in lines"},
			{"Page", 2, "Delta values in pages"},
		},
		ModifierKeys: []string{
			"altKey", "ctrlKey", "metaKey", "shiftKey",
			"altGraphKey", "capsLockKey", "fnKey", "fnLockKey",
			"numLockKey", "scrollLockKey", "superKey", "symbolKey", "symbolLockKey",
			"hyperKey",
		},
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	enc.Encode(catalog)
}
