# Agent notes

## Git

- Never add `Co-Authored-By` (or any co-author trailer) to commits in this repository.

## Verification

- Rust: `cargo test -p tiktools-core`, `cargo test -p tiktools-plugin-api`, `cargo clippy`
- Web: `bun scripts/task.ts frontend-lint`, `bun scripts/task.ts test web <filter>`
- Contracts: `bun run check`
