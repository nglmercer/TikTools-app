# TikTok LIVE emotes

Status: **integrated, with protocol details still unverified**. The app is
pinned to the signer revision that normalizes inline and standalone emotes.
Feed rows, event details, automation events, and the chat widget receive that
metadata. The shortcode catalog and protobuf index units still need
confirmation against captured LIVE traffic.

## Pipeline

TikTok carries graphical emotes in two forms:

1. Bracketed tokens in chat content, such as `Hello [laughcry]`.
2. Protobuf metadata on `WebcastChatMessage.emotes`, or a standalone
   `WebcastEmoteChatMessage.emote_list` event.

The app pins all `tiktok-signer` crates to
`9b4f344d105c62e23bf3a883c7a35066e001e078`. That revision adds bounded
`LiveEmote` metadata to inline chat and a separate normalized event for
emote-only messages.

Inline metadata flows through `UiEvent.emotes` and
`ChatAutomationData.emotes`. Standalone events use the separate
`tiktok.emote_chat` contract and do not invent a comment string. Both forms
reach the feed, details panel, and OBS chat widget. Emote arrays have an
explicit `NormalizedMessageArray` provenance transform in the generated
automation schema.

## Rendering behavior

`src/shared/emotes/` resolves text and optional metadata into text and image
segments. Rendering uses ordinary text children and `<img>` elements, never
`innerHTML` or `v-html`. URLs must be absolute HTTPS URLs, metadata is bounded
to 64 emotes and 2048-byte URLs, and a failed image falls back to its original
shortcode when one is available.

The signer preserves `EmoteWithIndex.index` without choosing a unit. The UI
tries UTF-8 byte, Unicode code point, and UTF-16 offsets, and uses the index
only if exactly one interpretation points to a complete bracket token and
agrees with any known shortcode. It never slices or rewrites the comment.
Ambiguous indices fall back to shortcode or catalog matching.

## TTS behavior

SonicBoom has matching TypeScript and Rust text sanitizers. They remove only
known emote tokens by default; Unicode emoji removal is opt-in, unknown
bracketed text stays intact, and meaningful zero-width joiners/non-joiners are
preserved. Sanitization runs after trigger matching and before points checks.
The settings card preview calls the same TypeScript sanitizer as the UI.

## Remaining verification

- Catalog entries such as `laughcry`, `happy`, and `wow` are hypotheses, not
  mappings verified against captured traffic. Catalog-only matches remain
  plain text; no artwork URLs are guessed.
- The protobuf index unit is unknown. Conservative candidate matching is a
  guarded fallback, not evidence that TikTok uses a particular offset unit.
- The signer accepts safe `http(s)` artwork URLs. The app renderer intentionally
  accepts HTTPS only, so HTTP-only URLs remain text.
- Actual shortcode mappings, image hosts, and index semantics should be
  confirmed using captured LIVE events before catalog changes are made.
