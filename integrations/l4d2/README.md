# L4D2 Interactive — direct RCON integration

Drive a Left 4 Dead 2 dedicated server from TikTools: test the
connection, spawn common hordes, spawn special infected, and drop items.
No game modifications beyond a small SourceMod adapter; no relay
services, no `sv_cheats`.

> **Status: verified live on a real SRCDS server (2026-09-28).**
> All gates below passed against a Linux dedicated server
> (`c1m1_hotel`, SourceMod 1.12.0-git7253, MetaMod 1.12.0-git1219,
> Left4DHooks 1.168, `spawn_infected_nolimit`) with `sv_cheats 0`
> throughout. The one boundary the evidence does not cross: no human
> client joined during verification (the local L4D2 client segfaults on
> this machine), so the server hibernated and director-positioned
> (non-fallback) special spawns plus visual in-game observation remain
> unconfirmed. See Gate 3 notes.

## Contents

- `adapter/tiktools_l4d2.sp` — SourceMod adapter source (console-safe
  `tt_*` commands, machine-readable `TT_OK`/`TT_ERR` protocol).
- `README.md` — this guide.

The TikTools side lives at `plugins/l4d2/` (first-party process
package): `plugin.json` manifest + `backend/` Rust crate
(`tiktools-l4d2` binary, workspace member).

## 1. Architecture (direct RCON only)

```text
TikTok event → TikTools behavior → l4d2.interactive action
    → backend opens TCP to game:27015 → RCON auth → tt_* command
    → adapter validates → spawns → prints TT_OK|... / TT_ERR|...|...
    → backend classifies → honest ActionResult
```

Why not reuse All4Dead directly: every upstream spawn handler rejects
console callers (`if (client == 0) … "cannot be used by server"` in
`all4dead2.sp`), and RCON executes as the console. The adapter exists
to provide console-safe commands using the same battle-tested spawn
patterns (entity spawn for commons/items, `NoLimit_CreateInfected` +
Left4DHooks positioning for specials).

One RCON connection per action (connect → auth → one command → close).
No retries after a timeout: the game may have acted despite the lost
reply, and a rerun could double-spawn. Transport failures are errors;
game rejections (`TT_ERR`) are result summaries; a missing confirmation
is an error, never a success.

## 2. Game-side install

Prerequisites on the game host: SRCDS L4D2 server, SourceMod 1.11+,
Left4DHooks, `spawn_infected_nolimit`. The exact set verified live:
SourceMod 1.12.0-git7253, MetaMod:Source 1.12.0-git1219, Left4DHooks
1.168 + its `left4dhooks.l4d2.cfg` gamedata (required — without it
Left4DHooks fails to load), `spawn_infected_nolimit`:

1. Install SourceMod + Left4DHooks
   (`https://github.com/SilvDev/Left4DHooks`) +
   `spawn_infected_nolimit`
   (`https://github.com/fbef0102/L4D1_2-Plugins/tree/master/spawn_infected_nolimit`)
   per their own READMEs. Verify with `sm plugins list` (both present,
   no errors).
2. Compile the adapter:
   ```sh
   spcomp tiktools_l4d2.sp \
     -i addons/sourcemod/scripting/include \
     -o addons/sourcemod/plugins/tiktools_l4d2.smx
   ```
   The compile needs `left4dhooks.inc` and
   `spawn_infected_nolimit.inc` on the include path.
3. Copy `tiktools_l4d2.smx` to the server's
   `addons/sourcemod/plugins/`, then `sm plugins load tiktools_l4d2`
   (or restart). Confirm `sm plugins info tiktools_l4d2` loads without
   errors.
4. Enable RCON in `server.cfg` (use a strong password; RCON is **not
   encrypted** — prefer `127.0.0.1` when TikTools runs on the same
   machine, otherwise firewall the port):
   ```cfg
   rcon_password "a-strong-unique-password"
   ```
   No `sv_cheats`, no other cvar changes required.
5. Tune the adapter ConVars (created on first load, persisted to
   `cfg/sourcemod/tiktools_l4d2.cfg`):

   | ConVar | Default | Meaning |
   |---|---|---|
   | `tt_max_common` | 10 (1..25) | Max commons per call |
   | `tt_max_special` | 8 (1..8) | Max specials per call |
   | `tt_max_item` | 5 (1..5) | Max items per call |
   | `tt_tank_cooldown_s` | 120 (0..3600) | Seconds between tanks |
   | `tt_cmd_cooldown_ms` | 500 (0..60000) | Global gap between spawn commands (`tt_status` exempt) |
   | `tt_max_entities` | 1900 (500..2000) | Refuse spawns above this entity count |
   | `tt_debug` | 0 | Extra server-log lines per call |

## 3. TikTools-side install

1. Stage the first-party package (builds `tiktools-l4d2` and stages it
   into `.dev-plugins/l4d2.interactive/`):
   ```sh
   bun run prepare:dev-plugins
   ```
2. Open the plugin settings and save **RCON host** (default
   `127.0.0.1`), **RCON port** (default `27015`), and the
   **RCON password** (secret field, never logged). Timeouts and the
   per-minute rate limit live under Advanced.
3. Press **Probar conexión** in the same Ajustes tab for a one-click
   live check (no automation needed; uses the saved settings — save
   first if you edited them). Or run **Test connection** as a *live*
   action (dry-run test consoles never call plugin backends by
   design). Expect a summary like
   `Connected: map c1m1_hotel (4 survivors, 12 commons, 812/2048 entities).`

> Import note: templates/profiles referencing `l4d2.*` actions refuse
> with a clear error unless this plugin is installed and enabled.

## 3b. Same-machine clients: prefer the LAN IP over loopback

Measured on Linux (2026-10-29, kernel packet log + full 2×2
source/destination matrix against `srcds_linux` bound to `0.0.0.0:27015`):

- UDP game packets with a **loopback source** (`127.0.0.0/8`) reach the
  server socket but are **never answered**; the same packets with a **LAN
  source** get a challenge reply within the same second.
- TCP/RCON is unaffected: `127.0.1.1:27015` authenticates fine (the RCON
  listener binds that address; `127.0.0.1:27015` and the LAN IP refuse
  TCP — point TikTools at `127.0.1.1` when both run on this box).
- Firewall tables (`filter`/`nat`/`mangle`/`raw`) are all ACCEPT and the
  engine ban lists are empty — the drop is the dedicated-server binary
  ignoring loopback-sourced game traffic, not your network.

Practical rule: **game client connects via the server's LAN IP**
(`connect 192.168.18.130:27015`), **TikTools RCON via `127.0.1.1`**.
Give the server box a DHCP reservation so the LAN IP is stable.
On Windows the loopback path is untested — if `connect 127.0.0.1`
fails there the same way, use the LAN IP as well.

## 4. Protocol reference

Engine commands (server console / RCON only; players cannot run them):

| Command | Args | Success |
|---|---|---|
| `tt_status` | `<reqid>` | `TT_OK\|<reqid>\|adapter=…\|map=…\|survivors=N\|specials=N\|commons=N\|witches=N\|entities=N\|entities_max=N\|positions=0/1\|flow=0/1\|nolimit=0/1\|anchor=N\|gametime=T\|engtime=T` |
| `tt_spawn_common` | `<reqid> <count>` | `TT_OK\|<reqid>\|spawned=N\|requested=M` |
| `tt_spawn_infected` | `<reqid> <class> <count>` | `TT_OK\|<reqid>\|class=…\|spawned=N\|requested=M\|fallback=K` |
| `tt_spawn_item` | `<reqid> <item> <count>` | `TT_OK\|<reqid>\|item=…\|spawned=N\|requested=M` |

Failures: `TT_ERR|<reqid>|CODE|detail` with `CODE` in
`BAD_REQID BAD_CLASS BAD_ITEM COMMON_CAP SPECIAL_CAP ITEM_CAP
TANK_CAP TANK_COOLDOWN COOLDOWN NO_SURVIVOR NO_NATIVES ENTITY_CAP
SPAWN_FAILED`. `spawned < requested` inside `TT_OK` means a partial
spawn (entity cap hit mid-loop) — the backend surfaces the shortfall.

`tt_status` diagnostics: `anchor` is the raw
`L4D_GetHighestFlowSurvivor()` reading — only meaningful when
`flow=1`; a non-positive value means the director has no survivor
flow data yet (expected while hibernating). `gametime` vs `engtime`
across two probes reveal hibernation (frozen game time while engine
time advances — normal on an empty server).

`fallback=K` on `tt_spawn_infected` counts specials placed by the
near-survivor fallback instead of director spawn areas. The adapter
always tries `L4D_GetRandomPZSpawnPosition` (anchor, then client 0)
first; only when the director has no areas ready — empty/hibernating
server — does it trace a ground point ~160 units ahead of the anchor
survivor and spawn there. The backend appends “(fallback positioning:
director spawn areas unavailable)” to the summary whenever `fallback`
is non-zero, so the placement path is never silent.

Allowlists: classes `tank witch smoker boomer hunter spitter jockey
charger`; items `first_aid_kit pain_pills adrenaline pipe_bomb
molotov defibrillator`. Anything else is rejected before any game
call. Tanks are always count 1.

## 5. Verification gates

| Gate | What | How | Status |
|---|---|---|---|
| 0 | Plugin SDK/protocol/manifest inspected; plan written | This repo + upstream sources (see §7) | **PASS** |
| 1 | `spcomp` compiles the adapter warning-free | SourcePawn Compiler 1.12.0.7253, `-i` server include + Left4DHooks/nolimit incs | **PASS** (19516-byte `.smx`, zero warnings) |
| 2 | `tt_status` over real RCON returns `TT_OK` with sane map/counts | `l4d2.test_connection` via real backend → `Connected: map c1m1_hotel (1 survivors, 0 commons, 1511/2048 entities).` | **PASS** |
| 3 | Each allowlisted class/item spawns once and confirms | One action per class/item through the real backend; counts delta-checked via `tt_status` | **PASS with boundary** (see notes) |
| 4 | Caps/cooldowns enforced | Over-cap calls → `TT_ERR`; tank twice → `TANK_COOLDOWN`; `sv_cheats` stayed `0` | **PASS** (7 refusal codes observed live) |
| 5 | Backend unit/integration tests incl. mock RCON | `cargo test -p tiktools-l4d2` (22 tests) + manifest contract test | **PASS** |
| 6 | Live end-to-end: gift event → confirmed spawn | Gift-shaped event through framed stdio into the real backend binary → `Spawn common horde: game confirmed 3/3.` + `"ok": true` | **PASS** |
| 7 | Soak: no errors/leaks/cheat requirements over a session | 30-min live session, `sv_cheats 0` throughout | **PASS** (30/30 actions ok, 6/6 cheat checks clean) |

Gate 3 evidence (all through the real `tiktools-l4d2` binary over
RCON to `c1m1_hotel`, `sv_cheats 0`):

- hunter, smoker, boomer, spitter, jockey, charger: `game confirmed
  1/1` each; `specials` 0 → 6.
- witch: `game confirmed 1/1`; `witches` 0 → 1. The `result > 0`
  check holds for the witch entity index — no tightening needed.
- tank: `game confirmed 1/1`; `specials` 6 → 7.
- All 6 items (`first_aid_kit`, `pain_pills`, `adrenaline`,
  `pipe_bomb`, `molotov`, `defibrillator`): `game confirmed 1/1`
  each.
- Commons: `game confirmed 5/5`; `commons` 0 → 5.

Gate 3 boundary (read before trusting blindly): every special above
used `fallback=1` — the test server hibernated (proven: `gametime`
frozen at 1749 across 6 s while `engtime` advanced 2691 → 2697, and
`anchor=-1` with `flow=1`), because no human client ever joined
(the local L4D2 client segfaults during load on this machine, before
and after a Steam validation + repair; the install itself is current
and protocol-compatible — client and server both report
`NetworkVersion=2.1.0.0`, server build Jun 30 2026 — so a join from
a working machine should succeed). Fake-client bots (`sb_add`)
do not unhibernate the server. So the director-positioned path
(`L4D_GetRandomPZSpawnPosition` success, `fallback=0`) and visual
in-game observation are still unconfirmed; they need one session
with a real player on the server. The adapter always tries the
director path first, so live servers with players will use it —
but that claim currently rests on the upstream All4Dead usage, not
on our own live run.

Gate 4 evidence (raw RCON, same server):

- `tt_spawn_common ev_cap1 25` →
  `TT_ERR|ev_cap1|COMMON_CAP|max=10`
- `tt_spawn_infected ev_cap2 hunter 9` →
  `TT_ERR|ev_cap2|SPECIAL_CAP|max=8`
- `tt_spawn_infected ev_tank2 tank 1` (second tank within 120 s) →
  `TT_ERR|ev_tank2|TANK_COOLDOWN|retry_in_s=101`
- `tt_max_entities` lowered to the live count, then
  `tt_spawn_common ev_cap3 1` →
  `TT_ERR|ev_cap3|ENTITY_CAP|entities=1511 max=1511` (cap restored
  to 1900 afterwards)
- `BAD_CLASS` (unknown class), `TANK_CAP` (tank count ≠ 1), and
  `COOLDOWN retry_in_ms=…` (sub-500 ms repeat) all observed on
  earlier probes, plus `SPAWN_FAILED pos_ok=0` before the fallback
  existed. Not observed live (covered by backend unit tests and
  code review instead): `NO_SURVIVOR`, `NO_NATIVES`, `BAD_REQID`,
  `BAD_ITEM`, `ITEM_CAP`.

Gate 7 evidence: 30 iterations × 60 s (00:12:30 → 00:41:35 UTC),
each driving the real `tiktools-l4d2` binary over its framed stdio
protocol (fresh process per action, as the host worker does),
cycling `l4d2.test_connection` → `l4d2.spawn_common` (count 1) →
`l4d2.spawn_item` (`pain_pills`, count 1). Result: `SOAK_DONE
ok=30 err=0 cheats_ok=6 cheats_bad=0` — every action returned
`"ok": true` with a game-confirmed summary, and `sv_cheats` read
back `0` on all 6 periodic checks. Final `tt_status`: 2 survivors,
8 specials, 14 commons, 1 witch, entities steady at 1511/2048; no
adapter errors in the server log, no entity growth beyond the
spawned backlog.

## 6. Troubleshooting

| Symptom | Cause → fix |
|---|---|
| `cannot reach the game server` | Wrong host/port or server down → check `status` on the game host, firewall, `hostip`/`port`. |
| `RCON authentication failed` | Wrong `rcon_password` → re-save settings; note RCON passes it in cleartext. |
| `RCON timed out` | Hung server or packet loss → check server FPS/`net_graph`; no auto-retry by design. |
| `no confirmation from the game` | Transport ok, no `TT_OK`/`TT_ERR` → adapter `.smx` not loaded (`sm plugins list`), or console output not captured (verify Gate 2 by hand). |
| `TT_ERR NO_NATIVES …` | Left4DHooks or `spawn_infected_nolimit` missing/broken → `sm plugins list`, fix errors, `sm plugins reload`. |
| `TT_ERR NO_SURVIVOR` | No alive survivor to anchor the spawn (lobby/preround) → expected outside live rounds. |
| `…(fallback positioning…) ` in a spawn summary | Director had no spawn areas ready (empty/hibernating server — check `gametime` frozen across two `tt_status` probes) → specials spawned near a survivor instead; normal without players, disappears once a round is live. |
| Specials never use the director path even with players | `L4D_GetRandomPZSpawnPosition` failing repeatedly → check Left4DHooks errors in `sm plugins list` / error logs; stale gamedata after a game update is the usual cause. |
| `TT_ERR TANK_COOLDOWN retry_in_s=N` | Working as designed → wait or lower `tt_tank_cooldown_s`. |
| `L4D2 rate limit reached` | Plugin-side per-minute cap → wait, or raise Advanced → Max actions per minute. |
| Import refuses `l4d2.*` actions | Plugin not installed/enabled → stage it (§3) and retry. |

Rollback: unload `sm plugins unload tiktools_l4d2` (game side) and
disable `l4d2.interactive` in TikTools (automation side). No host, DB,
or profile changes were made by this integration.

## 7. References (verified from source)

- Upstream console rejection + spawn patterns:
  `fbef0102/L4D1_2-Plugins … all4dead2.sp`
  (`client == 0` guards; `CreateEntityByName("infected")` + think-tick
  + `DispatchSpawn`/`ActivateEntity`/`TeleportEntity`;
  `L4D_GetHighestFlowSurvivor` +
  `L4D_GetRandomPZSpawnPosition(survivor, class, tries, pos)` with
  smoker=1 … charger=6, witch=7, tank=8).
- `NoLimit_CreateInfected(const char[] zomb, const float vecPos[3],
  const float vecAng[3], …)` returning client/entity index or `-1`:
  `spawn_infected_nolimit.inc`.
- `L4D_GetRandomPZSpawnPosition(int client, …)` accepting client `0`:
  `SilvDev/Left4DHooks … left4dhooks.inc` (both includes mark natives
  optional unless `REQUIRE_PLUGIN` is defined).
- Source RCON framing: Valve Developer Community (little-endian
  size/id/type/body + double null; auth id `-1` = reject; 4096-byte
  split responses).
