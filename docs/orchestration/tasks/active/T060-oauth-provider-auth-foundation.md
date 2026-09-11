# T060 — OAuth2 provider-auth foundation (spike)

**Epic:** T059. **Priority:** P2.
**Depends:** nothing (first child, unblocks T061–T063).
**Code:** new module, likely `crates/chronos-fm-services/src/cloud_auth/`
(name TBD by the spike) + keyring integration (`docs/persistence.md`
precedent).

## Problem

Google Drive / OneDrive / Dropbox all require OAuth2; S3's static-key +
keyring pattern (T011/T039) does not apply. No ticket in this repo has
built an OAuth flow before — this spike proves one working end-to-end
against a **single** real provider before three tabs are built on top of
an unverified foundation.

## Must

1. **Pick a flow and name why.** Device-code flow is the default
   recommendation for a desktop app (no embedded webview dependency, no
   local redirect-server port to manage) — but confirm against what
   Source/gpui and this app's process model can actually support before
   committing; if device-code isn't offered by a target provider's OAuth
   app type, say so and pick the alternative with a stated reason, not
   silently.
2. **One provider proven end-to-end**, recommended Google (most requested,
   Drive API v3 well-documented): app registration steps documented (not
   secrets — the *steps*, so T061 can repeat them), token acquisition,
   refresh-token exchange, and storage in keyring (same mechanism as
   T039's S3 credentials — reuse, don't reinvent).
3. **Token refresh works**, not just initial acquisition — expire/refresh
   is the failure mode that silent-breaks OAuth integrations in
   production.
4. **Never write tokens to config or logs.** Same rule as T039's
   `persist_credentials` seam — verify with a grep/keyring-lookup, not an
   assumption.
5. **Design-gate note for T061–T063**: what part of the flow is shared
   code vs. what each provider's OAuth app registration and token
   endpoint specifics require individually.

## Explicitly out of scope for this ticket

- Any Drive/OneDrive/Dropbox `FileSystemProvider` implementation — that's
  T061/T062/T063.
- UI — this is a services-layer spike; a CLI-only or test-only proof of
  the token flow is sufficient.

## Done when

1. Working token acquisition + refresh against a real Google account,
   demonstrated by a command/test that lists something from the Drive API
   using the stored token (proof the token is real and usable, not just
   that a flow completed).
2. Keyring storage verified (lookup after the flow shows the token;
   config/log files verified clean of it).
3. Design-gate note written for T061–T063 (shared vs. provider-specific).
4. Report + architect stamp. No self-ACCEPT.

## Related

T059 (epic) · T039 (keyring/credential precedent) · `docs/persistence.md`
