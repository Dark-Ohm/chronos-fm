# T063 — Dropbox tab

**Epic:** T059. **Priority:** P2.
**Depends:** T060 (OAuth foundation) — blocked until T060 ACCEPT.
Independent of T061/T062 — see T062's note on not assuming code transfers
1:1 between providers.
**Code:** `crates/chronos-fm-services/src/dropbox/`,
`crates/chronos-fm-pages/src/dropbox.rs` (T039/T061/T062 shape).

## Problem

Third cloud-provider tab. Dropbox API v2 is RPC-style (single `/2/files/*`
endpoints with JSON bodies), not REST-resource-style like Drive/Graph —
confirm this shape before assuming the same client-code structure as
T061/T062 applies.

## Must

1. `FileSystemProvider` impl against Dropbox API v2
   (`files/list_folder`, `files/download`, `files/upload`,
   `files/delete_v2`, `files/move_v2`). Path model is closest to S3/
   OneDrive (path-addressable) — verify, don't assume, before reusing
   T062's mapping code.
2. Tab UI, same T039/T061/T062 pattern.
3. Chunked upload via Dropbox's upload-session API
   (`upload_session/start` → `append_v2` → `finish`) for large files.
4. Honest empty/error states, same discipline as prior siblings.
5. Dropbox app permission type (App folder vs. Full Dropbox access) —
   decide v1 scope and state why; App folder is the safer default for a
   third-party desktop client.

## Design gate

Before large implementation: short note — what's reused from T060
directly, what Dropbox-specific auth/endpoint differences exist from
T061/T062 (RPC vs REST shape affects the HTTP client code, not just
scopes).

## Done when

1. Live proof: connect a real Dropbox account, list/upload/download real
   files, byte-identical roundtrip.
2. `class=chronos-fm` grims with real Dropbox data.
3. Unit tests for path-mapping/API-shape logic + workspace green.
4. Report + architect stamp. No self-ACCEPT.

## Related

T059 (epic) · T060 (auth dependency) · T061/T062 (siblings, NOT code
dependencies)
