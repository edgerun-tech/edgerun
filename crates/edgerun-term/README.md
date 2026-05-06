# edgerun-term

Terminal emulator for the Edgerun display stack. The primary Linux build uses `/home/ken/edgerun/crates/edgerun-compositor` directly through its in-repo wire/protocol modules and presents an SHM buffer, avoiding the old `winit`/`pixels` X11/Wayland client path.

## Run

Start `edgerun-compositor` first, then run:

```bash
cargo run -p edgerun-term-compositor --bin edgerun-term-compositor
```

The compositor socket defaults to `/tmp/edgerun-wayland-0`. Override it with `EDGERUN_COMPOSITOR_SOCKET=/path/to/socket`.

The legacy desktop backend remains available as:

```bash
cargo run -p edgerun-term-native --bin edgerun-term
```

## Install (overwrites existing `edgerun-term` in your `$PATH`)

```bash
make install          # builds edgerun-term-compositor, installs as ~/.local/bin/edgerun-term
# or choose a prefix:
PREFIX=/usr/local make install
```

## What it does

- Compositor backend: single PTY tab, CPU text rendering into compositor SHM, keyboard input through `edgerun-compositor` keymap handling.
- Native desktop backend: GPU renderer by default; set `TERM_GPU=0` to fall back to CPU. Optional shaping/ligatures via `TERM_SHAPING=1` (off by default).
- Tabs: `Super+T` opens a tab, `Alt+T` new tab, `Alt+Q` close tab, `Alt+1-5` or `Ctrl+Tab` / `Ctrl+Shift+Tab` to cycle.
- Suggestion palette: press `Ctrl+Space` or bare `` ` ``. It ranks shell history, learned acceptances (`~/.config/term/autocomplete.json`), clipboard snippets, curated commands, recent working directories, and inline path completions. ↑/↓/PgUp/PgDn or scroll to move; Enter runs; Tab/→ inserts without running; Esc closes.
- Copy/paste: drag to select (auto-copies on release), `Ctrl+Shift+C` copies, `Ctrl+Shift+V` pastes, middle-click pastes, right-click opens a copy/paste context menu. Bracketed paste is honored.
- Help overlays: `F1` toggles the mini help bar, `F2` opens a cheatsheet, `F4` opens Settings (download Noto Color Emoji or Nerd Font Symbols into `~/.local/share/fonts/term-emoji/`; font cache refreshes after download).
- Terminal behavior: 256-color + truecolor SGR (including colon form), bold/italic/underline, double-width glyphs, alt screen with scrollback preserved (10k-line cap), cursor shape/selection rendering, window resize => PTY resize.
- Fonts: embedded Source Code Pro; also probes common system emoji/symbol fonts and applies them per-glyph when available.

## How it compares to kitty

- Overlap: GPU text rendering, tabs, truecolor, fallback fonts for emoji/icons, copy/paste conveniences, context-aware history suggestions.
- Missing vs kitty: no window splits/tiling, no kitty remote-control/kittens/graphics protocol, no config file for remapping keys/themes (only a few env toggles), no built-in image display/icat, no scrollback search, and fewer VT extensions (focus/kitty keyboard protocol, hyperlinks, OSC 133/8, etc.).

Emoji/icons: for best coverage install Noto Color Emoji, Noto Sans Symbols 2, Nerd Font Symbols, or DejaVu Sans; `F4` can fetch the emoji/symbol packs for you.
