# Media host API

TikTools stores references to user media, not duplicate files. A reference is
JSON metadata plus a canonical path. The native host revalidates the path every
time it opens or plays it.

## Rust core API

The GUI-independent API is available from `tiktools-core`:

```rust
use tiktools_plugin_api::{AudioPlayOptions, MediaFileRef, MediaPickerOptions};

let selected = core.open_media_picker(MediaPickerOptions::default()).await?;
let Some(selection) = selected else { return Ok(()); };

if let tiktools_plugin_api::MediaSelection::File { file } = selection {
    core.play_audio(file, AudioPlayOptions::default()).await?;
}

// A process/WASM plugin can also pass a minimal reference. The host fills and
// checks metadata; it never trusts stale metadata supplied by the plugin.
core.play_audio(
    MediaFileRef::from_path("/music/alert.wav"),
    AudioPlayOptions { volume: 0.75, ..Default::default() },
).await?;
```

`MediaPickerOptions::default()` selects an audio file. Set `mode` to
`MediaPickerMode::Directory` to receive a `MediaDirectoryRef` instead. The
selection contains the canonical directory path and name without copying the
folder.

The audio picker accepts AAC, FLAC, M4A, MP3, OGA/OGG, Opus, WAV, WebM, and
MP4 containers. MP4 is treated as audio here: playback uses the container's
audio track and does not copy or play its video track.

## Frontend API

The Vue controller exposes the same request as a callback API:

```ts
app.openMediaPicker(
  { mode: 'file', kind: 'audio', extensions: ['wav', 'mp3'] },
  (selection, error) => {
    if (error) return showError(error);
    if (selection?.type === 'file') savePath(selection.file.path);
  },
);
```

The transport is the existing JSON IPC contract:

```json
{
  "type": "open-media-picker",
  "requestId": "media-42",
  "mode": "file",
  "kind": "audio",
  "extensions": ["wav", "mp3"]
}
```

The host answers with `media-selected`. Cancellation is represented by an
omitted `selection` and no `error`.

## Runtime boundaries

QuickJS scripts and the sample process plugin do not get filesystem handles,
audio objects, or arbitrary host callbacks through the TikTools protocol. They
return a bounded JSON audio intent. The SDK emits the typed `intents` shape;
the compatibility boundary also accepts legacy `playAudio` responses. The
core capability broker checks the plugin manifest and the host validates the
path before invoking the desktop provider. A process is a crash boundary, not
an OS sandbox: its executable still has the user's normal permissions. WASM
provides the execution sandbox; WASI must be explicitly configured and only
grants the system capabilities the host chooses to expose. The optional WASM
runtime is the place for explicit WASI/host imports when untrusted code must
be supported.

Native libraries remain trusted code and are not a sandbox. Use the process or
WASM runtime for downloadable/untrusted extensions.
