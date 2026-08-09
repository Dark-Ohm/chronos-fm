# S2 — scap: DEFER (не вендорим)

**Решение:** **DEFER** — git pin остаётся, `screen-capture` нигде не включён.
**Статус:** DONE — коммит `3a0fe17` в Source (комментарий в Cargo.toml), tree clean.
**Основание:** note `T032-notes/I1-scap.md` (решение I1: DEFER, условие VENDOR — включение `screen-capture` любым потребителем).

## Evidence (перепроверено 2026-08-09, после S1)

```
$ cargo tree -i zed-scap            # Source/          -> warning: nothing to print
$ cargo tree -i zed-scap            # Source/gpui-component -> nothing to print
$ cargo tree -i zed-scap --manifest-path ChronOS/Cargo.toml    -> nothing to print
$ cargo tree -i zed-scap --manifest-path Chronos-lm/Cargo.toml -> nothing to print
$ cargo tree -i zed-scap --manifest-path Chronos-FM/Cargo.toml -> error: did not match any packages
```

Chronos-FM даже не содержит `zed-scap` в resolve-графе. Единственные упоминания
`scap`/`screen-capture` у потребителей — `ChronOS/reference/kael-main/`
(архивная reference-копия Kael, не member workspace) — не потребитель.

`gpui` default = `font-kit, wayland, x11, windows-manifest` (без `screen-capture`);
`scap` — optional-зависимость под off-by-default feature.

## Стены (baselines)

| команда | exit |
|---|---|
| `cargo check --workspace` (ChronOS) | 0 |
| `cargo check --workspace` (Chronos-lm) | 0 |
| `cargo test --workspace` (Chronos-FM) | 0 (34 passed, 0 failed) |
| `cargo build -p gpui-component-story` | 0 |

## Что сделано

- `Source/Cargo.toml`: комментарий у git-pin `scap` — DEFER-обоснование
  (feature off, ничего не тянется, vendor при включении `screen-capture`).
- Больше ничего: graph/lockfile не тронуты, код не менялся.

## Условие перехода на VENDOR

Как только любой потребитель включит `screen-capture` — вендорить
`zed-scap@4afea48` в `Source/` (path workspace member, NOTICE, PATCHES.md)
по схеме S1.
