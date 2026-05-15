# kern

A terminal-native typing test inspired by Monkeytype — fast, minimal, and offline-first.

## Install

```bash
cargo install --path .
```

Or build and run directly:

```bash
cargo run
```

## Usage

```bash
kern                       # 30-word test (default)
kern --time 15             # timed mode: 15s, 30s, or 60s
kern --time 30
kern --time 60
kern --words 10            # word count mode: 10, 25, or 50 words
kern --words 25
kern --words 50
kern --time 60 --punctuation   # enable punctuation
kern --words 25 --numbers      # enable numbers
```

### Keybindings

| Key         | Action          |
|-------------|-----------------|
| `Tab`       | Restart test    |
| `Enter`     | Confirm restart |
| `Backspace` | Delete          |
| `Esc`       | Quit            |

### Results

After each test, kern displays WPM, raw WPM, accuracy, and a character breakdown (correct / incorrect / extra / missed). Stats are saved locally to `~/.config/kern/stats.json`.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT — see [LICENSE](LICENSE).
