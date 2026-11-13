---
title: "Sign in to your Cloud account"
docId: "user.cloud.account"
translationKey: "user.cloud.account"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "67b7118973ffa6e904cad9833e9cc82ae2170c517692349403f990e16c599d63"
translationRevision: "67b7118973ffa6e904cad9833e9cc82ae2170c517692349403f990e16c599d63"
order: 10
---

# Sign in to your Cloud account

## Goal and prerequisites

Connect the desktop app to its configured Cloud service for account-based catalog and collaboration operations. You need network access, a system browser and a working operating system credential store. Local editing remains useful without Cloud; a Cloud session does not authenticate your TikTok LIVE connection.

## Sign in and verify identity

1. Open **Settings → Account**. Read the current state before starting another login.
2. Select **Sign in**. The desktop starts a local callback receiver and opens the configured login page in the system browser.
3. Complete the service's login flow and return to TikTools. If the browser did not open, use the login URL shown by the app for this attempt. Treat that URL as private and do not post it publicly.
4. Wait for the account state to become signed in. Confirm the displayed name or email before opening a private catalog or team.
5. Open the Account diagnostics disclosure if the status reports an error. Use **Cancel** to abandon an unfinished attempt rather than leaving several attempts active.
6. When finished on a shared computer, select **Sign out** and verify the account state changed.

The native host handles the session token. It stores credentials through the operating system credential store and keeps only nonsecret status in the frontend. There is no plaintext-file fallback when secure storage is unavailable. A copied data folder is not a portable signed-in session.

## Expected result and status

Native sign-in, restore and sign-out are available in hosts built with Cloud authentication. A configured account can be restored on restart and revalidated with the service. An unconfigured or unsupported host cannot be fixed by entering a different TikTok cookie.

Service login availability and real provider/platform acceptance depend on the deployed service and your machine. Local validation does not prove every production sign-in route works.

## Recovery

For a cancelled or expired attempt, start a fresh login after closing the old one. For network or service errors, preserve the diagnostic code and retry after recovery. For a credential-store error, restore the operating system's secure-storage availability; do not paste a bearer token into settings or a support message. See [Changes](changes.md) for explicit synchronization and [collaboration](collaboration.md) for separate access grants.
