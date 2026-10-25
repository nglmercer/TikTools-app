# Pinned SDK normalization fix

Source: nglmercer/tiktok-signer, commit
`4f9d0b63799c3e23cf20be717779ece507b9eed4`, crates/ttl-live-events.
Upstream declares this crate MIT. The upstream unit-test fixture `fixtures/events/chat.pb` is copied verbatim
as `fixtures/chat.pb`; only the fixture path is relocated and assertions
are unchanged. This is upstream test data, not data from the current LIVE. All other source is
preserved except the wire
preflight depth check in src/dynamic.rs: normalization follows Prost's
100-level recursion bound instead of the diagnostic schema's 8-level view.
The 4 MiB event, 16 MiB batch, 8192 messages, 4096 fields/event and 65536
fields/batch bounds are unchanged. Diagnostic views retain their depth limit.

TikTools requests DecodeMode::Normalized, not the legacy Full diagnostic
representation: truncating a debug view must not turn a valid gift into
Unknown. Regression: crates/tiktools-tiktok/tests/fixtures/deep-gift.pb,
structurally scrubbed from a public LIVE packet (no original values).

Remove this patch when the pinned upstream revision contains the fix.
