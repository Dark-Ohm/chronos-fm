# T042 — Epic: smart pixel-copy of product mockups (Phase V)

**Priority:** P0 program. **Role:** index only — work lives in children.

## Strategy

| Phase | Meaning |
|-------|---------|
| **V** | Visual/IA = mockup; real data only; missing API → empty/disabled honestly |
| **F** | Function fill (git remotes polish, S3 transfers live, plugin host…) after V ACCEPT |

Mockup = product vision. Live догоняет. Icons: `crates/chronos-fm-ui/assets/icons/`.  
Source truth: `/home/neo/projects/chronos-ecosystem/Source`. Facts only. Vision for visual ACCEPT.

## Children

### Phase V pages

| Ticket | Surface | Mockup | Status |
|--------|---------|--------|--------|
| **T037** | Explorer shell | `chronos-file-manager.dc.html` | **done ACCEPT** |
| **T038** | Git | `Chronos-Git-Tab.dc.html` | **PARTIAL-ACCEPT** — residual: Changes/Branches/Stashes/Remotes grims |
| **T039** | S3 | `Chronos-S3-Tab.dc.html` | **PARTIAL-ACCEPT** — residual: sub-view grims + RustFS |
| **T040** | Settings | `Chronos-File-Manager-Settings.dc.html` | **done ACCEPT** (Phase V) |
| **T041** | Extensions | `Chronos-Extensions-Tab.dc.html` | **done ACCEPT** (Phase V) |

### Residuals / tooling

| Ticket | Issue | Status |
|--------|-------|--------|
| **T043** | Places empty / text missing | **done** |
| **T044** | Listing not in tree / empty pane | **done** |
| **T045** | `snapshot_memo` sibling poison | **done ACCEPT** (Source fix) |
| **T046** | Shared visual proof (`--page=` / sub) | **done ACCEPT** (hypr residual waived) |
| **T047** | Git History empty on >4KB log | **done ACCEPT** |

## Order

1. ~~T037 / T043–T045~~ **done**
2. ~~T040 / T041~~ Phase V **done ACCEPT**
3. **T038** finish sub-view vision grims → full V ACCEPT
4. **T039** sub-view grims + optional RustFS → full V ACCEPT (or waiver)
5. Phase F per page after V
6. Tooling: `script/dev/t046_page_smoke.sh` + `--page=page:sub`

## Non-negotiables

- Source edits only better + what/why/зачем
- `class=chronos-fm` grims only
- No secrets in artifacts
- Executors do not self-ACCEPT

## Done when (epic)

All five page children have full Phase V ACCEPT (vision) or PARTIAL + residual closed/waived. Phase F separate.
