# Local backup/restore testing (no Railway)

A guide to verify the cloud backup/restore end to end with a local backend. No Railway deployment needed.

Repositories:

- Backend: `/home/meme/Documentos/GitHub/RemakeBackend`
- App/CLI: `/home/meme/Documentos/GitHub/Tiktok-Live-APPv3`

---

## 1. Start the local backend

Option A (Docker for the database only):

```sh
cd /home/meme/Documentos/GitHub/RemakeBackend
cp .env.example .env
# Edit .env with these minimum values:
#   BIND_ADDR=127.0.0.1:3000
#   DATABASE_URL=postgres://cloud:local-only-password@localhost:5432/cloud
#   AUTH_SECRET=<at least 32 random bytes>
#   APP_ORIGIN=http://127.0.0.1:3000
# Comment out STORAGE_* (they point to an S3 not present in compose), or use
#   STORAGE_LOCAL_DIR=/tmp/tiktools-storage
docker compose up -d
export $(grep -v '^#' .env | xargs)
cargo run -p cloud-api -- migrate
cargo run -p cloud-api
```

Option B (no Docker, uses the bundled pgrust):

```sh
cd /home/meme/Documentos/GitHub/RemakeBackend
cargo run -p devtools -- db install   # once
cargo run -p devtools -- db up
cargo run -p devtools -- dev --no-watch
```

`dev --ui` changes `APP_ORIGIN` to `http://127.0.0.1:5173`; use `--no-watch` to keep the origin at `:3000`. Migrations **do not** run automatically on startup: run `migrate` once.

Check it responds:

```sh
curl http://127.0.0.1:3000/health/ready
```

---

## 2. Create a test user

```sh
curl -X POST http://127.0.0.1:3000/api/auth/sign-up/email \
  -H 'Origin: http://127.0.0.1:3000' \
  -H 'Content-Type: application/json' \
  -d '{"name":"Backup test","email":"backup@example.test","password":"Test-password-123!"}'
```

The `Origin` header must match `APP_ORIGIN` exactly; otherwise the backend returns `403 request origin is not trusted`.

---

## 3. Point the app and CLI at the local backend

In debug builds the runtime override works:

```sh
export TIKTOOLS_CLOUD_ORIGIN=local                       # = http://127.0.0.1:3000
# or: export TIKTOOLS_CLOUD_ORIGIN=http://127.0.0.1:3000
```

Release builds ignore that variable; build with the `e2e-cloud-origin` feature and use `TIKTOOLS_E2E_CLOUD_ORIGIN=http://127.0.0.1:3000` (loopback only).

Sign in either way:

```sh
# Desktop app (Settings > Account and Cloud > Sign in opens /desktop/login)
TIKTOOLS_CLOUD_ORIGIN=local bun run start

# or automated (creates the account if missing, performs the handoff, leaves the session ready)
bun scripts/e2e/login-instance.ts --target desktop \
  --email backup@example.test --password 'Test-password-123!'
```

Verify with `target/debug/tiktools cloud status`; it should show `signed-in`.

---

## 4. Test from the UI

1. Open **Settings > Account and Cloud > Backup**.
2. Click **Create backup**: a `tiktools-cloud-backup-YYYY-MM-DD.json` file is downloaded.
3. Change something in the cloud (for example disable a plugin under **Plugins** and sync).
4. Click **Restore backup**, select the JSON. A summary with counts is shown.
5. Click **Restore now** and verify the previous change is gone.

---

## 5. Test from the CLI

The CLI talks to the native host over IPC, so keep the app running (or use `--standalone` after signing in).

```sh
cd /home/meme/Documentos/GitHub/Tiktok-Live-APPv3
export TIKTOOLS_CLOUD_ORIGIN=local
cargo build -p tiktools-cli

target/debug/tiktools cloud backup create --out /tmp/copia.json
target/debug/tiktools cloud backup preview /tmp/copia.json
target/debug/tiktools cloud backup restore /tmp/copia.json           # summary only
target/debug/tiktools cloud backup restore /tmp/copia.json --yes     # applies the restore
```

`preview` and `restore` without `--yes` are read-only: they show how many records, documents and files would be imported.

---

## 6. Simulate a fresh, empty PC

Isolate the native state with a different `TIKTOOLS_HOME` and sign in with the **same account**:

```sh
TIKTOOLS_HOME=/tmp/tiktools-pc-b TIKTOOLS_CLOUD_ORIGIN=local bun run start
```

With no local data, the restore uses `replace` mode without conflicts: select the JSON, review the summary and confirm. From the CLI:

```sh
TIKTOOLS_HOME=/tmp/tiktools-pc-b TIKTOOLS_CLOUD_ORIGIN=local \
  target/debug/tiktools cloud backup restore /tmp/copia.json --yes
```

---

## 7. Call the endpoint directly (optional)

With a valid `sessionToken` (the native handoff stores it in the OS keychain):

```sh
curl -H "Authorization: Bearer $TOKEN" http://127.0.0.1:3000/api/v2/me/backup \
  | jq '{version: .schemaVersion, rules: (.workspaceSync.records | length), assets: (.assets.items | length)}'

curl -X POST http://127.0.0.1:3000/api/v2/me/restore \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d "{\"manifest\": $(cat /tmp/copia.json), \"mode\": \"replace\", \"dryRun\": true}"
```

---

## 8. Automated backend test

The added test is `#[ignore]` and needs a test database with `CREATEDB`:

```sh
cd /home/meme/Documentos/GitHub/RemakeBackend
CLOUD_TEST_DATABASE_URL=postgres://cloud:local-only-password@127.0.0.1:5432/cloud_test \
  cargo test --locked -p cloud-api \
  me_backup_and_restore_dry_run_report_a_versioned_summary \
  -- --ignored --exact --test-threads=1
```

---

## What is NOT restored (important for the summary shown to the user)

Secrets, TikTok cookies, session tokens, local feed history and plugin binaries. Plugin binaries are downloaded again from the configured registry. Passwords/sessions are renewed by signing in to the account, not by the backup.

## Common problems

- `403 request origin is not trusted` -> the `Origin` header does not match `APP_ORIGIN`.
- `TIKTOOLS_CLOUD_ORIGIN` is ignored -> you are using a release build; use a debug build or the `e2e-cloud-origin` feature.
- Storage error at startup -> the checked-in `.env` points to S3 on `:9000`; comment out `STORAGE_*` or set `STORAGE_LOCAL_DIR`.
- Ignored tests fail on permissions -> the `CLOUD_TEST_DATABASE_URL` role needs `CREATEDB`.
- Missing `pg_dump`/`pg_restore` -> only affects the `DATABASE-RECOVERY.md` drill, not this flow.
