# TikTools 4.2.1-rc.1

Candidate for reproducing the reported sync, Teams and Linux AppImage problems.
The application and backend changes are committed separately: app `338495a9`,
RemakeBackend `6a69c99` (`fix/teams-nickname-invitations`).

GitHub Actions is disabled in the repository. This candidate uses locally
compiled Linux artifacts (AppImage, DEB and portable archive), verified before
publication. It does not include a newly compiled Windows installer.

## Changes to verify

| Area | Expected behavior |
| --- | --- |
| Settings | Account/session and diagnostics remain; the old sync editor is removed. |
| Plugins | Plugin-only catalog, search, Changes, Install plugin and secondary menu. Templates/profiles stay in Automations. |
| Changes | One shared panel contains setup/recovery and the section comparison. A load error never claims everything is synced. |
| Plugin sync | Only plugin selections/settings are published by the section action. Other queues remain intact and downloaded records remain cached. |
| Migration | An absent implicit bundled-plugin directory is allowed. Explicit directories remain strict. Missing local packages are identified and can be repaired only after reviewing the exact IDs. Pending edits and conflicts are retained. |
| Teams | Create/select a team, enroll this PC as its host, then start serving. A host enrolled on another PC cannot enable this PC's Start action. |
| Invitations | Search by nickname, select an unambiguous result, create an invitation. Only that account can redeem it; the host still approves access. Emails are not displayed by search. |
| Guides | The optional question-mark control opens Driver.js help. Teams steps follow the real setup state. |
| AppImage | WebKit helpers, GStreamer plugins and scanner come from the bundle. Verify audio/video and absence of `appsink`/plugin-scanner failures. |

## Backend dependency

Nickname search and recipient-bound invitations require RemakeBackend commit
`6a69c99` and migration `20261113000001_remote_invitation_recipients.sql`.
The application prerelease does not deploy the backend or run production migrations.
If search fails against an older backend, record the Cloud origin and request ID
from Diagnostics before treating it as an AppImage regression.

## Reproduction record

For every remaining failure, record the displayed application version, operating
system, exact action, error/request ID, and relevant lines from Diagnostics/logs.
Do not include tokens, invitation codes or secret variable values.

Use an isolated copy of the affected profile for migration recovery tests. Review
the package IDs before confirming repair, then check that pending edits/conflicts
are preserved and preview migration again. Do not infer a successful migration
from a successful compatibility check.

The old LIVE session log contains SQLite lock errors and collaboration-retention
failures. Extended SQLite error codes are now logged. A competing-writer test
verifies rollback and successful retry, but the original lock holder remains
unidentified; this prerelease does not claim that historical cause is fixed.

## Scope still in progress

Automations Changes still compares against the local reference copy. Cloud sync
and reviewed application for the remaining section adapters, including Variables
and Overlays, are unfinished. These are separate from the fixed Plugins action.

## Validation

The implementation commit passed 146 frontend tests, 167 sync-engine tests,
PostgreSQL recipient-isolation tests and two-host native/relay acceptance.
The release version checks and packaging verification must also pass for this
candidate. Smoke-test the actual compiled AppImage, including its reported
`4.2.1-rc.1` version, before reporting the candidate as verified.
