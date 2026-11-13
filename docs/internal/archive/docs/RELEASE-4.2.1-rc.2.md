# TikTools 4.2.1-rc.2

Follow-up to RC1's reproduced Changes read contention. Linux candidate only:
AppImage, DEB and portable archive. GitHub Actions remains disabled, so artifacts
are compiled locally. Published RC1 assets are retained unchanged.

## Correction

Changes/conflict lists and protocol selection now share protocol observation
instead of reserving product mutation admission. Parallel toolbar/panel reads
can succeed while an unrelated package journal transaction occupies that slot.
Write fences, account scoping and explicit migration decisions remain in place.

The final account check waits for another short reader to finish, while rejecting
logout, pending transitions and replacement by another login to the same account.
Local lists do not restore credentials or initiate Cloud requests. Account
restoration remains part of the existing account startup flow.

Read failures show the native reason and a retry that only reloads the local
comparison. Retrying does not upload, fetch or resolve any record.

## Verification

- The deterministic native reproduction fails before the fix and passes all
  32 readers after it, with product mutation admission deliberately occupied.
- 37 CloudAuth tests, 169 sync-engine tests, 21 native v2 tests, one compatibility
  discovery test and 64 frontend sync tests pass.
- Privacy regressions cover damaged/disabled consent, foreign origin, saved but
  unrestored credentials and session replacement during a blocked disk read.
- Frontend TypeScript/ESLint and Rust formatting checks pass.
- The optimized desktop reports `4.2.1-rc.2`. The actual AppImage reaches
  frontend-ready with an empty native profile and separate XDG data/cache/config
  directories; no saved account is present. No startup appsink error or panic
  was observed. This does not test signed-in production sync or media playback.
- AppImage/DEB installer content verification and portable archive checks pass.
  AppImage SHA256:
  `35fc26e7bc5875c5a3fe99bf6a84baf77af8ca63d1c8b8e61825881187d6091f`.

## Remaining scope

Nickname search and recipient-bound invitations require backend commit `6a69c99`
and migration `20261113000001_remote_invitation_recipients.sql`. This release
does not deploy that backend change. During the RC1 check the new people route
was absent from production.

Automations still compares against a local reference copy. The remaining Cloud
section adapters/application, Variables and Overlays, are unfinished. The
historical LIVE SQLite lock holder remains unidentified; diagnostic codes and
a competing-writer regression are present.

RC1's copied-profile migration/account-policy event and retained conflicts are
recorded in [the debugging plan](SYNC-CHANGES-PLAN.md#421-rc1-reproduction).
RC2 smoke tests use a fresh isolated profile without saved account credentials.
