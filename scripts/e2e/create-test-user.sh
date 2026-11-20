#!/usr/bin/env bash
# Create a temporary email/password account on the local backend.
# Usage: scripts/e2e/create-test-user.sh <email> <password> [backend-origin]
# The origin must be one the backend trusts (its APP_ORIGIN, default http://127.0.0.1:3000).
set -euo pipefail
email="$1"; password="$2"; origin="${3:-http://127.0.0.1:3000}"
code=$(curl -s -o /tmp/tt-e2e-signup.$$ -w "%{http_code}" -H "Origin: $origin" \
  -H "Content-Type: application/json" \
  -d "$(python3 -c 'import json,sys;print(json.dumps({"name":"E2E test","email":sys.argv[1],"password":sys.argv[2]}))' "$email" "$password")" \
  "$origin/api/auth/sign-up/email")
rm -f /tmp/tt-e2e-signup.$$
[ "$code" = "200" ] || { echo "sign-up failed with HTTP $code (is the backend running at $origin?)"; exit 1; }
echo "created $email"
