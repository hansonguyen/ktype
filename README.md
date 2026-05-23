# ktype

[![CI](https://github.com/hansonguyen/ktype/actions/workflows/ci.yml/badge.svg)](https://github.com/hansonguyen/ktype/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/ktype.svg)](https://crates.io/crates/ktype)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A terminal-native typing test inspired by Monkeytype — fast, minimal, and offline-first.

## Features

- Timed tests: 15s, 30s, or 60s (cycle with `Tab`)
- Live WPM, raw WPM, accuracy, and character breakdown
- Persistent stats saved to `~/.config/ktype/stats.json`
- Zero config, zero network — runs entirely offline

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

ktype starts a 15-second timed test immediately. Press `Tab` to cycle through duration options (15s → 30s → 60s) before typing begins.

## Keybindings

| Key               | Action                                    |
|-------------------|-------------------------------------------|
| `Tab`             | Cycle duration (when not typing) / Restart |
| `Space` / `Enter` | Commit current word                       |
| `Backspace`       | Delete last character                     |
| `Esc`             | Quit                                      |

## Results

After each test, ktype shows:

- **WPM** — words per minute (correctly typed words only)
- **Raw WPM** — all keystrokes, including errors
- **Accuracy** — percentage of correct keystrokes
- **Breakdown** — correct / incorrect / extra / missed characters

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

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT — see [LICENSE](LICENSE).
