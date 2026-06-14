# fastweb — AIエージェント運用ルール

Rust + HTMX + MiniJinja + daisyUI の「ビルドを避けて開発する」高速スタック。
構造を作ってもエージェントの行動が局所性を壊すと意味がないので、以下を守ること。

## 大方針（3層の開発ループ）
1. **テンプレ・CSS・HTMX属性の変更** → 保存で即反映（Rustビルドゼロ。作業の7〜8割はここ）。
   日常CSSは **CDN**（ブラウザJIT）でビルドもwatchも不要。本番でだけ崩れる動的合成クラスは、
   push前のゲート（`assets/check-css.sh` = CLIクリーンビルド + semgrep）で確定検査する。
2. **ハンドラ・ロジックの変更** → 該当featureクレートだけ数秒で再ビルド
3. **最終確認/納品時のみ** → `CSS=built` で起動しCLI生成CSSを目視（release は常にCLI生成）。

> CSSモードは実行時グローバル `css_built` で切替（base.html）。`cfg!(debug_assertions)` には
> 紐付けない ── debug↔最終確認を往復してもRustの再ビルドが走らないようにするため（プロジェクトの肝）。

動的言語・スクリプト層は足さない。Rustの型チェックを品質担保装置として全面的に残す。

## ビルドを増やさないための禁止事項
- **触る機能のフォルダ以外に書き込まない。** 全ファイルへのフォーマッタ一括適用、
  ワークスペース共通 `Cargo.toml`（`[workspace.dependencies]` / プロファイル）の
  不用意な変更は、全クレート再ビルドを誘発するので禁止。
- **UIの変更はテンプレートとCSSで完結させる。** Rustに触るのはデータ取得の形が
  変わるときだけ。表示分岐・整形はできるだけ MiniJinja 側に寄せる。
- **Tailwindのクラス名は常に完全な形で書く。** `text-{{ color }}-500` のような動的合成は禁止
  （CLIのパージがリテラルを見つけられず本番で消える。**日常はCDNなので開発中は再現せず**、push前の
  semgrep が代わりに捕まえる）。`{% if error %}text-error{% else %}text-success{% endif %}` と完全形で分岐する。
- **HTMLはRustに書かずテンプレに置く。** `assets/input.css` は `source(none)` で走査対象を
  テンプレHTML(`@source`)だけに限定している。Rust文字列でHTMLを返すと、(1)変更のたびに再ビルドが要る
  (2)クラスが走査外で無スタイルになる、の二重で損をする。HTMXの部分HTMLも `state.render(...)` で
  テンプレから返すこと（例: `feature-hello` の `clicked`）。新しいクラスはテンプレに完全形で書く。
  ↑この2つは `assets/semgrep/tailwind-purge.yml` で機械的に検査（ゲートで自動enforce）。

## クレート構成のルール（package by feature）
- 各 `feature-*` クレートは **葉(leaf)** に保つ。依存は `webcore`（共有コア）のみ。
  feature間依存、`webcore → feature` の逆向き依存は**禁止**（局所性が崩壊する）。
- 共有コア `webcore` には「本当に安定した型・処理」だけを置く。開発中の型はまず
  feature内に置き、多少の型重複は許容する。**共有コアへの昇格は人間が判断する。**
- `bin`(app) は薄い層: ルーター組み立て・起動・ライブリロードのみ。各featureは
  `pub fn routes() -> Router<AppState>` を公開し、app が `.merge()` するだけ。
- テンプレートは各featureの `templates/` に同居。名前は **`feature名/部品名.html`** 規約。

## イテレーションの回し方
- 確認は **`cargo check -p <feature>`**（+ `cargo clippy -p <feature>`）。フルビルドを待たない。
- `bacon` が裏で回っている前提で動く（保存→check→再ビルド→ソケット維持で再起動）。
- テストは**触っているクレートの単体テスト**のみ（`cargo nextest run -p <feature>`）。
  HTTP越しの統合テストは別プロジェクト `tests-http/`（ワークスペース外）で、節目にだけ実行する。
- **push前のCSSゲート**: `bash assets/check-css.sh`（クリーンビルドで追加/削除パージ確定 + semgrep）。
  pre-commit は使わない（軽量開発と最終確認を分ける）。任意で `assets/hooks/pre-push` を入れると自動化できる。

## connect-rpc を足すとき
- proto とハンドラは `crates/rpc` に隔離する。codegen(build.rs/protoc)が走るのは
  proto を変更したときだけ。HTMLハンドラ側は影響を受けない。

## このリポの作法
- 推測で埋めない。事実（実データ・一次ソース・実際に動かした結果）を起点に答える。
  検証できない場合は「不明」と明記する。クレートのAPIは docs.rs や
  `~/.cargo/registry/src/` の実ソースで確認してから使う。
