# Agent I0 — inventory: reqwest_client (+ http_client_tls)

**Тип:** read-only разведка  
**Параллельно:** да  
**Приоритет:** низкий (почти закрыт WIP S1; запускать только если нужна formal note)

## Задача

Зафиксировать граф, фичи и обоснование вендоринга `reqwest_client` / `http_client_tls`.

## Скоуп (только читать)

- `Source/Cargo.toml` (~строки 151–152, workspace deps)
- `Source/gpui/Cargo.toml` — dev-dep `reqwest_client`
- `Source/gpui-component/Cargo.toml` — pin `reqwest_client`
- zed cache: `~/.cargo/git/checkouts/zed-*/876ec5a/crates/reqwest_client`
- zed: `.../crates/http_client_tls`
- WIP (уже на диске, untracked): `Source/reqwest_client/`, `Source/http_client_tls/`

## Команды

```bash
cd Source
cargo tree -i reqwest_client --target all -e normal,build,dev
cargo tree -i http_client_tls --target all -e normal,build,dev
rg -n 'reqwest_client|ReqwestClient' --glob '**/*.{rs,toml}'
```

## Уже известные факты (проверить, не выдумывать заново)

- dev-only для `gpui` (+ examples + gpui-component story)
- deps: `http_client` (git zed), `http_client_tls`, `zed-reqwest` git, local `gpui_util`
- src ~323 LOC; `http_client_tls` ~21 LOC

## Вне скоупа

- Проводка в workspace, коммиты, правки `Cargo.toml` → это **S1**
- Не трогать scap / font-kit / xim / wgpu

## Выход

Один файл:

`Chronos-FM/docs/orchestration/tasks/active/T032-notes/I0-reqwest_client.md`

Структура: потребители · prod vs dev · риски · рекомендация vendor (да) · что остаётся на git (`http_client`, `zed-reqwest`).

## Definition of done

- note ≤ ~40 строк, с командами и путями
- zero git changes in Source
