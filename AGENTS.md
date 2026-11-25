# Agent notes

## Git

- Never add `Co-Authored-By` (or any co-author trailer) to commits in this repository.

## Resource constraints

- This machine has **2 CPUs and 14 GiB RAM**. Never run heavy verification
  steps (frontend-lint, cargo test, vitest, contracts) in parallel background
  shells — run them strictly one at a time. Parallel runs already triggered
  systemd-oomd kills that took down the whole IDE.
- If the user runs OBS/Discord or other heavy apps, prefer the lightest
  verification that covers the change instead of the full matrix.

## Verification

- Rust: `cargo test -p tiktools-core`, `cargo test -p tiktools-plugin-api`, `cargo clippy`
- Web: `bun scripts/task.ts frontend-lint`, `bun scripts/task.ts test web <filter>`
- Contracts: `bun run check`

## Experimental Vize typecheck (upstream collaboration)

- `bun scripts/task.ts frontend-typecheck:vize` typechecks `.vue` files through
  [Vize](https://github.com/ubugeeei-prod/vize)'s content mapper on TypeScript
  7.1 beta (`tsconfig.vize.json`, aliased package `typescript-vize` — the main
  `typescript` dependency stays on the line `vue-tsc` needs).
- **`vue-tsc` remains the authoritative gate.** The Vize run is parallel and
  experimental; it must never block or replace `frontend-lint`.
- Measured on this machine: ~5 s vs ~37 s for `vue-tsc` (7x+).
- **Reporting policy — this is a collaboration with upstream, not just a tool:**
  - Every failure or divergence from `vue-tsc` must be reported: minimize a
    repro, then open an issue at https://github.com/ubugeeei-prod/vize/issues
    (or microsoft/typescript-go when the divergence is TS6-vs-TS7 behavior,
    visible because it also reproduces without any `.vue` file involved).
  - After opening the issue, evaluate whether it warrants a PR: if the fix is
    small and clearly understood, implement it upstream and link the PR from
    the issue; if not, say so in the issue and move on.
  - Never silence a divergence locally (no `// @ts-ignore`, no config
    exclusions) without linking the upstream issue in a comment.
- Version policy: `vize` and `typescript-vize` are pinned exact. Bump them
  deliberately every couple of weeks as part of the collaboration — never
  float. Before adopting a bump, run both typecheckers and diff the output.
