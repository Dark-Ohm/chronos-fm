[38;5;8m   1[0m [37m# T047 — Git History silently empty when `git log` exceeds 4KB truncate[0m
[38;5;8m   2[0m 
[38;5;8m   3[0m [37m**Priority:** P1 — undermines T038 History on any non-trivial repo.[0m
[38;5;8m   4[0m [37m**Source:** T046 sub-view grim `git:history` on Chronos-FM (174 commits).[0m
[38;5;8m   5[0m [37m**Architect:** filed 2026-08-10 after RC confirmed.[0m
[38;5;8m   6[0m 
[38;5;8m   7[0m [37m## Symptom[0m
[38;5;8m   8[0m 
[38;5;8m   9[0m [37mHistory view shows empty / “no commits” for real repos while[0m
[38;5;8m  10[0m [37m`git log -50` has plenty of commits.[0m
[38;5;8m  11[0m 
[38;5;8m  12[0m [37m## Root cause (confirmed)[0m
[38;5;8m  13[0m 
[38;5;8m  14[0m [37m1. `git::history()` uses `run_git_cmd` → `truncate_4k` (cap 4096 on[0m
[38;5;8m  15[0m [37m   stdout+stderr).[0m
[38;5;8m  16[0m [37m2. 50-commit pretty format on this repo is **~8880 bytes** > 4KB → last[0m
[38;5;8m  17[0m [37m   record cut mid-field.[0m
[38;5;8m  18[0m [37m3. `parse_commit_fields` fails exact-7-field check → `Err`.[0m
[38;5;8m  19[0m [37m4. `do_refresh` does `history(...).unwrap_or_default()` → false empty UI.[0m
[38;5;8m  20[0m 
[38;5;8m  21[0m [37mSame cap may affect large `commit_detail` diffs — check when fixing.[0m
[38;5;8m  22[0m 
[38;5;8m  23[0m [37m## Done when[0m
[38;5;8m  24[0m 
[38;5;8m  25[0m [37m1. Design choice implemented (architect preference):[0m
[38;5;8m  26[0m [37m   - **Preferred:** history-specific path without 4KB silence — e.g. larger[0m
[38;5;8m  27[0m [37m     dedicated cap (e.g. 256KB) **or** stream/parse line-by-line and stop[0m
[38;5;8m  28[0m [37m     cleanly at limit without poison parse; surface truncated-as-error only[0m
[38;5;8m  29[0m [37m     if zero complete records.[0m
[38;5;8m  30[0m [37m   - **Minimum:** do not map parse/`Operation` errors to empty list — show[0m
[38;5;8m  31[0m [37m     real error string in Git page error state.[0m
[38;5;8m  32[0m [37m2. Unit/integration test: history fixture or synthetic output > 4KB still[0m
[38;5;8m  33[0m [37m   returns some commits (or honest error, never silent empty).[0m
[38;5;8m  34[0m [37m3. Live grim: `--page=git:history` in Chronos-FM repo shows ≥1 commit.[0m
[38;5;8m  35[0m 
[38;5;8m  36[0m [37m## Walls[0m
[38;5;8m  37[0m 
[38;5;8m  38[0m [37m- Do not invent commits. Do not remove all output bounds without a limit.[0m
[38;5;8m  39[0m [37m- Secrets N/A. Facts only.[0m
[38;5;8m  40[0m 
[38;5;8m  41[0m [37m## Related[0m
[38;5;8m  42[0m 
[38;5;8m  43[0m [37mT038 PARTIAL · T046 `--page=page:sub` · report[0m
[38;5;8m  44[0m [37m`report/T046-subview-flag-and-history-bug-report.md`[0m
