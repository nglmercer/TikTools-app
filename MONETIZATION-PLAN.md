# Monetization plan - TikTools Cloud

Status: proposal (billing not implemented). Date: 2026-11-21.

To test backup/restore locally without deploying Railway, see `LOCAL-BACKUP-RESTORE-TESTING.md`.

## 0. Current state

Backup and restore are implemented end to end:

- `GET /api/v2/me/backup` - build a portable manifest.
- `POST /api/v2/me/restore` - dry-run summary or apply.
- `/api/v2/me/backups*` - store named backups in the account, list, download, delete and restore them.
- Desktop UI: Settings > Account and Cloud > Backup and restore, with a cloud/local mode switch and an explicit "upload to my account" action.
- Backups are available on **every plan**, including Free. Plans differ only in quotas.

Quotas are enforced in `crates/cloud-api/src/plans.rs` (`max_backups`, `max_backup_bytes`) and at creation time in `backup.rs`. Usage is returned by `/api/v1/native/plan` (`usage.backups`, `usage.backupBytes`) and shown in the Plan and storage section.

## 1. Goal

Turn the plans that already exist (`free | pro | max`, manually assigned by an admin) into a billable product **without changing which features are available**. Payment buys capacity and convenience, never access to a local feature or to the ability to back up your own data.

## 2. Principles

1. **Local-first, fail-open**: without an account, connectivity, or with a failed payment, everything that works today keeps working. Backups (local file and cloud) always work within the Free quota.
2. **The user's data belongs to the user**: export is never a paywall. Secrets, TikTok cookies and credentials never leave the device.
3. **No feature gating, only limits**: the same features on every plan - only quotas and retention differ. This keeps the product honest and avoids breaking existing users when a payment fails.
4. **Restore with review**: restoring shows the counts in a summary before applying, and never silently mixes unrelated states.
5. **Single source of truth**: Stripe/webhooks for entitlements; `POST /api/v1/admin/account-plan` stays as support override.

## 3. Proposed plans

All plans include: local automations, feed, plugins, overlays, v1/v2 sync, privacy export, local file backups, cloud backups, restore (dry-run summary + apply), and session renewal.

| Quota (per account) | Free | Pro | Max |
|---|---|---|---|
| Suggested price (to validate) | 0 | ~$4.99/mo or $39/year | ~$12/mo or $99/year |
| Workspace records | 1,000 | 10,000 | 50,000 |
| Workspace storage | 8 MB | 64 MB | 256 MB |
| Assets | 200 / 32 MB | 1,000 / 256 MB | 5,000 / 1 GB |
| Overlays | 50 / 16 MB | 250 / 64 MB | 1,000 / 256 MB |
| **Cloud backups** | **3** | **20** | **100** |
| **Backup storage** | **16 MB** | **128 MB** | **1 GB** |

Why these numbers:

- **3 backups on Free** is enough for "before the trip", "before the update", and a nightly copy - the safety net every user should have.
- **20 on Pro** covers a month of daily copies for active creators.
- **100 on Max** is effectively unlimited for a single account, with 1 GB total to bound cost.

A per-backup cap (`MAX_BACKUP_BYTES`, 64 MB today) stays as a hard safety ceiling regardless of plan.

Not monetized: ad removal (no ads), unlocking basic automation (already free), and charging for export (the data belongs to the user).

## 4. Architecture and technical changes

### 4.1 Backend (RemakeBackend) - already done

- `cloud_backups` table with owner, name, manifest, size and per-domain counts.
- Quota fields live in `plans.rs`; `backup_create_named` counts existing rows and their bytes before inserting.
- `usage_for` counts backups for the `/api/v1/native/plan` response.

### 4.2 Billing (still to build)

1. Tables: `billing_customers`, `billing_subscriptions`, `billing_invoices`, `billing_webhook_events` (dedupe by `event.id`).
2. Endpoints: `POST /api/v1/billing/checkout`, `POST /api/v1/billing/portal`, `POST /api/v1/billing/webhook` (verified signature), `GET /api/v1/native/entitlements`.
3. Webhook updates `account_plans`/`entitlements` idempotently on: `checkout.session.completed`, `customer.subscription.updated`, `customer.subscription.deleted`, `invoice.payment_failed`.
4. Entitlements: `{ tier, status, features[], limits, graceUntil, serverTime }`. Downgrade sets the Free plan but **never deletes backups or records above the new quota** - the user can still download and restore everything; only creating new backups is blocked until they are under the limit.

### 4.3 Session renewal (still to build)

There is no refresh today: the native token expires and the handoff must be repeated. Before charging, add silent reauthentication or a new device code so a paying user is never locked out. The dev launcher now enables `os-keychain`, and restore failures are recorded in `cloud.status.error`.

### 4.4 Desktop UI (partly done)

- Done: plan limits and usage shown in Settings (including Backups), backup section with cloud/local modes.
- To build: "Plan and billing" section with tier/status and *Upgrade* / *Manage billing* buttons opening the browser (reuse the `cloud.login.open` pattern).
- When a backup limit is reached, show a clear message ("You have 3 of 3 backups. Delete one or upgrade to Pro for 20.") with an Upgrade button - never a generic 413.

### 4.5 Shared contract

Keep `Entitlements`, `BackupManifest`, `StoredBackup` and `RestoreSummary` in `cloud-contracts` and the vendored client so desktop and backend cannot diverge.

## 5. Phases and acceptance criteria

| Phase | Work | Acceptance |
|---|---|---|
| 0 - Security | Session renewal, secret review, webhook signatures | No secrets in a backup; renewable session without manual login |
| 1 - Billing backend | Tables, Stripe, idempotent webhooks, `/entitlements` | Repeated webhook does not change the plan; `past_due` downgrades to Free without deleting data |
| 2 - Quota UX | Limit messages, upgrade dialog, usage meters including backups | User sees "3 of 3 backups" and a working upgrade path; no data deleted on downgrade |
| 3 - Beta and pricing | Closed beta, metrics, copy | 20+ users migrating without incidents; conversion and tickets reviewed |
| 4 - Launch | Enable Stripe in production, docs, support | Billing and entitlements stable for 30 days |

Backup/restore itself is already complete and is **not** a prerequisite for billing - it ships on Free.

## 6. Risks and mitigations

- **Charging for data ownership** -> backups and export stay on Free; only quotas differ.
- **Secrets in backups** -> explicit exclusion list plus automated tests over the manifest.
- **Forged or repeated webhooks** -> signature verification and dedupe by `event.id`.
- **System clock / offline mode** -> 7-day grace, `serverTime`, signed cache.
- **Payment failure** -> downgrade to Free while keeping all data and backups; no destructive blocking.
- **Legal/tax** -> choose entity, VAT handling and refund policy before enabling charges.

## 7. Metrics

Free-to-Pro conversion, 30-day retention, number of backups/restores and their success rate, quota utilization (especially backups), webhook failures, migration-related support tickets.

## 8. Open decisions

Provider (Stripe recommended), final prices and currency, legal entity, whether Max includes teams and SSO, and refund policy.
