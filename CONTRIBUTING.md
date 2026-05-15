# Contributing to kern

Thanks for your interest in kern. Contributions are welcome — bug fixes, features from the roadmap, tests, and documentation.

## Getting Started

1. Fork the repo and clone your fork
2. Install Rust stable: https://rustup.rs
3. Build: `cargo build`
4. Run tests: `cargo nextest run`
5. Lint: `cargo clippy -- -D warnings`
6. Format: `cargo fmt`

## Before Opening a PR

- Run `cargo fmt` and `cargo clippy -- -D warnings` — CI will reject failures
- Add tests appropriate to your change (see [Testing](#testing))
- Keep commits focused — one logical change per commit
- Use [conventional commits](https://www.conventionalcommits.org): `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, etc.

## Architecture

kern follows The Elm Architecture (TEA). Read `CLAUDE.md` for the module layout and the core constraints before touching `update.rs` or `view.rs`:

- `update` must be pure — no I/O or side effects; return a `Command` for anything effectful
- `view` must be pure — read `Model` only, never mutate state

## Testing

| Type        | Tool       | What it covers                                      |
|-------------|------------|-----------------------------------------------------|
| Unit        | `nextest`  | WPM/accuracy calculations, character classification |
| Property    | `proptest` | `update` invariants, generator output, persistence  |
| Snapshot    | `insta`    | Terminal frame rendering (UI regressions)           |
| Integration | `nextest`  | Full session flows, config toggles, persistence     |

Run snapshot review after UI changes: `cargo insta review`

## Roadmap

The MVP build order is tracked in `docs/kern_mvp.md`. If you want to work on something not yet started, open an issue first to align on scope.

## Issues and Discussions

- Bug reports: include your terminal, OS, and `kern --version` output
- Feature requests: explain the use case, not just the solution
- Breaking changes: always discuss in an issue before implementing

## Code of Conduct

Be respectful. Disagreements on technical choices are fine — personal attacks are not.
