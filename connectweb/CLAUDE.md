# connectweb — AIエージェント運用ルール

スキーマ（.proto）を単一の真実に置き、1つの生成型から HTML・埋め込みJSON・Connect API を
駆動するスタック。土台は [fastweb](../fastweb/) と同じ（ビルド回避 + package by feature）。
fastweb の `CLAUDE.md` のルールは原則すべて引き継ぐ。差分だけ以下に書く。

## 大方針（3層の開発ループ）
1. **テンプレ・CSS・HTMX属性の変更** → 保存で即反映（ビルドゼロ。作業の7〜8割）
2. **ハンドラ・サービス層(get_user等)の変更** → 該当クレートだけ数秒で再ビルド
3. **スキーマ(.proto)の変更** → schema クレートで codegen が走る（proto を触ったときだけ）

## スキーマファーストの掟（connectweb 固有）
- **型は .proto に定義する**。手書きの表示用 struct を別に作らない。画面・API・埋め込みJSONは
  同じ生成型を共有する（型を1つにするのがこのプロジェクトの目的）。
- **ビュー専用メッセージを切る**。画面に出す約束のフィールドだけを `XxxPageView` に定義する。
  「全データ入りの型を作って一部だけ使う」は**禁止** ── それは情報境界を壊すアンチパターン。
  view-source で見える範囲 = もともとその画面に出す約束の範囲、を常に保つ。
- **フルページは `render_view` を使う**。`state.render_view("feat/page.html", &view)` が
  同じインスタンスを「描画」と「`</body>` 直前の `<!-- view-data -->` コメント埋め込み」の
  両方に流す。データ取得は1回に保つ（描画用と埋め込み用で別々に取得しない ── ズレる）。
- **HTMXフラグメント（部分HTML）は `render_view_fragment` を使う**。フルページと同じく同一
  インスタンスを `<!-- view-data -->` コメントで埋め込む（断片なので先頭に付く）。view-data は
  `<script>` タグでなく**HTMLコメント**＝デバッグ用の覗き窓で、本番DOM/JSには出さない。
  context だけ渡したい素のHTMLは `render` を使う（埋め込みなし）。
- **テンプレは camelCase で参照する**。buffa の serde は proto3 JSON 準拠。proto の
  `recent_activities` は `view.recentActivities`。snake_case で書くと undefined になる。

## クレート依存の向き（package by feature + schema）
- `schema` は葉に近い土台（依存は buffa/connectrpc/serde のみ）。**全クレートが共有する**。
- `feature-*` の依存は **webcore と schema だけ**。feature 間依存・connectrpc 直依存はしない
  （API公開は rpc クレートが担当する）。
- `rpc` は Connect API の薄い殻。**サービス層関数を import して呼ぶだけ**でロジックを持たない。
  `rpc → feature-*` の依存はOK（同じ get_user を共有するため）。逆向きは禁止。
- ロジックの本体（get_user 等のサービス層関数）は feature クレートに置き、HTML 経路と
  rpc 経路の両方がそれを呼ぶ。**自分自身への gRPC ループバックを張らない**（同一プロセスは関数呼び出し）。

## codegen を増やさない
- proto とその codegen は `schema` クレートに隔離する。build.rs(protoc) が走るのは
  proto を変更したときだけ。feature/rpc/webcore の Rust 編集では再生成されない。
- proto を変えたら `cargo check -p schema` で生成と型を確認してから、利用側を直す。

## このリポの作法（fastweb から継承）
- 推測で埋めない。事実（実データ・一次ソース・実際に動かした結果）を起点に答える。
  クレートのAPIは docs.rs や `~/.cargo/registry/src/` の実ソースで確認してから使う。
- 触る機能のフォルダ以外に書き込まない。ワークスペース共通 `Cargo.toml` の不用意な変更は
  全クレート再ビルドを誘発するので避ける。
- Tailwindのクラス名は常に完全形で書く（動的合成は本番パージで崩れる）。
