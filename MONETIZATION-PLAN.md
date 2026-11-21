# Monetization plan - TikTools Cloud

Status: proposal (not implemented). Date: 2026-11-20.

To test backup/restore locally without deploying Railway, see `LOCAL-BACKUP-RESTORE-TESTING.md`.

## 0. Direct answer on PC-to-PC migration

**A full migration is not possible today.** The backend (`/home/meme/Documentos/GitHub/RemakeBackend`) syncs part of the configuration and offers privacy exports, but there is **no user-facing restore endpoint**:

- What syncs today: v1 `plugin-selections` and `automation-profiles`; v2 `plugin-selections`, `rule-templates`, `profiles`, `plugin-settings`, portable `preferences` (a subset) and `variables` (no secrets).
- What cannot be moved to another PC today: feed/event history, points, assets/overlays/packages (v3 are stored but are in no export), secrets/tokens (excluded by design), change history and local pending edits.
- `/api/v1/me/export` and `/api/v2/me/export` are **privacy reports**, not restorable backups (`docs/CLOUD-API-V2.md:469-476`). The only restore is `cloud-api restore-local DIR`, an operator action that rebuilds **the entire server** into an empty database.

For that reason, the monetization plan includes a **user backup/restore** as a prerequisite (Phase 2): it makes no sense to sell a "Pro with migration" plan until migration actually works.

## 1. Goal

Turn the plans that already exist (`free | pro | max`, `crates/cloud-api/src/plans.rs:15-93`, manually assigned by an admin at `plans.rs:192-213`) into a billable product without breaking the local-first behavior of the app, and close the PC migration gap.

## 2. Principles

1. **Local-first, fail-open**: without an account, without connectivity, or with a failed payment, everything that works today keeps working. Payment buys cloud capacity and convenience, not access to local features.
2. **The user's data belongs to the user**: the full JSON export is available on Free too. Secrets, TikTok cookies and credentials never leave the device (already true for `variables`).
3. **Restore with review**: restoring a backup never silently overwrites anything; it shows a diff and applies per record, using the same review pattern as `OwnerLibraryReview` in the desktop app.
4. **Single source of truth**: Stripe/webhooks for entitlements; `POST /api/v1/admin/account-plan` stays as support override, not as a billing mechanism.

## 3. Proposed plans

| | Free | Pro | Max |
|---|---|---|---|
| Suggested price (to validate) | 0 | ~$4.99/mo or $39/year | ~$12/mo or $99/year |
| Local automations, feed, plugins, overlays | yes | yes | yes |
| v1/v2 sync within current quotas | yes | yes | yes |
| Privacy export | yes | yes | yes |
| **Backup and restore PC to PC** | - | yes | yes |
| Cloud-persistent feed history | - | yes | yes |
| Expanded quotas (records/bytes/overlays) | current | 2-4x | current `max` |
| Beta update channels | - | yes | yes |
| Assets/overlays/packages in backup | - | - | yes |
| Remote multi-device + teams (3-5) | - | - | yes |
| Priority support | - | - | yes |

Do not charge for removing ads or for unlocking basic automations: neither exists and neither should appear.

## 4. Architecture and technical changes

### 4.1 Backend (RemakeBackend)

1. **Billing**
   - New tables: `billing_customers`, `billing_subscriptions`, `billing_invoices`, `billing_webhook_events` (dedupe by `event.id`).
   - Endpoints: `POST /api/v1/billing/checkout`, `POST /api/v1/billing/portal`, `POST /api/v1/billing/webhook` (verified Stripe signature), `GET /api/v1/native/entitlements`.
   - The webhook updates `account_plans`/`entitlements` idempotently on: `checkout.session.completed`, `customer.subscription.updated`, `customer.subscription.deleted`, `invoice.payment_failed`.
   - Entitlements response: `{ tier, status, features[], limits, graceUntil, serverTime }`.

2. **User backup/restore (blocking for migration)**
   - `GET /api/v2/me/backup`: versioned manifest with v1 + v2 + v3 (assets, overlays and revisions, packages) + portable preferences. Reuse `workspace::export` (`cloud-api/src/workspace.rs:498-528`) and add the domains currently missing.
   - `POST /api/v2/me/restore`: `mode: merge | replace`, with `dryRun` returning the diff (adds, changes by revision, removals, collisions). Apply per record with `revision`, never as an opaque cross-domain transaction.
   - Explicit per-plan limits (backup size, record count) reusing `plans.rs`.

3. **Session and renewal**
   - There is no refresh today (`docs/DESKTOP-AUTH.md:31`): the native token expires and the handoff must be repeated. Before charging, add session renewal (silent reauthentication or a new device code) so a paying user is never locked out.

4. **Admin**: the admin web app (`web/admin`) shows `tier` and `status`, but plan changes are made by the webhook.

### 4.2 Desktop (Tiktok-Live-APPv3)

1. **Entitlement store** (Pinia, `src/web/entities/cloud-account/`): `refresh()`, signed cache in `appStorage('local')`, 7-day `offlineGrace` and server time to prevent clock tampering.
2. **Settings > Account and Cloud** (`CloudAccountEditor.vue`, `CloudPlanUsage.vue`): new "Plan and billing" section with tier, status (`active`, `trialing`, `past_due`), and *Upgrade* / *Manage billing* buttons (open checkout/portal in the browser, reusing the `cloud.login.open` pattern).
3. **Guided migration**: *Export copy* / *Restore copy* buttons in the same section. Restore reuses the `OwnerLibraryReview` pattern: list of changes, confirmation, per-record result. Always show what is NOT restored (secrets, cookies, local pending edits).
4. **Subtle gating**: lock badge and an upgrade dialog with benefits on Pro/Max features. Never hide or delete data created when a plan expires: keep it and disable only cloud writes.
5. **Minimal opt-in telemetry**: anonymous counters for backup/restore usage and feature usage to adjust prices and quotas before launch.

### 4.3 Shared contract

Add `Entitlements`, `BackupManifest` and `RestoreDiff` types to `cloud-contracts` and the vendored Rust client (`crates/cloud-client/`), using the same pattern as `WorkspaceRecord`, so the desktop and backend cannot diverge.

## 5. Phases and acceptance criteria

| Phase | Work | Acceptance |
|---|---|---|
| 0 - Security (1-2 weeks) | Session renewal, secret review on exports, webhook signatures | No secrets in a backup; renewable session without manual login |
| 1 - Backend billing (2-3 weeks) | Tables, Stripe, idempotent webhooks, `/entitlements` | Repeated webhook does not change the plan; `past_due` downgrades to Free without deleting data |
| 2 - Backup/restore (2-3 weeks) | `/me/backup`, `/me/restore` with `dryRun`, v3 coverage | Pro user migrates PC A to PC B with rules, profiles, plugins, preferences and overlays; reviewable diff |
| 3 - Desktop UI (1-2 weeks) | Plan/billing, upgrade, guided migration, error states | Full flow from Settings without leaving the app (except the payment browser) |
| 4 - Beta and pricing (2 weeks) | Closed beta, metrics, copy, quotas | 20+ users migrating without incidents; conversion and tickets reviewed |
| 5 - Launch | Enable Stripe in production, docs, support | Billing and entitlements stable for 30 days |

## 6. Risks and mitigations

- **Selling migration before it works** -> Phase 2 is mandatory before Phase 3.
- **Secrets in backups** -> explicit exclusion list plus automated tests over the manifest.
- **Forged or repeated webhooks** -> signature verification and dedupe by `event.id`.
- **System clock / offline mode** -> 7-day grace, `serverTime`, signed cache.
- **Payment failure** -> downgrade to Free while keeping all data; no destructive blocking.
- **Legal/tax** -> choose entity, VAT handling and refund policy before enabling charges.

## 7. Metrics

Free-to-Pro conversion, 30-day retention, number of backups/restores and their success rate, webhook failures, migration-related support tickets, actual quota usage.

## 8. Open decisions

Provider (Stripe recommended), final prices and currency, legal entity, whether Max includes teams and SSO, and refund policy.

---

### Note

The backend already has the foundation (identity, handoff, v1/v2 sync, plans and quotas); what is missing to call it "fully implemented" is: **user-level restore, v3 coverage in the backup, session renewal, billing, and the UI that exposes them**. The rest of the app (feed, automations, guides, empty state, responsive layout) is already implemented and validated.
