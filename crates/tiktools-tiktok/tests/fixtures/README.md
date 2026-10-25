Synthetic protobuf fixtures from nglmercer/tiktok-signer revision 4f9d0b63799c3e23cf20be717779ece507b9eed4, fixtures/events. No real session or user data. Decoded by the existing pinned ttl-live-events implementation.

`deep-gift.pb` preserves only the field structure of a public LIVE gift that
was valid for Prost but truncated by the SDK's eight-level diagnostic view.
Every number was replaced by 1; every string by `fixture`; opaque bytes and
unknown fields were removed. No original sender, chat, ID, URL, cookie or
other original field value is retained.

`deep-gift.batch.pb` wraps that structural fixture in a synthetic SDK batch
with msgId 9007199254740994 to test the actual decoder-to-core/UI path.
