# Testing & Quality Infrastructure

> Status: Draft (P1 で基盤構築、後続 Phase で拡充)
> Related: [`ROADMAP.md`](./ROADMAP.md), [`docs/dev-environment.md`](./dev-environment.md)

本書はテスト方針 / カバレッジパイプライン / 静的解析のセットを定めます。

---

## 1. テスト層構成

| 層 | 配置 | 用途 |
|----|------|------|
| **ユニットテスト** | 各 crate 内の `#[cfg(test)] mod tests` | 関数単位の検証 |
| **GPUI テスト** | 各 crate 内、`TestAppContext` を使う | view state / async / cx interaction の検証 |
| **統合テスト** | `crates/<crate>/tests/` | crate 跨ぎでない、対外的なシナリオ |
| **end-to-end** | (P5 以降検討) | UI レベルの自動操作。`script/ui-run.sh` の延長 |

ワークスペース root の `tests/` ディレクトリは **撤廃** します (現在の `tests/indexing_test.rs` / `tests/watcher_test.rs` は P1 で削除予定。検索リアーキの一環)。

### GPUI テストの基本パターン

CLAUDE.md のルール (重要):

```rust
#[gpui::test]
async fn test_my_view(cx: &mut TestAppContext) {
    // window-bound な sub-entity (InputState 等) を持つ view は test window 内で構築する。
    let window = cx.add_window(|window, cx| MyView::new(window, cx));
    window.update(cx, |view, window, cx| view.start_async_work(window, cx)).unwrap();
    // 必ず GPUI executor の timer を使う
    cx.background_executor.timer(Duration::from_millis(50)).await;
    cx.run_until_parked();
    window.read_with(cx, |view, _cx| {
        assert_eq!(view.something, expected);
    }).unwrap();
}
```

gpui crate は dev-dependency で `gpui = { version = "0.2", features = ["test-support"] }` を
有効にする (`TestAppContext` / `#[gpui::test]` マクロが利用可能になる)。実例は
[`crates/nohrs-pages/src/explorer/tests.rs`](../crates/nohrs-pages/src/explorer/tests.rs)。

**避けるべきパターン**: `smol::Timer::after` / `tokio::time::sleep`。これらは GPUI scheduler が tracking しないため `run_until_parked()` が早期に "nothing left" を返す。

### snapshot / property

| 技術 | 用途 |
|------|------|
| `insta` | 検索パーサ / config パーサ / WIT bindings 等の出力固定 |
| `proptest` | 検索クエリ正規化、permission ガード等で限定的に。**過剰投入しない** |
| `tempfile` | 実ディレクトリ作成型のテスト |

---

## 2. カバレッジパイプライン

`cargo-llvm-cov` で計測し、GitHub の native code coverage (PR diff inline) と、ダウンロード可能な
HTML レポート artifact を **併用**。外部 SaaS (Codecov / Coveralls) は **採用しない**。

### CI ワークフロー (`.github/workflows/ci.yml` の `coverage-*` ジョブ)

`test` ジョブと同じ split に倣い、2 つの coverage ジョブで計測する:

| job (tier) | runner | コマンド | 範囲 | gate |
|------------|--------|----------|------|------|
| **coverage-core** | `ubuntu-latest` | `cargo llvm-cov -p nohrs-core --locked --cobertura ...` | 基盤 crate `nohrs-core` | `--fail-under-lines 80` |
| **coverage-overall** | `macos-latest` | `cargo llvm-cov --workspace --all-features --locked --cobertura ...` | gpui を含む全 crate (GUI) | `--fail-under-lines 50` |

- 「core」は基盤 crate `nohrs-core` を指す。`nohrs-models` / `nohrs-services` は overall tier
  (全 workspace) 側でカバーする (search backend 等の外部プロセス依存コードは単体テスト困難なため、
  default-members 全体を 80% gate にはしない)。
- macOS 側は gpui を Metal バックエンドで build するため、Linux のような system library
  (`libxcb` / `wayland` / `vulkan` 等) の導入は不要。`#[gpui::test]` は両 OS で `TestAppContext`
  により headless に実行される。
- 各ジョブが生成する Cobertura XML を GitHub native code coverage に tier 別ラベル
  (`code-coverage/core` / `code-coverage/overall`) で upload し、HTML レポートを
  `coverage-html-core` / `coverage-html-overall` artifact として upload する。
- matrix は **使わない**: matrix leg は単一の `outputs` を共有し、GitHub は最後に完了した leg の
  値だけを残す (last-writer-wins) ため、両 tier の rate を確実に下流へ渡せない。代わりに 2 つの
  独立ジョブがそれぞれ安定した `rate` output を公開する。
- **閾値は最終ステップで強制**する: rate 抽出と各種 upload の **後** に `cargo llvm-cov report
  --fail-under-lines <N>` を実行するので、gate を割っても PR コメントと HTML artifact は投稿される。
- 後続の `coverage-report` ジョブ (`always()` で実行) が両ジョブの `rate` output をまとめ、PR に
  **1 つ**のコメントで core / overall を目標値・達成可否 (✅/❌) と並べて表示する。

### 閾値 (gate)

カバレッジは **enforced gate**: `coverage-core` が `nohrs-core` の line coverage 80% 未満、または
`coverage-overall` が workspace 全体で 50% 未満になると、当該ジョブが fail し CI を落とす。

| tier | 閾値 | 動作 |
|------|------|------|
| core (`nohrs-core`) | line ≥ **80%** | 未達で CI fail |
| overall (`--workspace --all-features`) | line ≥ **50%** | 未達で CI fail |

`#[gpui::test]` を含むテスト基盤が揃い目標値を満たしたため、当初の informational 運用から enforced
gate へ移行した。

---

## 3. 静的解析・ルール定義

| ツール | 用途 | CI |
|--------|------|-----|
| `cargo fmt --check` | フォーマット | required |
| `cargo clippy -- -D warnings -W clippy::unwrap_used -W clippy::expect_used` | lint | required |
| `cargo-deny check` | ライセンス・脆弱性・重複依存・bans | required (P1 から) |
| `cargo-machete` | 未使用 dependency | 週次 |
| `typos` | typo 検出 | required (固有名詞は `typos.toml` で除外) |

### `clippy.toml`

```toml
disallowed-methods = [
  { path = "std::fs::read", reason = "use services::fs or cx.background_spawn" },
  { path = "std::fs::write", reason = "use services::fs or cx.background_spawn" },
  { path = "std::fs::read_to_string", reason = "use services::fs or cx.background_spawn" },
]
```

### `rustfmt.toml`

```toml
edition = "2021"
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
reorder_imports = true
```

### `deny.toml`

```toml
[licenses]
allow = ["MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "BSD-2-Clause", "BSD-3-Clause", "Unicode-DFS-2016", "ISC", "Zlib"]
confidence-threshold = 0.93

[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/RustSec/advisory-db"]
vulnerability = "deny"
unmaintained = "warn"
yanked = "deny"

[bans]
multiple-versions = "warn"
deny = [
  # アプリコアでは禁止。WASI プラグイン実行層 (nohrs-plugin-host, P4 以降) のみ許可
  { name = "tokio", wrappers = ["nohrs-plugin-host"] },
  { name = "openssl-sys" },        # rustls 統一
]

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-git = []
```

---

## 4. ドキュメントテスト

- `#![warn(missing_docs)]` を **P1 から workspace 全体で有効**
- pub API に rustdoc が付いていなければ warning
- P7 で `deny` に格上げ
- `#[doc(test)]` でコード例の動作を確認

---

## 5. ベンチマーク (P2 以降)

| crate | ベンチ対象 |
|-------|-----------|
| `nohrs-store` | SQLite/redb の get/put レイテンシ、batch 操作 |
| `nohrs-services` | fs listing 並列度、search クエリ実行 |
| `nohrs-launcher` (P3) | nucleo ranking、起動時間 |
| `nohrs-plugin-host` (P4) | WIT host call レイテンシ、permission check overhead |

`criterion` crate を採用。CI では fail させず、main へのマージで履歴保存 (将来 regression 検知に使用)。

---

## 6. ローカル検証コマンド

```bash
# 全テスト
cargo test --all-features --workspace

# カバレッジ (HTML を target/llvm-cov/html で開く)
cargo llvm-cov --all-features --html
open target/llvm-cov/html/index.html

# lint
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings \
    -W clippy::unwrap_used -W clippy::expect_used

# 静的解析
cargo deny check
cargo machete
typos

# unsafe / panic 残置検出
grep -rn 'unsafe' crates/ | grep -v '//' | grep -v 'unsafe_code' || echo OK
grep -rn 'panic!\|unimplemented!\|todo!' crates/

# rustdoc
cargo doc --no-deps --all-features --workspace
```
