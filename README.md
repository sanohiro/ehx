# hx — Hex Editor with Emacs Keybindings

A terminal hex editor for those who think in `C-x C-s`.

This package includes two tools:
- **hx** — Interactive TUI hex editor
- **bx** — CLI binary tool for pipes

> If xxd feels primitive and GUI hex editors feel heavy,
> hx is the middle ground.

[日本語](README.ja.md)

---

## Why hx?

Most hex editors use vi-style or custom keybindings. **hx** is for Emacs users who don't want to context-switch their muscle memory just to edit a binary file.

- Same navigation: `C-f`, `C-b`, `C-n`, `C-p`
- Same file operations: `C-x C-s`, `C-x C-c`
- Same selection: `C-SPC`, `M-w`, `C-w`, `C-y`
- Same search: `C-s`, `C-r`

No relearning. No mode switching confusion.
Just edit bytes like you edit text.

---

## Key Characteristics

- **Emacs keybindings**
  Familiar cursor movement and editing commands.

- **Dual-pane view**
  HEX and ASCII side by side. Tab to switch input focus.

- **Multi-encoding support**
  UTF-8, UTF-16, Shift-JIS, EUC-JP, and more.

- **Clipboard integration**
  System clipboard + OSC 52 (works over SSH).

- **Full-width character support**
  Input `０-９`, `Ａ-Ｆ` as hex digits. Japanese IME friendly.

---

## Install

### Build from source

```bash
# Requires Rust 1.85+
git clone https://github.com/sanohiro/hx
cd hx
cargo build --release
cp ./target/release/hx ~/.local/bin/
```

---

## Quick Start

```bash
hx file.bin          # Open a file
hx                   # Start with empty buffer
cat file.bin | hx    # Read from stdin
echo -n "Hello" | hx # Pipe data
```

Save and quit: `C-x C-s` → `C-x C-c`

---

## Keybindings

hx uses Emacs-style keybindings. `C-` means Ctrl, `M-` means Alt/Option.

### Cursor Movement

| Key | Action |
|-----|--------|
| `C-f` / `C-b` / `C-n` / `C-p` | Move cursor |
| `C-a` / `C-e` | Beginning / end of row |
| `C-v` / `M-v` | Page down / up |
| `M-<` / `M->` | Beginning / end of buffer |

### Editing

| Key | Action |
|-----|--------|
| `C-d` / `Backspace` | Delete byte |
| `Tab` | Toggle HEX / ASCII input |
| `Insert` | Toggle Overwrite / Insert mode |
| `C-u` / `C-/` | Undo / Redo |

### Selection & Clipboard

| Key | Action |
|-----|--------|
| `C-SPC` | Start selection |
| `M-w` / `C-w` / `C-y` | Copy / Cut / Paste |
| `C-g` | Cancel |

### Search & Replace

| Key | Action |
|-----|--------|
| `C-s` / `C-r` | Search forward / backward |
| `M-%` | Query replace |

During query replace: `y` (replace), `n` (skip), `!` (replace all), `q` (quit)

Search/replace accepts text or HEX patterns:
- `hello` — ASCII text
- `48 65 6C 6C 6F` — Spaced HEX
- `48656C6C6F` — Continuous HEX

### File Operations

| Key | Action |
|-----|--------|
| `C-x C-s` | Save |
| `C-x C-w` | Save as |
| `C-x C-f` | Open file |
| `C-x k` | Close buffer (new empty buffer) |
| `C-x C-c` | Quit |

Unsaved changes prompt: `y` (save & continue), `n` (discard), `c` (cancel)

### Navigation

| Key | Action |
|-----|--------|
| `M-g` | Goto address (hex: `0x100`, `100h`, or decimal) |

### Commands (M-x)

| Command | Action |
|---------|--------|
| `fill` / `f` | Fill selection with byte (e.g., `00`, `FF`) |
| `insert` / `i` | Insert N bytes at cursor (e.g., `16 00`, `0x10 FF`) |
| `goto` / `g` | Jump to address |
| `save` / `s` | Save file |
| `quit` / `q` | Quit |
| `help` / `?` | Show command list |

### Display

| Key | Action |
|-----|--------|
| `F2` | Cycle encoding |

---

## Input Modes

### HEX Mode (default)

Type hex digits (`0-9`, `A-F`) to edit bytes directly.
Full-width characters (`０-９`, `Ａ-Ｆ`) are automatically converted.

### ASCII Mode

Press `Tab` to switch. Type any character — it will be encoded using the current encoding:
- UTF-8: `あ` → `E3 81 82` (3 bytes)
- Shift-JIS: `あ` → `82 A0` (2 bytes)

---

## Paste Formats

Bracketed paste auto-detects format:
- `48 65 6C 6C 6F` — Spaced HEX
- `48656C6C6F` — Continuous HEX
- `Hello` — Raw text (as bytes)

---

## Clipboard Integration

- Uses **OSC 52** escape sequence to copy to system clipboard
- Works over SSH with iTerm2, kitty, alacritty, WezTerm
- **tmux**: Add `set -g allow-passthrough on` to your `.tmux.conf`

---

## Inspiration

- [ze](https://github.com/sanohiro/ze) — Zero-latency Emacs-like editor
- [hexyl](https://github.com/sharkdp/hexyl) — Hex viewer
- [Stirling](https://github.com/nickg/stirling) — GUI hex editor

---

## bx — Binary Tool for Pipes

Unix-style binary manipulation tool included with hx.

```bash
# Find hex pattern
echo -n "Hello" | bx find 6C6C        # Find "ll"
bx find DEADBEEF -i firmware.bin

# Extract byte range
bx slice 0x100:0x200 -i file.bin      # Extract bytes
bx slice 0:512 -i file.bin -x         # Hex dump

# Replace pattern
bx replace FF00 AA55 < in.bin > out.bin
bx replace --all 00 FF < in > out     # Replace all

# Patch at offset
bx patch 0x100=DEAD 0x200=BEEF < in > out

# File info (size, entropy)
bx info -i file.bin

# Convert hex <-> binary
echo -n "Hello" | bx conv bin2hex     # 48 65 6C 6C 6F
echo "48656C6C6F" | bx conv hex2bin   # Hello
```

---

## License

MIT

---

*"Edit binary like you edit text."*
