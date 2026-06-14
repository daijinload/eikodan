# eikodan

理想のWebシステムを模索するための曳光弾（えいこうだん）プロジェクトです
モノレポでサンプルを作っていきます！！

## 要件

* システムが高速に動作すること
* 開発が高速にできること
* 出来るだけシンプルに開発できること
  * Reactの複雑性を排除してHTMXを使う
  * Rust側でMiniJinjaを使うことによりHTMX部分はコンパイル無しで開発できる
* 自動テストがあること

## サブプロジェクト

| ディレクトリ | 概要 |
| --- | --- |
| [rust-htmx](./rust-htmx/) | Rust + HTMX + MiniJinja + DaisyUI の TODO CRUD サンプル（第一弾） |
| [subsecond-demo](./subsecond-demo/) | Dioxus 0.7 + subsecond による Rust コードのホットパッチ検証デモ |
| [fastweb](./fastweb/) | 「ビルドを避けて開発する」に全振りした Rust + HTMX スタック |
| [connectweb](./connectweb/) | .proto を単一の真実に、生成型から HTML / JSON / Connect API を駆動 |
| [pg-bench](./pg-bench/) | 「最速 Postgres をメモリに書く」を実効速度で横並び比較するベンチ |
| [playwright-sample](./playwright-sample/) | Microsoft Playwright の E2E テストサンプル（Playwright MCP は評価のうえ不採用。比較は同 README） |
| [lint-format](./lint-format/) | lint/format ツールを種別ごとのサンプルで実演する showcase（rustfmt・clippy・oxfmt・buf・shfmt/shellcheck・sqlfluff） |

## agent-browser のインストール

ブラウザ操作・スクショ・QA を AI エージェントから行うための CLI（[vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser)）を使っています。

```sh
# CLI 本体をグローバルにインストール
npm i -g agent-browser
agent-browser install
```

リポジトリ直下の `.agents/` と `skills-lock.json` が skill の定義・ロックファイルです（コミット済みのものをそのまま使えます）。
