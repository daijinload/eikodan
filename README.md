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

## 本体

| ディレクトリ | 概要 |
| --- | --- |
| **[lastshot](./lastshot/)** | **最終進化形態。各サブプロジェクトの結論を全部統合した本番実装** |
| ├ [lastshot/docs](./lastshot/docs/) | **知見の集約先。実測・設計判断・採否記録はすべてここ** |
| └ [lastshot/docs/decisions](./lastshot/docs/decisions/) | 採用/不採用の判断記録（「これ試したっけ？」はここ） |

**ドキュメントを探すなら [`lastshot/docs/README.md`](./lastshot/docs/README.md) が入口です。**

## 実験場（アーカイブ）

個別要素を試した曳光弾。**これらのフォルダは将来削除する予定**です。
知見は [`lastshot/docs/`](./lastshot/docs/) に取り込み済みで、**lastshot はこれらを一切参照していない**ので、
消しても情報は失われません。ここに残しているのは計測を再現するためのコードと手順だけです。

| ディレクトリ | 何を試したか | 出た結論の行き先 |
| --- | --- | --- |
| [rust-htmx](./rust-htmx/) | Rust + HTMX + MiniJinja + DaisyUI の TODO CRUD（第一弾） | [`docs/htmx-vs-spa.md`](./lastshot/docs/htmx-vs-spa.md) |
| [subsecond-demo](./subsecond-demo/) | Dioxus 0.7 + subsecond による Rust コードのホットパッチ | **不採用** → [`decisions/0001`](./lastshot/docs/decisions/0001-subsecond-hotpatch.md) |
| [fastweb](./fastweb/) | 「ビルドを避けて開発する」に全振りしたビルド高速化 | [`docs/build-speed.md`](./lastshot/docs/build-speed.md) / [`docs/hot-reload.md`](./lastshot/docs/hot-reload.md) |
| [connectweb](./connectweb/) | .proto を単一の真実に、生成型から HTML / JSON / Connect API を駆動 | [`lastshot/CLAUDE.md`](./lastshot/CLAUDE.md) スキーマファーストの掟 |
| [pg-bench](./pg-bench/) | 「最速 Postgres をメモリに書く」を実効速度で横並び比較 | [`docs/postgres.md`](./lastshot/docs/postgres.md) |
| [playwright-sample](./playwright-sample/) | Playwright の E2E サンプル + Playwright MCP の評価 | MCP は**不採用** → [`decisions/0002`](./lastshot/docs/decisions/0002-playwright-mcp.md) |
| [lint-format](./lint-format/) | lint/format ツールを種別ごとに実演する showcase | [`docs/lint-format.md`](./lastshot/docs/lint-format.md) |
| [task-runners](./task-runners/) | 「Makefile の代わりに何を使うか」を 9 通りで横並び比較 | [`docs/task-runner.md`](./lastshot/docs/task-runner.md) |

### 比較用の実装はブランチに置く

他スタックとの比較（Node.js / Next.js / Laravel）は**実装をマージせず、ドキュメントだけを main に入れる**運用です。

| ブランチ | 中身 | ドキュメント |
| --- | --- | --- |
| [`bench-rust-vs-node`](https://github.com/daijinload/eikodan/tree/bench-rust-vs-node) | `lastshot-node/` | [`docs/bench/rust-vs-node-db.md`](./lastshot/docs/bench/rust-vs-node-db.md) |
| [`feat/lastshot-3stack-compare`](https://github.com/daijinload/eikodan/tree/feat/lastshot-3stack-compare) | `lastshot-bench/` `lastshot-next/` `lastshot-laravel/` | [`docs/bench/3stack-heavy-screen.md`](./lastshot/docs/bench/3stack-heavy-screen.md) |

## リポジトリ共通のドキュメント

| ファイル | 内容 |
| --- | --- |
| [CLAUDE.md](./CLAUDE.md) | AIエージェント向けの共通方針（ドキュメントをどこに書くか） |

## agent-browser のインストール

ブラウザ操作・スクショ・QA を AI エージェントから行うための CLI（[vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser)）を使っています。

```sh
# CLI 本体をグローバルにインストール
npm i -g agent-browser
agent-browser install
```

リポジトリ直下の `.agents/` と `skills-lock.json` が skill の定義・ロックファイルです（コミット済みのものをそのまま使えます）。
