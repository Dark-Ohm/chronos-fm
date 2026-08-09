# S1 — vendor reqwest_client (+ http_client_tls)

**Статус:** DONE — коммит `c0c4f23` в Source, working tree clean.

## Что сделано

- `Source/reqwest_client/`, `Source/http_client_tls/` — vendored zed@876ec5a
  (src verbatim, manifest переписан; PATCHES.md + LICENSE-APACHE на месте).
- `Source/Cargo.toml`: оба в `members`; `[workspace.dependencies]` →
  `{ path = ... }`; git-строка `reqwest_client` удалена; добавлены `bytes = "1.0"`,
  `reqwest` (zed-reqwest pin c1566246), `rustls = "0.23.26"`,
  `rustls-platform-verifier = "0.5.0"`. Комментарий `util_macros/http_client/
  reqwest_client` → `util_macros/http_client`.
- `Source/gpui-component/Cargo.toml`: `reqwest_client = { path = "../reqwest_client" }`;
  оба комментария «нет локального форка» обновлены.
- `Source/NOTICE`: секции reqwest_client + http_client_tls (Apache-2.0, zed@876ec5a).
- Оба Cargo.lock: git-source → path (source-строка удалена).

## Команды и exit codes

| команда | exit |
|---|---|
| `cargo check -p reqwest_client` (Source) | 0 |
| `cargo check --workspace` (ChronOS) | 0 |
| `cargo check --workspace` (Chronos-lm) | 0 |
| `cargo test --workspace` (Chronos-FM) | 0 |
| `cargo build -p gpui-component-story` (Source/gpui-component) | 0 |
| `cargo tree -i reqwest_client` (Source) | 0 — `(.../Source/reqwest_client)`, path, не git |

## Осталось на git

- `http_client` (zed@876ec5a) — T030.
- `util_macros` (zed@876ec5a).
- `reqwest` / `zed-reqwest` git fork (c1566246).
- `gpui_macos` / `gpui_windows` (cfg-gated, не тянутся на Linux).
- scap / font-kit / xim / wgpu — вне скоупа S1 (S2–S5).
