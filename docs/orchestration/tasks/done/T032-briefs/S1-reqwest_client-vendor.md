# Agent S1 — vendor: reqwest_client (+ http_client_tls)

**Тип:** реализация, шаг 1  
**Приоритет:** P0 — первый writer  
**Параллельно с:** I1–I4 (они read-only). **Не** параллельно с S2–S4 на том же tree.

## Цель

Убрать git-dep `reqwest_client` из Source и gpui-component; path members.

## WIP уже на диске (не копировать заново)

Untracked в `Source/`:

- `reqwest_client/` — src verbatim zed@876ec5a, Cargo.toml, PATCHES.md, LICENSE-APACHE
- `http_client_tls/` — то же

Root `Cargo.toml` **ещё** указывает:

```toml
reqwest_client = { git = "https://github.com/zed-industries/zed", rev = "876ec5a8..." }
```

## Сделать

1. **Workspace members** — добавить `reqwest_client`, `http_client_tls`.
2. **`[workspace.dependencies]`:**
   - `reqwest_client = { path = "reqwest_client" }`
   - `http_client_tls = { path = "http_client_tls" }`
   - `bytes = "1.0"` (если ещё нет)
   - `reqwest` = zed-reqwest pin (как zed):
     ```toml
     reqwest = { git = "https://github.com/zed-industries/reqwest.git",
       rev = "c15662463bda39148ba154100dd44d3fba5873a4",
       default-features = false,
       features = ["charset","http2","macos-system-configuration","multipart",
                   "rustls-tls-native-roots","socks","stream"],
       package = "zed-reqwest", version = "0.12.15-zed" }
     ```
   - `rustls = { version = "0.23.26" }`
   - `rustls-platform-verifier = "0.5.0"`
3. Убрать git-строку `reqwest_client`; поправить комментарий про util_macros/http_client (**http_client остаётся git** — T030).
4. **`gpui-component/Cargo.toml`:** `reqwest_client = { path = "../reqwest_client" }`; обновить комментарии «нет локального форка».
5. **`Source/NOTICE`** — секции reqwest_client + http_client_tls (Apache-2.0, zed@876ec5a).
6. Проверки:
   ```bash
   cargo check -p reqwest_client --manifest-path Source/Cargo.toml
   cargo check --workspace --manifest-path ChronOS/Cargo.toml
   cargo check --workspace --manifest-path Chronos-lm/Cargo.toml
   cargo test  --workspace --manifest-path Chronos-FM/Cargo.toml
   cargo build -p gpui-component-story --manifest-path Source/gpui-component/Cargo.toml
   cargo tree -i reqwest_client --manifest-path Source/Cargo.toml
   # path, не zed git
   ```
7. **Один коммит** в Source (conventional), без AI trailer.

## Не трогать

- scap, font-kit, xim, wgpu
- `http_client` git pin
- `gpui_macos` / `gpui_windows`

## Приёмка

- [ ] `reqwest_client` → path в tree
- [ ] `http_client_tls` → path (не zed git)
- [ ] 4 consumers green (≥ baseline EXIT:0)
- [ ] story build green
- [ ] NOTICE + PATCHES.md на месте
- [ ] один коммит

## Заметка агента (короткая)

`T032-notes/S1-reqwest_client.md` — команды, exit codes, что осталось на git.
