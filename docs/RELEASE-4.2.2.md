# TikTools 4.2.2

Release built by the active workflow in `hernan-lc/TikTools-app`, with
`RELEASE_REPO=nglmercer/TikTools-app` as its publication destination. Existing
4.2.1 candidate assets are retained unchanged.

## Variables

Variables now uses the shared Changes panel. Send publishes only the selected
saved native values; download updates only the cache. Applying a reviewed Cloud
value and removing a Cloud copy are separate, explicit actions. Withdrawal
preserves the local value and its secret classification. Dirty local drafts must
be saved or discarded first, and older hosts without the native bridge hide the
sync control.

The host rechecks the exact session, consent, local revision and target comparison.
Queued edits can be replaced or cancelled. Uncertain uploads use epoch-fenced
recovery, including withdrawal after a variable becomes secret, without replaying
its previous value. Decision generations now advance after acknowledgement, so
a later edit does not fail by reusing generation one. Unchecked Variables and
other sections' pending queues remain outside the selected send operation.

Types remain explicit: text, number, boolean, list and local secret. Saving locally
does not upload. Secret or blocked values have no diff, publish or adopt control.
There is no field merge, force, history or autosync control for this adapter.

## CI corrections

The failures in run `43438391891` were test modules placed before later production
items (`items_after_test_module`) and the plugin lifecycle fixture's stable-only
SemVer range on a candidate host. Test modules/helpers are ordered correctly;
the lifecycle fixture explicitly opts into its candidate host while retaining
its original compatibility bounds. Production plugin compatibility policy is
unchanged. A full-workspace Clippy run also identified an unhandled partial read
in an account test server; the fixture now reads bounded complete headers.

The Linux release job starts the generated AppImage under an isolated graphical
session and empty native/XDG profile before uploading it. A failed start prints
bounded startup diagnostics. Installer content validation, immutable upload,
downloaded checksum verification and signed-feed publication retain their
existing gates.

## Validation gates

- Rust 1.99.1: workspace Clippy, all targets/features, locked, `-D warnings`.
- Serial workspace tests, plugin runtime tests on Linux/Windows, MSRV checks,
  native TikTok regression and sandbox API verification through CI.
- Frontend lint/type checks and complete unit suites.
- Browser tests of typed Variables, reviewed adoption and secret withdrawal.
- Linux/Windows installer/portable packaging and uploaded checksum checks.

## Remaining scope

Teams nickname discovery and recipient-bound invitations still require backend
commit `6a69c99` and additive migration
`20261113000001_remote_invitation_recipients.sql`. This release does not deploy
the backend. Automations Cloud transport, Overlay revisions, history/branch
provenance and the remaining Collaboration presentation audit are separate work.
The historical LIVE SQLite lock holder has not been identified; extended reason
codes and the competing-writer retry regression remain available.

RC1's copied-profile account-policy event is documented in the
[debugging plan](SYNC-CHANGES-PLAN.md#421-rc1-reproduction). Local validation of
this release uses isolated profiles and fake accounts; it does not establish
signed-in production sync or production Teams compatibility.
