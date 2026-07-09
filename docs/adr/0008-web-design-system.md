# 0008 — web のデザイン北極星は zed.dev、フロントエンドは Tailwind v4 + Radix 再スキン

> Status: Accepted
> Date: 2026-05-28

## Context

P1 で web (`chronos-fm.app`) を立ち上げるにあたり、当初の「MVP・skeleton で良い」方針を改め、**P1 から本格的・production-grade で立ち上げる** (issue #55 を re-scope) ことになった。これに伴い、(1) サイト全体のデザイン言語 (北極星) と (2) フロントエンドの実装スタック (スタイリング・コンポーネント) を確定する必要がある。

デザインの参照として zed.dev / Vercel / Cursor の 3 つが挙がったが、3 者は美学が大きく異なる:

- **zed.dev**: ライト/ダーク両対応・エディトリアル/職人的・独自タイポ + mono 多用・余白広め・モーション抑制。"craft tool"。
- **Vercel**: 白黒ストイック・Geist・幾何学的 bento・モーション/グラデ多用。"プラットフォーム"。
- **Cursor**: ダーク + 青紫グラデ・巨大デモ動画・グロッシー。"AI プロダクト"。

3 者を対等に混ぜると一貫性が崩れる。1 つを土台 (DNA) に据え、残りをアクセントとして借りる必要がある。

ブランドカラーについては、アプリ本体 (`src/ui/theme.rs`) の `ACCENT` が `#DEA584` (warm tan) であり、これは Rust 言語色でもある。アプリは現状ライトテーマ (`BG=WHITE`)。

実装スタックは TanStack Start (React) 上で、(a) フルスクラッチ CSS、(b) Tailwind + headless プリミティブ再スキン、(c) 意見の強い UI ライブラリ、のいずれか。

## Decision

### デザイン北極星

**zed.dev を土台 (DNA) とする**。職人的・エディトリアルなトーンが chronos-fm (Rust 製パワーユーザ向けツール) の製品性格に最も合う。残り 2 つはアクセント:

- Vercel から: タイポグラフィの規律、mono ラベル、コードブロック表現
- Cursor から: ヒーローの製品デモ (GIF/動画) の見せ方

### カラー

- **ライト主 / ダーク従** (トグル切替。アプリ本体がライトテーマなのでブランド一致)
- warm 寄りニュートラル
- **アクセント = Rust tan `#DEA584`** に一本化 (アプリ・Rust アイデンティティ・暖色ダークの三方に効く)。web では青系を使わない。

### タイポグラフィ

- ラテン見出し/本文: グロテスク・サンス (`Geist Sans` / `Inter` 系)
- アクセント/コード/ラベル: モノ (`Geist Mono` / `JetBrains Mono`)
- 和文: `Zen Kaku Gothic New` (ラテンとウェイトを揃える)
- 全フォントを Cloudflare に self-host

### モーション

控えめ・意味のある動きのみ (CSS 主体、一部 Motion、`prefers-reduced-motion` 対応必須)。グラデ/3D/パララックスは封印。

### フロントエンドスタック

**Tailwind v4 (CSS 変数デザイントークン) + shadcn/Radix の headless プリミティブを自前トークンで再スキン**。速さ × アクセシビリティ × 独自の見た目を両立する。

### 品質基準 (ローンチ条件)

a11y (WCAG AA)・パフォーマンス予算 (Lighthouse 95+)・フル SEO (sitemap・hreflang・OG/Twitter meta・JSON-LD)。

## Consequences

### Positive

- 一貫したデザイン言語 (1 土台 + 2 アクセント) で「3 者ブレンドのmush」を回避
- ブランドが Rust tan に一本化され、アプリ本体と web で統一される
- Radix によりモーダル (`Ctrl+K` 検索) / ドロップダウン / トグルの a11y を低コストで担保
- CSS 変数トークンでライト/ダーク・warm パレットを一元管理、保守容易
- self-host フォントで FOUT / GDPR / edge 遅延を回避

### Negative

- アプリ本体 `theme.rs` の `ACCENT` はコメントが "Blue" と誤記、`ACCENT_HOVER`/`ACCENT_LIGHT` が青系のまま残る。web を tan に一本化するのに合わせ、**アプリ側のブランド統一を別 issue で扱う**必要がある
- Tailwind / Radix の取り込みにより web 側の依存と build 設定が増える (Cargo workspace からは `exclude` 済なので Rust 側影響なし。ADR 0006)
- 「zed.dev を土台」は強い制約。将来 chronos-fm 独自の美学に育てる際は本 ADR を更新する

## Alternatives Considered

| 案 | 棄却理由 |
|----|---------|
| Vercel / Cursor を土台 | chronos-fm の職人ツール性格とずれる (プラットフォーム/AI プロダクト寄り)。構造も zed.dev に倣う以上、DNA も揃える方が一貫 |
| 3 者を対等にブレンド | 一貫性維持が難しく、デザインシステム構築コストが最も高い |
| ダーク主 / ダーク固定 | zed DNA の魅力はダークにあるが、アプリ本体がライトテーマでありブランド一致を優先。ライト主 + ダークトグルで両取り |
| フルスクラッチ CSS (vanilla-extract 等) | 職人度・制御は最高だが最も遅く、a11y を自前で背負う |
| 意見の強い UI ライブラリ (Mantine/Chakra) | 速いが zed 級の独自美学と衝突し汎用感が出る |
