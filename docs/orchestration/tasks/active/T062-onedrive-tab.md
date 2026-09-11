# T062 — OneDrive tab

**Epic:** T059. **Priority:** P2.
**Depends:** T060 (OAuth foundation) — blocked until T060 ACCEPT.
Independent of T061 (Google Drive) — order between T061/T062/T063 is by
priority, not a hard dependency chain.
**Code:** `crates/chronos-fm-services/src/onedrive/`,
`crates/chronos-fm-pages/src/onedrive.rs` (T039/T061 shape).

## Problem

Second cloud-provider tab. Microsoft Graph API differs from Drive/S3 in
auth app registration (Azure AD, not Google Cloud Console) and API shape
— do not assume T061's code transfers 1:1; confirm each difference before
copying.

## Must

1. `FileSystemProvider` impl against Microsoft Graph
   (`/me/drive/root:/{path}:`) — Graph's path-addressable API is closer to
   S3's key-shaped model than Drive's file-ID model; note this difference
   from T061 explicitly rather than assuming the same mapping code
   applies.
2. Tab UI, same T039/T061 pattern (sub-nav Explorer / Transfers /
   Properties).
3. Chunked/resumable upload via Graph's upload-session API for large
   files.
4. Honest empty/error states, same discipline as T061/T039.
5. Azure AD app registration: multi-tenant vs. single-tenant — decide and
   state why (affects which Microsoft accounts can connect).

## Design gate

Before large implementation: short note — what's reused from T060/T061
directly, what Graph-specific auth scope/endpoint differences exist
(`Files.ReadWrite` scope, `/me/drive` vs `/drives/{id}` for shared
drives — decide v1 scope, personal-only vs. org/shared drives).

## Done when

1. Live proof: connect a real Microsoft account, list/upload/download
   real files, byte-identical roundtrip.
2. `class=chronos-fm` grims with real OneDrive data.
3. Unit tests for path-mapping logic + workspace green.
4. Report + architect stamp. No self-ACCEPT.

## Related

T059 (epic) · T060 (auth dependency) · T061 (sibling, NOT a code
dependency — differences must be verified, not assumed)
