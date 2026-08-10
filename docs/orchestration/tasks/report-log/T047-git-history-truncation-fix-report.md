# T047 — Git History silently empty on truncate — Fix report

> ## ✅ ARCHITECT VERDICT: **ACCEPT / CLOSED** (2026-08-10)
>
> Both preferred + minimum shapes landed and verified:
> - `run_git_cmd_capped` + `GIT_LOG_OUTPUT_CAP` 256KB for `history` /
>   `commit_detail`; generic callers stay at 4KB
> - `parse_history` drops only trailing mangled record; non-trailing /
>   all-fail still Err
> - page `do_refresh` surfaces history Err on `page.error`, not empty list
>
> Architect re-ran `git::` **46/46**. Live grim
> `report-log/T047-shots/t047_git_history_fixed.png`: **History · 50 commits**
> with real SHAs on Chronos-FM repo (vision OK). Honest residual: no e2e
> async fault-injection for all-fail→error path (unit-covered).
>
> Unblocks honest T038 History claim. Ticket → `done/`.


**Status:** IMPLEMENTED, NOT ACCEPTED — not claimed done. Per session
policy the executor does not accept its own work.
**Date:** 2026-08-10
**Executor:** Claude (Sonnet 5)
**Follows:** T047 ticket, architect preference: "1. Preferred: history-
specific path (больший cap или parse complete records only, stop
cleanly). 2. Minimum: never map real Err → empty list — show error
string." Implemented **both**, not just the minimum.

## Root cause (recap, confirmed in the T046 report that filed this ticket)

`git::history()` shelled out via `run_git_cmd`, capped at 4096 bytes.
`git log -50` on this repo's real format string is ~8.8KB — over the cap
— so the last record came back cut mid-field. `parse_commit_fields`'s
exact-7-field check then failed on that one mangled record, `parse_history`
propagated that as an `Err` for the **entire** call, and `do_refresh`'s
`git::history(&repo, HISTORY_LIMIT).unwrap_or_default()` silently turned a
real parse error into an empty `Vec` — rendering as "no commits yet" for a
174-commit repo.

## Fix — both preferred and minimum

### 1. History-specific output path (preferred, §1 of the ticket)

- `run_git_cmd_capped(workdir, args, cap)` — the same command-running
  logic as `run_git_cmd`, parameterized by cap instead of hardcoding 4096.
  `run_git_cmd` is now a thin wrapper (`run_git_cmd_capped(..., 4096)`) —
  every other call site's behavior is unchanged.
- `GIT_LOG_OUTPUT_CAP = 256 * 1024` (256KB) — `history()` and
  `commit_detail()` (the ticket's "check `commit_detail` too") both use
  this instead of the generic 4KB cap. 256KB comfortably covers
  `HISTORY_LIMIT` (50) commits even with long subject lines, or one
  commit's full `--numstat` file list.
- `parse_history` rewritten to be tolerant of a truncated **trailing**
  record specifically (the only place a cap-cut can land, by
  construction): if the last record fails to parse, it's dropped (logged
  at `debug`) and the call still returns the complete records that came
  before it. A malformed record anywhere **other** than last is still a
  hard `Err` — that's real corruption or a format mismatch, not a
  truncation artifact, and silently dropping it would be exactly the kind
  of under-reporting T047 is about. If **every** record fails (not just
  the tail), the call still errors — an all-garbage result must never
  look like an honestly empty history.

### 2. Never mask a real error as empty (minimum, §2 of the ticket)

`do_refresh` in `chronos-fm-pages/src/git.rs` kept `history` as a
`Result<Vec<CommitEntry>, GitError>` all the way to the page instead of
`.unwrap_or_default()`-ing it in the background task. On `Err`, the page
now sets `page.error = Some("git history: {err}")` and clears `history` —
a visible error, not a false empty state. `status`/`branches`/`stashes`/
`remotes` still apply from the same refresh even if history specifically
fails (independent fields, no reason to lose everything over one).

This is defense in depth: with the 256KB cap and trailing-record leniency
above, hitting the "every record failed" error path should now be rare —
but if it ever happens (or a future regression reintroduces it), it will
show as a real error message, not silently look like an empty repo again.

## Tests (TDD — written before/alongside the fix)

```
cargo test -p chronos-fm-services git::
running 46 tests ... test result: ok. 46 passed; 0 failed
```

4 new:
- `parse_history_drops_a_malformed_trailing_record_but_keeps_the_rest` —
  simulates the exact bug shape (mangled last record), asserts the good
  record still comes back.
- `parse_history_errors_when_a_non_trailing_record_is_malformed` — the
  leniency is scoped to the last record only; a malformed record earlier
  in the stream is still a hard error.
- `history_survives_output_past_the_old_4kb_cap` — builds a real repo
  with 40 long-subject commits (past the confirmed ~8.8KB/50-commit
  cliff) and asserts `history()` returns all of them, not an empty list.
- `parse_history_malformed_record_is_an_error_not_invented_data`
  (pre-existing) — re-verified it still holds: a single, wholly malformed
  record still errors (zero parsed records after dropping the "trailing"
  one, which is the whole input here, hits the "none parsed successfully"
  branch) — T047's leniency doesn't reopen T038's original "don't invent
  data" guarantee.

```
cargo test -p chronos-fm-pages git::            → 6 passed; 0 failed
cargo test -p chronos-fm-services -p chronos-fm-pages → 240 passed; 0 failed
cargo test --workspace --no-fail-fast           → 0 failed anywhere
cargo build --release -p chronos-fm             → clean
```

## Live grim — the ticket's own "Done when" #3

`script/dev/t046_page_smoke.sh 'git:history' /tmp/t047_git_history_fixed.png`
against the release binary, this repo (174+ real commits at the time):
**History now shows "50 commits"** with real entries (`fdad9db`,
`c1e9144`, `7e9f94f`, `19efd36`, …, author `dark-ohm`, real relative
timestamps, `main` tag on HEAD) — not "no commits yet". `GRIM_OK`, zero
log errors.

## What's not done

- Did not add an integration test that forces the "every record fails"
  Err path end-to-end through `do_refresh`/`page.error` — the unit-level
  `parse_history` test covers the parsing side; the page-level wiring
  (`history_result` → `page.error`) is exercised by inspection and the
  existing `do_refresh` test suite (6/6 green, unchanged) rather than a
  new dedicated test forcing a synthetic history failure through the full
  async path. Would need a fault-injection seam that doesn't exist yet.
- `commit_detail`'s cap bump is applied but not independently tested with
  a large fixture (the ticket said "check", not "must add a test") —
  its existing parse loop already degrades gracefully (skips malformed
  trailing numstat lines rather than erroring), so the risk there was
  lower to begin with; flagging the gap rather than silently calling it
  fully covered.
