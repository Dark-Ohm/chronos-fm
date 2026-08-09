# T036 — Fork: story-web WASM needs nightly (`wasm_thread` feature)

**Приоритет:** P3 — WASM gallery is not on the daily FM path.
**Скоуп:** toolchain + `Source/gpui-component` story-web (no forced rustup
install without architect go).
**Источник:** T028 PARTIAL-ACCEPT (2026-08-09), Wall 2.2.

## Symptom

`scripts/build-wasm.sh` fails on stable Rust 1.97.1:

```
wasm_thread-0.3.3: #![feature(stdarch_wasm_atomic_wait)]
error[E0554]: `#![feature]` may not be used on the stable release channel
```

Infra ready otherwise: wasm32 std, wasm-bindgen 0.2.121, bun. **rustup absent.**

## Decision needed before code

A. Install rustup + nightly **only for wasm** (document PATH isolation so the
   five-project stable default is unchanged), then finish T028 part 2 steps 3–5.
B. Find/await a `wasm_thread` (or dep graph) that builds on stable.
C. Explicitly defer story-web WASM until a ChronOS/FM product need appears.

Architect picks A/B/C when the ticket is scheduled. Do not install rustup as a
side effect of an unrelated task.

## Done when

Chosen path executed; either `story-web` builds and a browser frame is captured,
or DECISIONS.log records explicit deferral (path C).
