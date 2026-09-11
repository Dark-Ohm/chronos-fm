# T059 — Epic: Cloud storage providers (Google Drive / OneDrive / Dropbox)

**Priority:** P2 program. **Role:** index only — work lives in children.
**Orthogonal to:** T042 (pixel-copy), T048 (explorer essentials).
**Not** the plugin system — see "Why not extensions" below.

## Why

S3 (T011→T039) proved the pattern: `FileSystemProvider` trait
(`crates/chronos-fm-services/src/fs/provider.rs`) lets Explorer list/read/
write/delete/rename through any backend uniformly. Google Drive / OneDrive /
Dropbox are the next natural backends users expect next to S3 in a
Finder-replacement file manager.

## Why not "as extensions"

Checked before writing this ticket, not assumed:

- `chronos-fm-plugin-host` is **commented out** of Cargo workspace members
  (`Cargo.toml:14`) — no plugin runtime exists to host anything in.
- Extensions tab (T012/T041) is an honest UI mockup only: Available/
  Marketplace state is empty by design, nothing executes.
- The real plugin host is ROADMAP Phase P4 (`0.3.0`), not started, and per
  ADR 0009 the engine changed from WASM to Luau — a different foundation
  than what "write a Drive plugin" would have assumed a year ago.

Conclusion: these ship as **native Rust tabs**, same architectural class as
S3 (T039), not as sandboxed third-party plugins. If P4 lands later, a
thin plugin-facing adapter over the same `FileSystemProvider` trait could
follow — not blocking this epic.

## The new wall none of S3/T039 crossed: OAuth2

S3 authenticates with static access/secret keys via keyring
(`docs/persistence.md`). None of the three targets support that:

| Provider | Auth | API |
|---|---|---|
| Google Drive | OAuth2, Google Cloud Console app | Drive API v3 |
| OneDrive | OAuth2, Azure AD app registration | Microsoft Graph API |
| Dropbox | OAuth2, App Console app | Dropbox API v2 |

No prior ticket in this repo implemented a browser/device-code OAuth
redirect + refresh-token storage. This is genuinely new work, not a copy
of `s3.rs`'s credential form — named here so no child ticket discovers it
as a surprise mid-implementation (`writing-briefs-name-the-wall`).

## Children

| ID | Title | Pri | Status |
|----|-------|-----|--------|
| **T060** | OAuth2 provider-auth foundation (spike, shared by all three) | P2 | open — **next** |
| **T061** | Google Drive tab | P2 | open — blocked on T060 |
| **T062** | OneDrive tab | P2 | open — blocked on T060 |
| **T063** | Dropbox tab | P2 | open — blocked on T060 |

## Order now

1. **T060** — OAuth2 spike: pick a flow (device-code preferred for a
   desktop app — no embedded webview/redirect-server dependency; research
   first, no invented APIs), one working end-to-end token acquisition +
   keyring storage + refresh, proven against **one** provider (Google,
   most-requested). Design-gate note before wiring the other two: what the
   flow needs from each provider's OAuth app config, what's shared vs.
   provider-specific.
2. **T061** Google Drive tab (reuses T060's flow directly)
3. **T062** OneDrive tab (reuses T060's pattern, Microsoft Graph specifics)
4. **T063** Dropbox tab (reuses T060's pattern, Dropbox API specifics)

Each of T061–T063 follows the T039 shape: `FileSystemProvider` impl +
sub-nav tab (Explorer/Transfers/Properties at minimum — no Buckets
equivalent, these are single-root not multi-bucket) + chunked
upload/download on the S3 transfer-engine pattern where the target API
supports resumable/chunked transfer (all three do).

## Non-negotiables

- No secrets committed; tokens live in keyring only, never in config or
  reports (same rule as T039's env-credential seam).
- Design gate before large per-provider work: short note — what the API
  needs, what's shared with T060, what's provider-specific (mirrors T052's
  gate).
- `class=chronos-fm` grims only for visual proof; real account/data,
  never fabricated screenshots.
- Executors do not self-ACCEPT. No push without user ask.

## Done when (epic)

T060 ACCEPT (shared OAuth foundation proven against one real account) and
at least one of T061–T063 full ACCEPT with live evidence. Remaining two
providers ACCEPT or explicitly deferred with a filed residual — this is a
P2 program, not required to land as one atomic unit.

## Related

T042 (pixel-copy pattern precedent) · T039 (S3, closest sibling
implementation) · `docs/persistence.md` (keyring/credential storage) ·
ROADMAP Phase P4 (plugin host, not a dependency of this epic)
