# Rule templates and profiles

Behavior rules (one trigger event plus its actions) can be shared as JSON
documents: single **templates** (`.tiktemplate.json`) and multi-rule
**profiles** (`.tikprofile.json`). The CLI owns the full round trip
(import, per-entry options, export); the desktop Behavior tab imports
single templates through the Rule templates gallery and loads whole
profiles with default options through the header profile selector, where
one pack stays active at a time. Both sides parse the same v1
documents (`crates/tiktools-core/src/services/templates.rs` and
`src/web/views/behavior/rule-templates.ts`) and pin their agreement with
the fixtures under `examples/templates/fixtures/`.

## Template document (v1)

```json
{
  "templateVersion": 1,
  "id": "minecraft-gift-command",
  "title": "Minecraft gift command",
  "description": "…",
  "icon": "code",
  "params": {
    "type": "object",
    "properties": {
      "giftName": { "type": "string", "title": "Gift name", "default": "Rose" },
      "commandPort": { "type": "number", "title": "CommandAPI port", "default": 8080 }
    },
    "required": ["giftName"]
  },
  "actions": [
    {
      "name": "Send Minecraft command",
      "typeId": "core.fetch",
      "enabled": true,
      "config": {
        "method": "POST",
        "url": "http://{{ params.commandHost }}:{{ params.commandPort }}/api/chat",
        "body": "{\"text\": \"{{ params.command }}\"}",
        "allowPrivateNetwork": true
      }
    }
  ],
  "event": {
    "name": "Minecraft gift",
    "trigger": "tiktok.gift",
    "filters": [{ "path": "event.data.giftName", "operator": "eq", "value": "{{ params.giftName }}" }],
    "cooldownMs": 0,
    "cooldownScope": "global",
    "runMode": "all"
  }
}
```

Rules: 1–8 actions, 0–12 filters, ids match
`[a-z0-9][a-z0-9._-]{1,63}`. `title`/`description` accept a plain string or
`{"default": "…", "i18key": "…"}`.

## Params substitution

`{{ params.* }}` resolves once, when the template is applied:

- a string holding exactly one span takes the **raw** value, so numbers and
  booleans survive for typed configs (`"delta": "{{ params.delta }}"` → `10`);
- embedded spans interpolate as text;
- `{{ event.* }}` spans always survive for per-event runtime rendering;
- unknown `{{ params.* }}` keys render as `""`.

Param layers merge outer-to-inner; later layers win:
schema defaults → profile params → profile entry params → `--param k=v`.
CLI `--param` values parse as JSON when possible (`--param port=8080` is a
number) and fall back to strings.

## Runtime globals

`{{ params.* }}` bakes a value into each record at import. For values that
change after import — an ephemeral integration port, a rotated token — use
runtime globals instead: `{{ globals.commandPort }}` renders at event time
from the host store, so one edit repoints every rule with no re-import.

```bash
tiktools globals set commandPort 46665
tiktools globals get commandPort
tiktools globals list
tiktools globals delete commandPort
```

The desktop Settings tab edits the same store, and `globals.*` rows appear
in every template field's autocomplete. Keys are identifiers (`1..=64`
chars, letters/digits/`._-`, leading letter or `_`); values render as
text (≤4096 chars, ≤128 keys). Unknown `{{ globals.* }}` spans render as
empty, and a URL left host-less after globals render fails closed.

## Profile document (v1)

```json
{
  "profileVersion": 1,
  "id": "minecraft-gifts",
  "name": "Minecraft gifts",
  "description": "…",
  "params": { "commandHost": "127.0.0.1", "commandPort": 8080 },
  "templates": [
    { "template": { "templateVersion": 1, "id": "…", "…" : "…" }, "params": {} }
  ]
}
```

1–32 inline entries. Each entry stays a valid standalone template so it can
also be imported on its own or from the desktop gallery.

## Active profile (desktop)

The Behavior header shows a profile select next to the Templates button so
packs switch with one gesture. Pack membership lives in host app.state
(`behavior.profiles.packs` + `behavior.profiles.active`); records carry no
profile column. `default` always exists and owns every rule outside the
stored packs; rules saved by hand while another pack is active are adopted
into it. Switching enables exactly the target pack's rules and disables
everything else, and the tables list only the active pack's rules — a fresh
pack shows empty tables, never foreign rules. Ids whose records were
deleted prune silently. The selector
menu creates empty packs, opens the profile import dialog for
`.tikprofile.json` files (staged preview plus an editable settings form
prefilled from the file, so imports never silently apply defaults), exports
any pack back to a profile file (CLI `profile-export` shape, round-trippable),
and deletes packs with a confirm — deletion removes the pack's events and
actions too. Only `default` refuses deletion. Re-applying a profile replaces
its membership
(previous records fall back to `default`). Profile params merge as schema
defaults → profile → entry → dialog edits (CLI: `--param` instead);
per-entry options stay a CLI feature (`--param`, `--event-name`, `--disabled`).

## CLI

```bash
# Offline plan: validate, merge params, print the records that would be created.
tiktools template import rules.tiktemplate.json --dry-run --param giftName=Rose

# Instantiate records (actions first, then the linked event) and store the
# document in the template gallery shared with the desktop modal.
tiktools template import rules.tiktemplate.json --param giftName=Rose

# Profiles: same flow for every entry, profile params merged underneath.
tiktools template profile-import examples/profiles/minecraft-gifts.tikprofile.json --dry-run
tiktools template profile-import examples/profiles/minecraft-gifts.tikprofile.json

# Round trip back to files.
tiktools template export --event <event-id> --out rose.tiktemplate.json
tiktools template profile-export --out backup.tikprofile.json

# Gallery.
tiktools template list
tiktools template delete <template-id>
```

Useful flags: `--event-name` / `--action-name` (single-template imports),
`--no-save` (skip the gallery), `--disabled` (create records disabled).
Imports check requirements first and refuse with a clear error when an
action type is unavailable (missing or disabled plugin) or a trigger is
unknown; nothing is created in that case.

## Minecraft example

`examples/profiles/minecraft-gifts.tikprofile.json` replicates the classic
gift wall: Roses give apples, Galaxies give diamonds, TNT gives TNT, GG
grants speed, Hearts heal, and follows post a welcome message. Every rule is
a `core.fetch` POST to the local CommandAPI:

```text
POST http://127.0.0.1:8080/api/chat   {"messages": ["/give @p minecraft:apple 3", "/title @a times 10 50 10", ...]}
```

Each rule fires one batched `messages` request: the reward command plus
`/title` commands that flash a big colored title and a thanks subtitle
naming the sender, so one HTTP call covers reward and announcement.

CommandAPI runs on loopback with an ephemeral port by default, so the
templates set `allowPrivateNetwork: true` and take `commandHost` /
`commandPort` params — override once per profile instead of editing six
rules (`--param commandPort=9090`). Because the port changes on every
Minecraft start, either pin it in-game (`/commandapi port 8080`) or point
the rules at a runtime global (`http://{{ globals.commandHost }}:{{
globals.commandPort }}/api/chat`) and update the value after each start
(`tiktools globals set commandPort 46665`, or the desktop Settings tab).
Start Minecraft with the mod, join a world (commands fail while no world
is loaded), then import:

```bash
tiktools template profile-import examples/profiles/minecraft-gifts.tikprofile.json
tiktools automation list --kind all
```

Caveat: streakable gifts emit one event per combo tick with the running
`repeatCount`, so a ×5 Rose streak fires five commands. Tune with per-rule
`cooldownMs` if your server economy minds.
