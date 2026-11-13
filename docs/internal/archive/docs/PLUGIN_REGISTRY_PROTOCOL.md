# Plugin registry protocol (v1)

A registry is a tree of static JSON files served over HTTPS. Any static host
works (GitHub Pages, R2, S3, a CDN); a dynamic service can serve the same
files later without client changes. The client lives in
`crates/tiktools-plugin-registry`; `tiktools-plugin index` writes registries.

## Layout

```text
<base>/root.json                      root public keys (unsigned; pinned by fingerprint when added)
<base>/keys.json      + .sig          delegated keys, signed by a root key
<base>/config.json    + .sig          protocol, name, catalog size, revoked digests, optional api
<base>/index/<aa>/<bb>/<id>.json + .sig   one document per plugin, sharded by id prefix
<base>/catalog/page-<n>.json + .sig   paged summaries for browsing
<base>/packages/<id>/<file>.plugin    artifacts (may live on any other host)
```

`<aa>` and `<bb>` are the first and next two characters of the id, padded
with `_` (`acme.tts` → `index/ac/me/acme.tts.json`). Clients send
`If-None-Match` with the stored `ETag` and fall back to the cached copy when
the registry is unreachable; cached copies are verified again before use.
Unknown fields are ignored. A breaking change publishes under a new base
(`/v2/`) and bumps `config.json.protocol`.

## Trust model

```text
root keys      pinned by the client (compiled in for the official registry,
               confirmed by fingerprint for registries a user adds)
  └─ keys.json (signed by a root key) delegates:
       index             signs config.json, index documents, catalog pages
       publisher:<name>  signs packages whose ids it may publish
```

Every signature is a detached ed25519 signature over the exact bytes of the
signed file, stored next to it as `<file>.sig`:

```json
{ "version": 1, "algorithm": "ed25519", "keyId": "index-1", "signature": "<base64>" }
```

A delegated key entry:

```json
{
  "keyId": "acme-1",
  "algorithm": "ed25519",
  "publicKey": "<base64 32 bytes>",
  "role": "publisher",
  "publisher": "acme",
  "scopes": ["acme.*"],
  "notBefore": 1767225600,
  "notAfter": 1798761600,
  "revoked": false
}
```

A publisher key without `scopes` may sign ids that start with
`<publisher>.`. `scopes` grants grandfathered ids explicitly
(`["tiktools.*", "sonicboom.server"]`). Delegations never replace pinned root
keys. Revoked or expired keys fail verification.

## Packages

A `.plugin` file is a ZIP with `plugin.json`, the entry, assets,
`checksums.json` (SHA-256 of every other file), and for signed packages
`signature.json`, a detached signature over the exact bytes of
`checksums.json`:

```json
{ "version": 1, "algorithm": "ed25519", "keyId": "acme-1", "publisher": "acme", "signature": "<base64>" }
```

Install order: archive digest against the index → signature against a
delegated publisher key that may publish the id → manifest identity and
access against the release entry → host compatibility → extraction with
per-file checksum verification.

## config.json

```json
{
  "protocol": 1,
  "name": "official",
  "description": "Official TikTools plugins",
  "api": "https://plugins.example.com/api",
  "mirrors": [],
  "catalogPages": 3,
  "revokedDigests": ["<sha256>"],
  "generatedAt": 1767225600,
  "expires": 1798761600
}
```

When `api` is present, search uses `GET {api}/search?q=&category=` (returns a
catalog page); otherwise clients filter the static catalog pages.
`revokedDigests` blocks installation of those exact archives.

## Package document

```json
{
  "id": "acme.tts",
  "name": "Acme TTS",
  "publisher": "acme",
  "description": "Reads chat aloud.",
  "icon": "voice",
  "categories": ["audio"],
  "tags": ["tts"],
  "homepage": "https://acme.dev/tts",
  "repository": "https://github.com/acme/tts-plugin",
  "license": "MIT",
  "screenshots": [],
  "dist-tags": { "stable": "1.2.0", "beta": "1.3.0-beta.1" },
  "releases": [
    {
      "version": "1.2.0",
      "channel": "stable",
      "publishedAt": 1767225600,
      "yanked": false,
      "advisory": null,
      "runtime": "process",
      "engines": { "tiktools": ">=0.4.0" },
      "schemaVersion": 3,
      "protocolVersion": 1,
      "abiVersion": null,
      "permissions": ["audio.output"],
      "capabilities": ["audio.play"],
      "dependencies": {},
      "changelog": "# 1.2.0 …",
      "artifacts": [
        {
          "target": "win32-x64-msvc",
          "url": "packages/acme.tts/acme.tts-1.2.0-win32-x64-msvc.plugin",
          "size": 4194304,
          "sha256": "<hex>",
          "keyId": "acme-1"
        }
      ]
    }
  ]
}
```

`target` is a plugin target (`win32-x64-msvc`, `linux-x64-gnu`, …) or `any`
for portable packages. Relative artifact URLs resolve against the registry
base; absolute URLs must be HTTPS.

## Resolution

For a spec `id[@requirement]` the client picks the registry that claims the
id by scope, otherwise the enabled unscoped registries in priority order, and
selects the highest release that:

- matches the requirement (a bare version means exactly that version);
- is not yanked, unless that exact version was requested;
- is on the `stable` channel without a prerelease tag, or also `beta` when the
  plugin or default policy follows beta;
- satisfies `engines.tiktools`, the protocol, schema, and ABI versions, and a
  runtime available on this host;
- has an artifact for this host's target or `any`.

When nothing qualifies the error lists every rejected version and why.

## Updates and advisories

The host checks registry-installed plugins on startup and every
`checkIntervalHours` (default 12). An update applies automatically only when
the plugin's policy allows that kind of change (`patch` or `minor`), it is not
pinned, the new release is signed by the same publisher, and it requests no
new permissions or capabilities; everything else waits for review. A release
`advisory` (`low`, `moderate`, `high`, `critical`) is shown for the installed
version even when no update exists.

## Operating a static registry

```bash
tiktools-plugin keygen --id root-1 --role root --out keys
tiktools-plugin keygen --id index-1 --role index --out keys
tiktools-plugin index init site/v1 --name official --root-key keys/root-1.key.json --index-key keys/index-1.key.json
tiktools-plugin index delegate site/v1 --root-key keys/root-1.key.json --delegate acme-1.pub.json
tiktools-plugin publish acme.tts-1.2.0-win32-x64-msvc.plugin --registry site/v1 --index-key keys/index-1.key.json
tiktools-plugin index yank site/v1 acme.tts@1.1.0 --index-key keys/index-1.key.json
tiktools-plugin index advise site/v1 acme.tts@1.1.0 --severity high --message "Fixed in 1.2.0" --index-key keys/index-1.key.json
tiktools-plugin index revoke site/v1 <sha256> --index-key keys/index-1.key.json
```

Keep the root key offline; it is only needed to change delegations. The index
key signs every regenerated document and can live in CI. Publish the root key
fingerprint (`index init` prints it) where users can compare it when they add
the registry. Hosts built with `TIKTOOLS_OFFICIAL_REGISTRY_URL` and
`TIKTOOLS_OFFICIAL_REGISTRY_ROOT_KEYS` (a JSON array of public keys) ship that
registry preconfigured as `official`.
