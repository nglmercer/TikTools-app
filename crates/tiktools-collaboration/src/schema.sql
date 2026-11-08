CREATE TABLE IF NOT EXISTS metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS invitations (
 id TEXT PRIMARY KEY, verifier TEXT NOT NULL, grant_json TEXT NOT NULL,
 created_at INTEGER NOT NULL, expires_at INTEGER NOT NULL, state TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS principals (
 id TEXT PRIMARY KEY, label TEXT NOT NULL, kind TEXT NOT NULL,
 grant_json TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 1, revoked INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS pairing_requests (
 id TEXT PRIMARY KEY, invite_id TEXT NOT NULL UNIQUE REFERENCES invitations(id),
 exchange_hash TEXT NOT NULL, label TEXT NOT NULL, kind TEXT NOT NULL,
 grant_json TEXT NOT NULL, state TEXT NOT NULL, expires_at INTEGER NOT NULL,
 principal_id TEXT REFERENCES principals(id)
);
CREATE TABLE IF NOT EXISTS sessions (
 id TEXT PRIMARY KEY, principal_id TEXT NOT NULL REFERENCES principals(id), kind TEXT NOT NULL,
 access_hash TEXT NOT NULL UNIQUE, access_expires INTEGER NOT NULL,
 refresh_hash TEXT UNIQUE, created_at INTEGER NOT NULL, last_activity INTEGER NOT NULL,
 absolute_expires INTEGER NOT NULL, revoked INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS used_refresh (
 verifier TEXT PRIMARY KEY, session_id TEXT NOT NULL REFERENCES sessions(id)
);
CREATE TABLE IF NOT EXISTS audit (
 cursor INTEGER PRIMARY KEY AUTOINCREMENT, timestamp INTEGER NOT NULL,
 principal_id TEXT NOT NULL, action TEXT NOT NULL, resource_id TEXT NOT NULL, outcome TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS audit_details (
 cursor INTEGER PRIMARY KEY REFERENCES audit(cursor) ON DELETE CASCADE,
 details_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS audit_imports (
 event_id TEXT PRIMARY KEY, payload_hash TEXT NOT NULL, source_timestamp INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_audit_imports_timestamp ON audit_imports(source_timestamp);
CREATE TABLE IF NOT EXISTS rate_limits (
 category TEXT NOT NULL, identity TEXT NOT NULL, window INTEGER NOT NULL, count INTEGER NOT NULL,
 PRIMARY KEY (category, identity)
);
CREATE INDEX IF NOT EXISTS idx_sessions_principal ON sessions(principal_id);
CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit(timestamp);
