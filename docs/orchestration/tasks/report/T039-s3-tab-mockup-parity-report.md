# T039 — S3 tab: visual parity with design mockup

> ## ⚖️ ARCHITECT VERDICT: **DESIGN APPROVED (option C) — with gates** (2026-08-09)
>
> Inventory facts OK (5 connect states; embed pane only; no transfer jobs;
> multipart available in SDK; mockup 4 views). **Option C full transfer
> subsystem** approved as product vision — real progress/cancel, not fake
> queue. Whole-object provider paths must stay (T011/T021).
>
> **Open gates before heavy impl:**
> 1. Compile probe **ashpd** FileChooser (or document chosen dialog path).
> 2. Design spec file (like T038) with job state machine + error matrix.
> 3. Secrets never in report/grim.
>
> **Not** implementation ACCEPT. Ticket stays `active/`.


**Status:** DESIGN AGREED (option C) — implementation not started. This is an
inventory + design-approval report, not an implementation/acceptance report.

**Truth bases used:**
- Chronos-FM source: `crates/*`
- GPUI fork source: `/home/neo/projects/chronos-ecosystem/Source`
- Visual authority: `docs/design/mockups/Chronos-S3-Tab.dc.html`
- Ticket: `docs/orchestration/tasks/active/T039-s3-tab-mockup-parity.md`

## Inventory (facts only)

### Current S3 page (`crates/chronos-fm-pages/src/s3.rs`)

- **Claim:** The page has a five-state connect/browse flow and embeds the
  explorer pane after connect.
  **Evidence:** `S3State` enum (lines 22–28: `NoProfiles`, `NeedCredentials`,
  `Connecting`, `Browsing`, `Error { message }`); `S3Page` struct (lines 30–40,
  `s3_pane: Option<Entity<ExplorerPane>>`); `start_connect` (lines 98–195);
  `wire_pane` (lines 207–224); render switch at `content` (lines 250–296);
  card helpers (lines 298–420).
  **Truth base:** Chronos-FM source.

- **Claim:** There is no multi-view shell today — after connect the page
  renders only the embedded pane.
  **Evidence:** `content()` lines 250–296: Browsing/Connecting branch renders
  `page.s3_pane` inside a `div().flex_1().relative()`; no view switcher, no
  bucket/transfer/properties sub-views exist.
  **Truth base:** Chronos-FM source.

### S3 service (`crates/chronos-fm-services/src/s3/`)

- **Claim:** The client already provides bucket/object operations and metadata.
  **Evidence:** `mod.rs` — `list_buckets` (line 80), `list_objects` (line
  123), `get_object` (line 193), `put_object` (line 210), `delete_object`
  (line 230), `head_object` (line 247); path helpers `parse_s3_path`,
  `s3_bucket_path`, `s3_profile_root` (lines 270–306); `provider.rs`
  implements the sync `FileSystemProvider`.
  **Truth base:** Chronos-FM source.

- **Claim:** No transfer/job model exists; current operations are whole-object
  and synchronous (`Vec<u8>` via `block_on`).
  **Evidence:** `get_object` returns `Result<Vec<u8>>` (line 193);
  `put_object` takes `&[u8]` and builds `ByteStream::from(content.to_vec())`
  (line 210–216); every method runs through `self.runtime.block_on`.
  **Truth base:** Chronos-FM source.

- **Claim:** SDK version and multipart API are available.
  **Evidence:** `aws-sdk-s3 = "1"` (`crates/chronos-fm-services/Cargo.toml`),
  locked at `1.140.0` (`Cargo.lock`); local source at
  `~/.cargo/registry/src/index.crates.io-*/aws-sdk-s3-1.140.0` includes
  `create_multipart_upload`, `upload_part`, `complete_multipart_upload`,
  `abort_multipart_upload` operations.
  **Truth base:** Cargo.lock + local registry source.

### Mockup (`docs/design/mockups/Chronos-S3-Tab.dc.html`)

- **Claim:** The mockup specifies a four-view shell with toolbar and transfer
  queue.
  **Evidence:** sidebar nav Explorer / Buckets / Transfers / Properties (lines
  465–470); view headers/subtitles (lines 503–520); Explorer breadcrumb +
  object table + preview pane (lines 541–623); Buckets table (lines 625–638);
  Transfers rows with progress bar and `done`/`queued`/`in progress` labels
  (lines 640–655); Properties cards (lines 657–725).
  **Truth base:** mockup HTML.

## Approved design decisions

1. **Chosen scope: option C — full transfer subsystem.**
   Real upload/download jobs with measurable progress, cooperative
   cancellation, retry, and Clear-finished; not a cosmetic queue.
2. **Transfer engine: chunked S3 engine.**
   `S3Client` gains thin chunked/streaming APIs (part-based download and
   multipart upload) so progress is real and cancellation is cooperative
   between parts. Whole-object `get_object`/`put_object` paths remain for the
   existing provider, avoiding a rewrite of T011/T021 semantics.
3. **Local source/destination: native file dialogs.**
   User picks local files/directories through a system dialog.
   **Open sub-decision:** the project has no established native dialog
   dependency (`rfd` absent; `ashpd` present only transitively via
   `gpui_linux`/`oo7`). Recommended: add a direct `ashpd` dependency
   (xdg-desktop-portal FileChooser); must be verified with a compile probe
   against local `ashpd 0.13.13` sources before spec finalization.
4. **UI:** S3 wrapper shell with Explorer / Buckets / Transfers / Properties
   views, S3-native chrome, honest empty/loading/error states per T023; icons
   only from `crates/chronos-fm-ui/assets/icons/`; palette only
   `chronos_fm_ui::theme`.
5. **Safety:** T011/T021 connect/browse semantics preserved; secrets stay in
   the keyring; no secrets in config/report/screenshots; no fake progress or
   fake transfer state.

## Verification

- **Claim:** Implementation verified.
  **Evidence:** **Not claimed.** No code changes for T039 exist in the working
  tree at the time of this report.
  **Truth base:** none — not started.

- **Claim:** Visual parity proven by grim.
  **Evidence:** **Not claimed.** Requires release binary + grim after
  implementation, reviewed by a vision-capable model against the mockup.
  **Truth base:** T039 "Done when" criteria.

## Next gate

1. Resolve the native dialog sub-decision (compile probe for `ashpd` vs fork
   API) and finalize the T039 design spec.
2. Write implementation plan (tests first: chunked transfer engine, job
   state machine, parser helpers).
3. Implement service + UI, run `cargo fmt --check`, targeted service/page
   tests, `cargo check -p chronos-fm-pages`, release build
   `cargo build --release -p chronos-fm`.
4. Live grim pack (NoProfiles, Browsing, Buckets, Transfers, Properties) +
   vision review; no secrets in artifacts.
5. Move this report to `report-log/` only after acceptance.
