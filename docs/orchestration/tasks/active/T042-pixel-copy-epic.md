# T042 — Epic: smart pixel-copy of product mockups (Phase V)

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
| **T039** | S3 | **PARTIAL-ACCEPT** — NoProfiles gate hides 4-view chrome; need profile path |
| **T040** | Settings | **done ACCEPT** |
| **T041** | Extensions | **done ACCEPT** |

### Residuals / tooling

| Ticket | Status |
|--------|--------|
| **T043–T045** | **done** (Places / listing / snapshot_memo) |
| **T046** | **done ACCEPT** (`--page=` + sub; hypr waived) |
| **T047** | **done ACCEPT** (History 256KB) |

## Order

1. ~~T037–T038, T040–T041, T043–T047~~ **done**
2. **T039** finish Phase V vision (profile + sub-views) → full V ACCEPT
3. Phase F backends
4. Tooling: `script/dev/t046_page_smoke.sh` (for Git populated: clean `state.redb` or repo path)

## Non-negotiables

- Source edits only better + what/why/зачем
- `class=chronos-fm` grims only
- No secrets · executors do not self-ACCEPT

## Done when (epic)

All five page children full Phase V ACCEPT. T039 last open page child.
