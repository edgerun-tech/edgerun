# term

GPU-accelerated terminal emulator built with winit + pixels + portable-pty + vte. It launches your shell in a PTY, renders text with an embedded Source Code Pro font, and layers on a few quality-of-life tools (tabs, autocomplete palette, cheatsheet, context menu).

## Run

```bash
cargo run
```

## Install (overwrites existing `term` in your `$PATH`)

```bash
make install          # defaults to ~/.local/bin/term
# or choose a prefix:
PREFIX=/usr/local make install
```

## What it does

- GPU renderer by default; set `TERM_GPU=0` to fall back to CPU. Optional shaping/ligatures via `TERM_SHAPING=1` (off by default).
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
