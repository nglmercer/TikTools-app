# L4D2 Interactive — local L4D2api integration

TikTools drives a **normal Windows Steam L4D2 local/single-player campaign** through the [L4D2api](https://github.com/nglmercer/L4D2api) VScript addon.

**No RCON, dedicated server, SourceMod, port forwarding, password, or TikTools-managed installer.**

## Install the game addon once (outside TikTools)

Download `l4d2api-installer.exe` and `tiktools-l4d2api-0.0.1.vpk` from [L4D2api v0.0.1](https://github.com/nglmercer/L4D2api/releases/tag/v0.0.1). Place both files in the same directory, then run:

```powershell
.\l4d2api-installer.exe install --vpk .\tiktools-l4d2api-0.0.1.vpk --version 0.0.1
```

If Steam discovery fails, pass `--game-dir "D:\SteamLibrary\steamapps\common\Left 4 Dead 2"`. Verify with `l4d2api-installer.exe status --json`.

TikTools **never invokes this executable** or modifies the game installation. Install/update/repair/uninstall the addon using the upstream tool only.

## Run

1. Start Left 4 Dead 2 and host a local campaign.
2. Start TikTools. Enable `l4d2.interactive` and open the plugin's settings.
3. Leave **Game directory** empty to auto-detect Steam; set it explicitly for unusual installs or multiple copies.
4. Press **Test connection** (from PR #41) to check the live game session.
5. Configure events/automations to execute `l4d2.spawn_item`, `l4d2.spawn_weapon`, or `l4d2.spawn_common`.

No game-side server needs to be started. File IPC happens under `<game-dir>/left4dead2/ems/tiktools_l4d2/`.

## Actions

| TikTools action | Status | Count |
|---|---|---|
| `l4d2.test_connection` | Live readiness + VScript acknowledgement | — |
| `l4d2.spawn_item` | Stock items | 1–5 |
| `l4d2.spawn_weapon` | Stock weapon pickups | 1–5 |
| `l4d2.spawn_common` | Experimental, capability-gated | 1–10 |
| `l4d2.spawn_infected` | Reserved ID, currently unavailable unless addon advertises support | 1–8 |

The installed addon advertises its own available capabilities. Unavailable actions fail explicitly, not by falling back to another transport.

## Error handling

- **No game detected**: supply the Steam L4D2 directory in plugin settings.
- **No heartbeat**: install the upstream addon and launch L4D2.
- **Not ready**: host a local game with an active round.
- **Capability unavailable**: current addon does not implement that action.
- **Timeout**: request execution is unconfirmed. Do not automatically retry, as it may have already executed.
- **Partial**: only the confirmed number of entities spawned; inspect the summary.

TikTools uses the pinned upstream Rust client for checksums, sequence numbers, locks, heartbeat and reply validation. It does not reimplement the wire protocol.

## Development

Build and stage the existing process plugin with `bun run prepare:dev-plugins`. Run its focused tests with `cargo test -p tiktools-l4d2`.

The user reported successful Windows local-host gameplay via L4D2api; this does not constitute automated Windows end-to-end proof of the TikTools plugin. After checkout, test the button, item/weapon/common spawning, map transition and game restart before marking the integration verified.
