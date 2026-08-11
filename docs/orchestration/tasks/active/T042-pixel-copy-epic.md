# T042 — Epic: smart pixel-copy of product mockups (Phase V)

> ## ⚖️ ARCHITECT (2026-08-11): **EPIC COMPLETE** — all five Phase V page
> children full ACCEPT (T039 closed today via live MinIO run; previously
> PARTIAL for the NoProfiles gate). Done-when met. Index only — Phase F
> backend work lives in its own tickets.

**Priority:** P0 program. **Role:** index only — work lives in children.

## Strategy

| Phase | Meaning |
|-------|---------|
| **V** | Visual/IA = mockup; real data only; missing API → empty/disabled honestly |
| **F** | Function fill after V ACCEPT |

Mockup = product vision. Icons: `crates/chronos-fm-ui/assets/icons/`.  
Source: `/home/neo/projects/chronos-ecosystem/Source`.

## Children

### Phase V pages

| Ticket | Surface | Status |
|--------|---------|--------|
| **T037** | Explorer shell | **done ACCEPT** |
| **T038** | Git | **done ACCEPT** (all 5 sub-views grimed on real repo) |
| **T039** | S3 | **done ACCEPT** (live MinIO: 4 sub-views grimed on real profile, residuals 1+2 closed) |
| **T040** | Settings | **done ACCEPT** |
| **T041** | Extensions | **done ACCEPT** |

### Residuals / tooling

| Ticket | Status |
|--------|--------|
| **T043–T045** | **done** (Places / listing / snapshot_memo) |
| **T046** | **done ACCEPT** (`--page=` + sub; hypr waived) |
| **T047** | **done ACCEPT** (History 256KB) |

## Order

1. ~~T037–T041, T043–T047~~ **done** — all five Phase V children full ACCEPT
2. Phase F backends
3. Tooling: `script/dev/t046_page_smoke.sh` (for Git populated: clean `state.redb` or repo path)

## Non-negotiables

- Source edits only better + what/why/зачем
- `class=chronos-fm` grims only
- No secrets · executors do not self-ACCEPT

## Done when (epic)

All five page children full Phase V ACCEPT. ✅ **MET 2026-08-11** (T039
last closed).

## Parallel program

**T048 Explorer essentials** (T049–T056) is a separate P0 track — daily FM
parity (keys, marquee select, DnD). **Current:** T052 external DnD (next).

