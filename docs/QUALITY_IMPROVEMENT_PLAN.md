# Nohrs 品質向上ロードマップ

## Context（背景）

`nohrs` は Rust + gpui で書かれた macOS 向け高速ファイラーで、`v0.1.0`・約5,000行・コミット42本のスナップショット。コア機能（ファイル一覧・検索・プレビュー）は動くが、メンテナ自身が「えいやで作った」と認めており、以下のような OSS 公開には早い状態にある:

- **検索の設計と実装が乖離している**: 当面は Spotlight（`mdfind`）に一本化する方針だが、現状 `SearchScope::Home` は Tantivy ベースの `IndexManager`（`src/services/search/indexer.rs` 421行）+ `FileWatcher`（`notify-debouncer-mini`）で並走しており、起動時に `~/Documents` をフルインデックスしている（`src/services/search/engine.rs:50-94`）。`docs/SEARCH_IMPLEMENTATION.md:45` もこの2系統並走を「現状」と記述しており、方針と矛盾している
- `.github/` が `.gitignore` 入りで CI/CD が一切なく、`Cargo.toml` に `description`/`license`/`repository`/`rust-version` 等のメタ情報が無い
- `unsafe` で `Box<dyn Fn>` の生ポインタを closure 経由で渡している（`src/ui/components/file_list.rs:158-163`）
- 同期 I/O を非同期 UI コンテキストでブロッキング呼び出し（`src/pages/explorer/mod.rs:547-675` の `open_preview`）
- `apply_filter()` の二重呼び出しなど明らかなバグ（`src/pages/explorer/mod.rs:533-534`）
- `format_date` の日付計算が独自実装でうるう年・各月日数を無視（`src/ui/components/file_list.rs:209-226`）
- `core::errors::Error` が3バリアントしか無く、`anyhow` と `thiserror` の境界が曖昧
- テストは `tests/indexing_test.rs`（Tantivy 専用）と `tests/watcher_test.rs` の2本のみ。Spotlight 一本化に伴い再設計が必要
- ドキュメントは README/`docs/SEARCH_IMPLEMENTATION.md` のみで、rustdoc コメントは50ファイル中わずか
- `src/ui/app.rs:55` で `SearchService::new` 失敗時に `panic!`

ユーザー選好（確認済み）:
1. **既存コード品質の地固め優先** — Planned 機能は後回しでよい
2. **macOS 専用を当面維持** — 抽象化（`SearchBackend` trait）は維持
3. **フェーズ別ロードマップ形式** — マイルストーン単位
4. **日本語ドキュメント**

ゴール: `v0.2.0` 公開時点で「他人が Issue/PR を出したくなる」OSS プロジェクトの体裁を整え、後続の Planned 機能を安全に積み上げられる土台を作る。

---

## 全体方針

- **「動く現状を壊さない順序」** で並べる。フェーズは原則直列、ただし P2（OSS 体裁）は P1（緊急修正）と並行可能。
- **既存ユーティリティを活かす**: `SearchBackend` trait、`thiserror` ベースの `core::errors`、`spawn_blocking`（既に `engine.rs`/`fs/listing.rs` で適用済）。新規抽象は最小限。
- **検索バックエンドは Spotlight 一本化を最優先**: 当面 macOS に絞っている以上、自前 Tantivy インデックスを並走させる合理性は薄い。インデックス未完了タイミングでの空振り、起動コスト、`~/.nohrs/index` のディスク占有、watcher 競合などの問題を一括で解消する。Tantivy/notify/grep/ignore は依存ごと撤去（非 macOS は ripgrep フォールバックのみ残す）。
- **Breaking change は v0.2.0 のメジャー bump で許容**。`0.1.0 → 0.2.0` 切り替えのこのタイミングに集約する。
- **`unsafe` を 0 にする** ことを v0.2.0 のリリースゲートに含める。
- 各フェーズの完了条件として **「`cargo fmt --check && cargo clippy -D warnings && cargo test`」が CI で通る** ことを必須化。

---

## フェーズ別ロードマップ

### Phase 1 — 緊急修正（バグ/安全性） 🚨

OSS 公開以前のレベルの不具合・安全性問題を潰す。順序通り。

| # | タスク | 対象 |
|---|--------|------|
| 1.1 | `unsafe` ブロックの除去。`FileListDelegate.on_confirm` を `Arc<dyn Fn>` に変更し、`render_item` 内で `Arc::clone` してクロージャに move する | `src/ui/components/file_list.rs:13, 154-164` |
| 1.2 | `apply_filter()` 二重呼び出しを削除 | `src/pages/explorer/mod.rs:533-534` |
| 1.3 | `open_preview` の同期 I/O を `tokio::task::spawn_blocking` 経由に。完了後 `cx.update` で UI 反映 | `src/pages/explorer/mod.rs:547-675` |
| 1.4 | `SearchService::new` 失敗時の `panic!` を撤廃。`Result` を上位に伝搬し、起動時にダイアログ表示 or 検索無効化モードで起動 | `src/ui/app.rs:50-57` |
| 1.5 | `format_date` を `time` crate（既に依存）の `OffsetDateTime::from_unix_timestamp` ベースに置換 | `src/ui/components/file_list.rs:209-226` |
| 1.6 | `SpotlightBackend::search` の `mdfind` 引数注入対策。クエリを `--`/`-onlyin` 指定でリテラル扱いし、`mdfind` 終了コード別ハンドリング | `src/services/search/spotlight.rs:51-79` |
| 1.7 | `pages/explorer/mod.rs:174-177` の "ログだけしてユーザーに伝えない" エラー処理を、フッターのステータスバーへ通知する経路に置換（フッターは `src/ui/components/layout/footer.rs` に存在） | `src/pages/explorer/mod.rs:174-177`, `src/pages/explorer/types.rs` |

**完了条件**: `unsafe` 0個、`grep -rn 'panic!\|unimplemented!\|todo!' src/` の結果が0、`cargo test` パス。

---

### Phase 2 — OSS 体裁の最低ライン整備（P1 と並行可） 🧰

外向きに公開しても恥ずかしくない最小セットを整える。

| # | タスク | 対象 |
|---|--------|------|
| 2.1 | `.gitignore` から `/.github` 行を削除し、`.github/workflows/ci.yml` を新規追加（`fmt --check` / `clippy -D warnings` / `cargo test` / `cargo build --features gui`） | `.gitignore:27`, 新規 `.github/workflows/ci.yml` |
| 2.2 | `Cargo.toml` の `[package]` に `description`, `repository`, `homepage`, `license = "MIT"`, `readme`, `keywords`, `categories`, `authors`, `rust-version` を追加 | `Cargo.toml` |
| 2.3 | Cargo の features を crates.io 公開準備として整理。`gpui` 系の重い依存は `optional = true` 維持、`image`/`syntect`/`tantivy` の `default-features` を最小化 | `Cargo.toml` |
| 2.4 | `CONTRIBUTING.md`（環境構築/ブランチ規約/コミット規約/テスト方法）, `CODE_OF_CONDUCT.md`（Contributor Covenant v2.1）, `SECURITY.md`（脆弱性報告窓口）, `CHANGELOG.md`（`Keep a Changelog` 準拠）を追加 | 新規 |
| 2.5 | `.github/ISSUE_TEMPLATE/bug_report.yml`, `feature_request.yml`, `PULL_REQUEST_TEMPLATE.md` を追加 | 新規 |
| 2.6 | `.github/dependabot.yml` で cargo + github-actions の週次更新を有効化 | 新規 |
| 2.7 | `rustfmt.toml`（基本は default + `imports_granularity = "Crate"` 程度）と `clippy.toml`（`disallowed-methods` で `std::fs::read` を禁止し `tokio::fs` を強制、など最低限）を追加 | 新規 |
| 2.8 | `LICENSE` ファイル先頭の Copyright 行を「年 + 著者名」に整える（現状要確認） | `LICENSE` |

**完了条件**: GitHub 上で CI バッジが green。`cargo publish --dry-run` がメタ情報エラーを出さない。

---

### Phase 3 — 検索アーキテクチャの整合（Spotlight 一本化） 🔍

「インデックスは当面 Tantivy ではなく Spotlight」という方針と現状コード／ドキュメントを揃える。**最も影響範囲が広い変更**であり、エラー設計（P4）・リファクタ（P5）・テスト（P6）・ドキュメント（P7）の前提となるためここに置く。

#### 現状（食い違いの整理）

| 観点 | コードの現状 | 方針 |
|------|-------------|------|
| `SearchScope::Home` | Tantivy `IndexManager` でフルテキスト検索（`src/services/search/indexer.rs`, `engine.rs:50-94` で起動時にフル indexing 起動）| Spotlight (`mdfind -onlyin $HOME`) で代替 |
| `SearchScope::Root` | macOS: Spotlight / 他: Ripgrep | 維持（macOS は Spotlight、非 macOS は Ripgrep） |
| ファイル watcher | `notify-debouncer-mini` で home を監視し Tantivy index を更新 | 不要（Spotlight が OS 側でインデックスを維持）→ 撤去 |
| 初期 indexing | 起動時 `~/Documents` を tantivy で walk | 不要 → 撤去 |
| ドキュメント | `docs/SEARCH_IMPLEMENTATION.md:45` で「Home は Tantivy」と明記 | 「macOS は Home/Root とも Spotlight、非 macOS は Ripgrep」と書き直す |

#### タスク

| # | タスク | 対象 |
|---|--------|------|
| 3.1 | `SpotlightBackend` をスコープ対応に拡張: `SpotlightBackend::new(scope_paths: Option<Vec<PathBuf>>)` を受け、`mdfind` 引数に `-onlyin <path>` を付与（Home: `$HOME`、Root: 引数なし） | `src/services/search/spotlight.rs` |
| 3.2 | `SearchEngine` から `IndexManager` / `FileWatcher` / 初期 indexing 起動コードを削除。macOS ビルドでは Home/Root とも `SpotlightBackend` を使う。非 macOS は `RipgrepBackend::new(home_dir)` と `RipgrepBackend::new(PathBuf::from("/"))` の2インスタンス | `src/services/search/engine.rs:11-103` |
| 3.3 | `src/services/search/indexer.rs`（421行）と `src/services/search/watcher.rs`（39行）を削除。`src/services/search/mod.rs` の `pub mod indexer;` / `pub mod watcher;` 行も削除 | `src/services/search/{indexer,watcher,mod}.rs` |
| 3.4 | `progress_subscription` は Spotlight では不要（`mdfind` は即時応答）。`SearchService::progress_subscription` を削除し、呼び出し側（`pages/explorer/mod.rs` のプログレスバー表示があれば）を整理 | `src/services/search/{mod,engine}.rs`, `src/pages/explorer/**` |
| 3.5 | `Cargo.toml` から `tantivy`、`notify`、`notify-debouncer-mini`、`ignore`、`grep` を削除。非 macOS では `ignore` + `grep` が必要なので、`[target.'cfg(not(target_os = "macos"))'.dependencies]` セクションに移す | `Cargo.toml` |
| 3.6 | `tests/indexing_test.rs`（Tantivy 専用）を削除。`tests/watcher_test.rs` も削除（watcher 自体が無くなる） | `tests/*.rs` |
| 3.7 | `SpotlightBackend` のテスト追加: `mdfind` が無い環境ではテストを `#[ignore]`、ある環境では `tempfile` で実ファイル作成 → `mdfind` 反映待ちが不安定なため、テストは「コマンド組み立て」と「出力パース」を純粋関数化して検証 | 新規 `src/services/search/spotlight.rs` 末尾 `#[cfg(test)]` |
| 3.8 | `docs/SEARCH_IMPLEMENTATION.md` を全面改訂。「Home は Tantivy」記述を削除し、Spotlight 一本化の理由・トレードオフ（mdfind が無効な環境/Spotlight 除外パスの扱い）・将来 Tantivy 復活させる場合の判断基準を明記 | `docs/SEARCH_IMPLEMENTATION.md` |
| 3.9 | `docs/README.ja.md` も同期更新 | `docs/README.ja.md` |
| 3.10 | `~/.nohrs/index` ディレクトリのマイグレーション処理（起動時に存在すれば warning ログ + 削除提案、または黙って削除）を `src/ui/app.rs` 初期化に追加 | `src/ui/app.rs` |

**完了条件**:
- `cargo tree | grep -E '^(tantivy\|notify\|grep)'` が macOS ビルドで空
- 起動時に `~/Documents` の walk が走らないことを `tracing` ログで確認
- `mdfind "TODO" -onlyin $HOME` で得られる結果と nohrs Home 検索の結果が一致
- `docs/SEARCH_IMPLEMENTATION.md` と実装が整合

---

### Phase 4 — エラー設計と境界の整理 🧱

P1/P3 で潰した穴に対する恒久対策。コードベース全体の「エラーの渡し方」を統一する。

| # | タスク | 対象 |
|---|--------|------|
| 4.1 | `core::errors::Error` にドメインバリアント追加: `Subprocess { cmd: &'static str, code: Option<i32> }`（mdfind 失敗）、`Decode(String)`、`Walk(#[from] ignore::Error)`（非 macOS）など。**Tantivy/notify バリアントは P3 で依存が消えるため不要** | `src/core/errors.rs` |
| 4.2 | サービス層（`src/services/**`）の戻り値型を `anyhow::Result` から `crate::core::errors::Result` に揃える。`anyhow` はバイナリ層（`src/gui/main.rs`）と test のみで使用するルールに | `src/services/search/*.rs`, `src/services/fs/*.rs` |
| 4.3 | ユーザー向けエラー表示の単一経路（`ExplorerPage::set_status(StatusKind, String)` 等）を導入。`pages/explorer/types.rs` に `Status` 構造体を追加し、`footer.rs` で描画 | `src/pages/explorer/types.rs`, `src/pages/explorer/mod.rs`, `src/ui/components/layout/footer.rs` |
| 4.4 | `clippy::unwrap_used` / `clippy::expect_used` を `#![warn]` レベルで lib に追加。残った `unwrap` を `?` ないし `unwrap_or_else` に書き換え | 全 `src/**` |

**完了条件**: `cargo clippy -- -D warnings -W clippy::unwrap_used -W clippy::expect_used` が通る。

---

### Phase 5 — リファクタリングと巨大ファイル分解 ✂️

`src/pages/explorer/mod.rs` (688行) を中心に責務分解。**機能追加せず、純粋に構造改善のみ**。

| # | タスク | 対象 |
|---|--------|------|
| 5.1 | `ExplorerPage` のステート/ハンドラ/ビューを分割。状態を `ExplorerState`（`types.rs` 拡張）、検索周りを `pages/explorer/search.rs`（既存）、プレビュー周りを新規 `pages/explorer/preview.rs` に移管 | `src/pages/explorer/mod.rs` 全体 |
| 5.2 | `view/listing/row.rs` (276行) で多発する `.clone()` を `Arc<FileEntryDto>` 共有に置換し、closure キャプチャ回数を削減 | `src/pages/explorer/view/listing/row.rs` |
| 5.3 | `apply_filter()` のフィルタ結果を `Vec<usize>` インデックス保持にして毎キーで `entries.clone()` するのを止める | `src/pages/explorer/mod.rs:320-340` 付近 |
| 5.4 | 言語拡張子→syntect language の対応表（現在 `open_preview` 内のハードコード match）を `services::syntax` に集約し、テスト可能にする | `src/services/syntax.rs`, `src/pages/explorer/mod.rs:577-593` |
| 5.5 | マジックナンバー（2MB プレビュー上限、1000マッチ上限、行長1000上限など）を `core/config.rs` に切り出し、`pub const` 化 | 新規 `src/core/config.rs`, `src/services/search/spotlight.rs:28,40`, `src/pages/explorer/mod.rs:556` |

**完了条件**: `tokei` ないし `cloc` で 1ファイル 400行超を 0 にする。`cargo clippy` の `too_many_lines` 警告 0。

---

### Phase 6 — テスト基盤の拡充 🧪

P3 で Tantivy 系テストが消えるため、ここで再構築する。

| # | タスク | 対象 |
|---|--------|------|
| 6.1 | `tests/` を `tests/common/mod.rs` の共有ヘルパ + ユースケース別ファイル構成に再編 | 新規 |
| 6.2 | `services::fs::listing` のユニットテスト追加（隠しファイル/シンボリックリンク/権限なしディレクトリ）。`tempfile` で実ディレクトリを作って検証 | 新規 `src/services/fs/listing.rs` 末尾 `#[cfg(test)]` |
| 6.3 | `services::syntax`（5.4で切り出し）と `core/config`（5.5で切り出し）のユニットテスト | 新規 |
| 6.4 | `SpotlightBackend` の `mdfind` 引数組立・出力パースを純粋関数化してテスト | `src/services/search/spotlight.rs` 末尾 |
| 6.5 | `ui/components/file_list::human_bytes` / `format_date` のテーブル駆動テスト（`time` crate ベースに置換後） | `src/ui/components/file_list.rs` 末尾 |
| 6.6 | `pages/explorer` の純粋ロジック（フィルタ適用、検索結果マージ）を `#[cfg(not(feature = "gui"))]` で抽出してテスト可能にする | `src/pages/explorer/search.rs`, 新規 |
| 6.7 | GitHub Actions に `cargo llvm-cov` を組み込み、PR にカバレッジ差分を投稿（codecov / coveralls 任意） | `.github/workflows/ci.yml` |

**完了条件**: ライン カバレッジ 50% 以上、コアロジック（`core/`, `services/`）は 80% 以上。

---

### Phase 7 — ドキュメント整備 📚

| # | タスク | 対象 |
|---|--------|------|
| 7.1 | `docs/ARCHITECTURE.md` を新規作成。`core / services / pages / ui / models` の責務とデータフロー図（Mermaid） | 新規 |
| 7.2 | 主要型・関数に rustdoc を付与。少なくとも `pub` API は全てに `///` 必須。`#![warn(missing_docs)]` を lib に追加 | `src/lib.rs`, 各 `mod.rs` |
| 7.3 | `README.md` に「Status: Pre-alpha」セクション、スクリーンショット更新、`cargo install` での導入手順（公開後）を追記 | `README.md` |
| 7.4 | `docs/SEARCH_IMPLEMENTATION.md`（P3.8 で大改訂済）と最終コードの差分を再点検 | `docs/SEARCH_IMPLEMENTATION.md` |
| 7.5 | `docs/README.ja.md` も同期させる | `docs/README.ja.md` |

**完了条件**: `cargo doc --no-deps` が warning 0 で生成完了。

---

### Phase 8 — リリース運用整備 🚀

| # | タスク | 対象 |
|---|--------|------|
| 8.1 | `.github/workflows/release.yml` を追加。タグ push (`v*`) で macOS バイナリビルド & `gh release` 作成 | 新規 |
| 8.2 | `cargo-release` 設定（`release.toml`）。`CHANGELOG.md` 自動更新ルール | 新規 |
| 8.3 | `v0.2.0` をタグ付け & リリースノート公開 | git tag |

**完了条件**: GitHub Releases に `v0.2.0` のバイナリ（または `.dmg`）が掲載される。

---

### Phase 9 以降（後続）— Planned 機能着手 🔮

P1–P8 完了後に README の "Planned Features" を着手。優先順序（暫定）:
1. Settings ページの実体化（テーマ切替、フォント、デフォルトソート）— 既存 `src/pages/settings.rs` 42行スタブを置換
2. Tabs / Split view — Explorer の state を `Vec<ExplorerState>` 化
3. Bulk rename（regex）
4. Git 統合（`gix` 推奨、`git2` は libgit2 依存で重い）
5. Plugin システム — API 設計を別途 RFC として `docs/rfcs/` に
6. S3 統合

各機能着手前に `docs/rfcs/NNNN-name.md` で設計レビュー（GitHub Discussions or PR）を必須化する。

---

## 重要ファイル一覧

| パス | 主な改修フェーズ |
|------|------------------|
| `Cargo.toml` | P2, P3 |
| `.gitignore` | P2 |
| `.github/workflows/ci.yml` (新規) | P2, P6 |
| `src/core/errors.rs` | P4 |
| `src/core/config.rs` (新規) | P5 |
| `src/ui/app.rs` | P1, P3 |
| `src/ui/components/file_list.rs` | P1, P6 |
| `src/pages/explorer/mod.rs` | P1, P3, P4, P5 |
| `src/pages/explorer/preview.rs` (新規) | P5 |
| `src/pages/explorer/view/listing/row.rs` | P5 |
| `src/services/search/mod.rs` | P3 |
| `src/services/search/engine.rs` | P3 |
| `src/services/search/spotlight.rs` | P1, P3, P4, P6 |
| `src/services/search/indexer.rs` | **P3 で削除** |
| `src/services/search/watcher.rs` | **P3 で削除** |
| `src/services/search/ripgrep.rs` | P3（非 macOS 用に scope 引数を追加） |
| `src/services/syntax.rs` | P5, P6 |
| `src/services/fs/listing.rs` | P4, P6 |
| `tests/indexing_test.rs` | **P3 で削除** |
| `tests/watcher_test.rs` | **P3 で削除** |
| `docs/SEARCH_IMPLEMENTATION.md` | P3（全面改訂） |
| `docs/README.ja.md` | P3, P7 |
| `docs/ARCHITECTURE.md` (新規) | P7 |
| `docs/rfcs/` (新規) | P9 |

---

## 検証方法

各フェーズの完了時、ローカルで以下を実行し、CI でも同じ手順を回す:

```bash
# フォーマット & lint
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings

# ビルド（lib のみ + GUI 含む）
cargo build
cargo build --features gui

# テスト
cargo test --all-features

# (P6以降) カバレッジ
cargo llvm-cov --all-features --lcov --output-path lcov.info

# (P7) ドキュメント
cargo doc --no-deps --all-features

# (P2以降) 公開チェック
cargo publish --dry-run

# unsafe ブロック検出（P1 完了後は 0 件であること）
grep -rn 'unsafe' src/ | grep -v '//' || echo "no unsafe blocks"

# 残置 TODO/panic 検出
grep -rn 'panic!\|unimplemented!\|todo!\|FIXME' src/

# (P3 完了後) macOS では tantivy / notify が依存ツリーから消えていること
cargo tree --target aarch64-apple-darwin | grep -E 'tantivy|notify|^grep ' && echo "FAIL: legacy deps remain" || echo "OK"

# (P3 完了後) Home 検索が mdfind と一致することを目視確認
mdfind "<keyword>" -onlyin "$HOME" | head
cargo run --features gui --bin nohrs   # アプリ側で同じクエリを実行

# GUI 手動確認（macOS 上）
cargo run --features gui --bin nohrs
# - 起動・ディレクトリ移動・検索 ON/OFF（Home/Root とも）・プレビュー
#   （テキスト/画像/大きいファイル/バイナリ）・親ディレクトリへの戻りなど
#   ゴールデンパスを一通り操作
```

---

## 想定タイムライン（参考）

| Phase | 規模感 | 並行性 |
|-------|--------|--------|
| P1 緊急修正 | 1–2日 | — |
| P2 OSS 体裁 | 1日 | P1 と並行可 |
| P3 検索アーキ整合（Spotlight 一本化） | 2–3日 | P1 完了後 |
| P4 エラー設計 | 2–3日 | P3 完了後 |
| P5 リファクタ | 3–5日 | P4 完了後 |
| P6 テスト | 3–4日 | P5 と一部並行可 |
| P7 ドキュメント | 1–2日 | P6 と並行可 |
| P8 リリース運用 | 1日 | P7 完了後 |

合計: 約 2.5–3.5 週間で `v0.2.0` リリース可能な状態へ。
