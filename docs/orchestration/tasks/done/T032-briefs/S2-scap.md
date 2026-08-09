# Agent S2 — scap: defer или vendor

**Тип:** реализация, шаг 2  
**Старт:** только после note `T032-notes/I1-scap.md`  
**Merge:** после S1 (hot file `Source/Cargo.toml`)

## Цель

По результату I1:

- **DEFER** (ожидаемо): не вендорить; зафиксировать в note/отчёте, что git pin остаётся, фича `screen-capture` нигде не включена; опционально короткий коммит только с комментарием в `Cargo.toml` **или** zero commit + note для R.
- **VENDOR** (если I1 нашёл включение): path-vendor `zed-scap@4afea48` → `Source/scap` или `Source/zed-scap`, workspace path, NOTICE, PATCHES.md.

## Стены

```bash
cargo check --workspace --manifest-path ChronOS/Cargo.toml
cargo check --workspace --manifest-path Chronos-lm/Cargo.toml
cargo test  --workspace --manifest-path Chronos-FM/Cargo.toml
cargo build -p gpui-component-story --manifest-path Source/gpui-component/Cargo.toml
```

- Не трогать font-kit / xim / wgpu / reqwest (после S1).
- Worktree sibling of repo, не `/tmp`, если S1 ещё не смержен.

## Выход

- `T032-notes/S2-scap.md` — решение DEFER|VENDOR + evidence
- при VENDOR: коммит + baselines green

## Приёмка

- [ ] решение явно названо
- [ ] если DEFER — `cargo tree -i zed-scap` пуст на consumers (или documented)
- [ ] если VENDOR — path + NOTICE + 4 baselines
