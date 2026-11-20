# Scripts

Every script here is TypeScript. There are no shell or PowerShell scripts in this repository.

## Runtimes

- **Bun** runs `scripts/*.ts` (the task runner `task.ts` and the packaging helpers).
- **Node 22.18 or newer** runs the Termux scripts and the Windows PE checker. Node runs
  these files directly with native type stripping, so they need no build step. Termux
  ships Node, not Bun.

## Index

| Script | Runtime | Purpose | Invoked by |
| --- | --- | --- | --- |
| `task.ts` | Bun | Task runner: build, test, lint, CI | `bun run …`, workflows |
| `check-windows-pe.ts` | Bun or Node | Confirms a Windows executable uses the GUI subsystem | `task.ts build`, `ci.yml`, `release*.yml` |
| `termux/check.ts` | Node | Toolchain check and `cargo check -p tiktools-cli` on Termux | `node scripts/termux/check.ts` |
| `termux/doctor.ts` | Node | Environment report and standalone CLI checks | `node scripts/termux/doctor.ts` |
| `termux/test.ts` | Node | Test matrix with an isolated `TIKTOOLS_HOME` | `node scripts/termux/test.ts` |
| `termux/smoke.ts` | Node | Standalone CLI smoke test: IPC, signals, stdio, sandbox | `node scripts/termux/smoke.ts` |
| `termux/common.ts` | Node | Helpers shared by the Termux scripts (not run directly) | imported by the Termux scripts |

`appimage/linuxdeploy-plugin-tiktoolswebkit.sh` stays a shell script on purpose: linuxdeploy executes it on Linux packaging hosts only, and `package-installers.ts` copies it into the packager cache.

`termux/web-smoke.mjs` is a separate JavaScript check for the local web pairing page.
