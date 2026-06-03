# ktype

[![CI](https://github.com/hansonguyen/ktype/actions/workflows/ci.yml/badge.svg)](https://github.com/hansonguyen/ktype/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/ktype.svg)](https://crates.io/crates/ktype)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A terminal-native typing test inspired by Monkeytype — fast, minimal, and offline-first.

## Features

- Time mode (15s / 30s / 60s) and words mode (10 / 25 / 50 / 100)
- Custom time (`1h30m` syntax) and custom word counts, plus infinite mode
- Punctuation (`@`) and numbers (`#`) toggles for the word bank
- Live WPM, raw WPM, accuracy, time, and a WPM-over-time chart
- Themeable colors and caret style via `~/.config/ktype/config.toml`
- Persistent stats saved to `~/.config/ktype/stats.json`
- Offline-first — no network use while typing

## Install

### From crates.io

```bash
cargo install ktype
```

### From source

```bash
git clone https://github.com/hansonguyen/ktype
cd ktype
cargo install --path .
```

## Usage

```bash
ktype
```

ktype starts in time mode. Before typing begins, `Shift+Tab` switches between
time and words mode, and `←` / `→` cycle the selected option (durations in time
mode, word counts in words mode). Cycling past the last preset lands on a
**custom** slot — press `Space` to open a prompt and enter a value: time accepts
`1h30m`-style input, words accepts a plain number. A custom value of `0` enables
**infinite mode** (no limit; press `Ctrl+E` to end). Just start typing to begin.

## Keybindings

| Key               | Action                                              |
|-------------------|-----------------------------------------------------|
| `←` / `→`         | Cycle the selected option (when idle)               |
| `Shift+Tab`       | Switch time / words mode (when idle)                |
| `Tab`             | Restart with fresh words                            |
| `Space` / `Enter` | Commit word (when typing) / open custom prompt (custom slot, idle) |
| `Backspace`       | Delete character, or step back to the previous word |
| `@`               | Toggle punctuation (when idle)                      |
| `#`               | Toggle numbers (when idle)                          |
| `Ctrl+E`          | End the current test                                |
| `Esc`             | Quit (or close the custom prompt)                   |

## Results

After each test, ktype shows:

- **WPM** — words per minute (correctly typed words only)
- **Raw WPM** — all committed words, including errors
- **Accuracy** — percentage of correct keystrokes
- **Time** — elapsed seconds
- **Test type** — mode and active word bank
- **WPM chart** — speed over the course of the test

Stats are saved automatically to `~/.config/ktype/stats.json`.

## Configuration

ktype reads `~/.config/ktype/config.toml` on startup, creating it with defaults on first run.

### Theme

Edit the `[theme]` section to customize colors. All values are `#rrggbb` hex strings.

```toml
[theme]
bg = "#323437"           # terminal background
main = "#e2b714"         # primary accent (WPM, countdown, chart line)
caret = "#e2b714"        # cursor
sub = "#646669"          # muted text (untyped chars, labels, hints)
sub_alt = "#2c2e31"      # alternate muted (reserved for future use)
text = "#d1d0c5"         # correctly typed characters and stat values
error = "#ca4754"        # incorrect characters
error_extra = "#7e2a33"  # overflow indicator (reserved for future use)
colorful_error = "#ca4754"        # richer error (reserved for future use)
colorful_error_extra = "#7e2a33"  # reserved for future use
```

The defaults match [MonkeyType's serika dark](https://monkeytype.com) palette. Restart ktype after editing the file.

### Caret

The `[caret]` section sets the cursor style. Valid values are `off`, `default`,
`block` (the default), and `underline`.

```toml
[caret]
style = "block"
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT — see [LICENSE](LICENSE).
