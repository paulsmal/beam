# Repository Guidelines

## Project Structure & Module Organization
`src/main.rs` wires the Axum router, startup configuration, and HTTP surfaces. Shared behavior (stream coordination, token lifecycle helpers) lives in `src/lib.rs`. Integration coverage is under `tests/integration_test.rs`, expecting a running server through Axum test harness. Build artifacts land in `target/`; keep large assets out of version control. Add new modules as submodules under `src/`, and expose only through `lib.rs` re-exports to keep the binary lean.

## Build, Test, and Development Commands
Use `cargo check` for a fast type pass during iteration. `cargo fmt --all` and `cargo clippy --all-targets --all-features` must be clean before you push; they match CI expectations. Compile the binary with `cargo build --release` and run it locally via `cargo run`. When profiling upload/download flows, start the server and exercise the curl scripts in `README.md`.

## Coding Style & Naming Conventions
Follow Rust 2021 defaults with 4-space indentation and `snake_case` for functions, `CamelCase` for types, and `SCREAMING_SNAKE_CASE` for constants. Always run `cargo fmt` before committing, and accept its layout decisions. Keep modules focused; prefer small functions returning `impl IntoResponse` rather than large handlers. Treat tokens and filenames as sensitive: avoid logging raw values except at trace level.

## Testing Guidelines
Integration tests use Tokio and Axum helpers; extend `tests/integration_test.rs` or add new `*_test.rs` files alongside it. Name async tests with the behavior they cover (e.g., `streams_uploads_until_client_disconnects`). Run `cargo test` before every PR; consider `cargo test -- --nocapture` when debugging streaming assertions. For new streaming scenarios, add end-to-end tests that validate both uploader and downloader branches and ensure tokens expire appropriately.

## Commit & Pull Request Guidelines
Commits follow the `type: summary` pattern (`feat:`, `fix:`, `chore:`) as shown in recent history. Keep messages in present tense and scoped to a single change. Pull requests should summarize the behavior change, link any tracking issues, and call out testing performed (`cargo test`, curl walkthroughs). Attach new API responses or terminal transcripts when they clarify the streaming flow.

## Security & Configuration Tips
Tokens are single-use identifiers; store them in memory only and redact them in logs. Refresh tokens stay alive while streaming, so document any timeout adjustments in PR notes. Enable HTTPS or auth layers behind a reverse proxy when deploying beyond local demos.
