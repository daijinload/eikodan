# lastshot docs — 知見インデックス

eikodan の各サブプロジェクト（実験場）で試した結果のうち、**lastshot に効く「今の結論」**をここに集約する。

- **使い方・セットアップ** → [`../README.md`](../README.md)
- **AIエージェント運用ルール** → [`../CLAUDE.md`](../CLAUDE.md)
- **ここ（`docs/`）** → なぜそうしたか・実測値・採らなかった理由

> **lastshot は自己完結している。** 実験場（`fastweb/` `pg-bench/` `rust-htmx/` `subsecond-demo/`
> `connectweb/` `playwright-sample/` `lint-format/` `task-runners/`）は**将来消える前提**なので、
> ここのドキュメントは実験場を参照しない。知見は全て取り込み済み。

---

## まずここから

| ドキュメント                     | 何が書いてあるか                                                                      |
| -------------------------------- | ------------------------------------------------------------------------------------- |
| [`fast-rust.md`](./fast-rust.md) | **Rust 開発を加速するためにやったこと（総まとめ）。他のドキュメントへの目次も兼ねる** |
| [`decisions/`](./decisions/)     | **採用/不採用の判断記録。「これ試したっけ？」はここを見る**                           |

## 速度・ビルド

| ドキュメント                         | 何が書いてあるか                                                                                                           |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------- |
| [`cold-start.md`](./cold-start.md)   | **増分ビルド + 起動**が画面に出るまでの実測（約1秒の底）。cold start の正体（codesign / systemfd / リンカ）                |
| [`build-speed.md`](./build-speed.md) | **ビルド速度の実測台帳**（規模スケール・リンカ・並列フロント・sccache・cranelift・opt-level・LTO・フルビルドの warm/cold） |
| [`hot-reload.md`](./hot-reload.md)   | 保存→ブラウザ反映の仕組み（テンプレ監視 + CSS の2系統）                                                                    |
| [`postgres.md`](./postgres.md)       | in-memory Postgres 方式の横並び比較（unix ソケットが最速）                                                                 |

> 速度系の3本は担当を分けてある。**「実測値そのもの」は `build-speed.md` に集約**し、
> `cold-start.md` は増分＋起動の1秒ループ、`fast-rust.md` は施策の総覧、と役割で切っている。
> 数字を足すときは `build-speed.md` に足して、他は要約＋リンクで済ませる。

## 設計・運用

| ドキュメント                                   | 何が書いてあるか                                                                                                         |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| [`testing.md`](./testing.md)                   | テストの配置規約（仕様 md とテストの同名ペア / `spec_` と `unit_` / 実装とテストを分離する理由）とテストライブラリの採否 |
| [`htmx-vs-spa.md`](./htmx-vs-spa.md)           | HTMX を選んだ理由。SPA との比較・白フラッシュの正体・コード量                                                            |
| [`directory-layout.md`](./directory-layout.md) | 設定ファイルの置き場所（ルート直下 vs フォルダ集約）。自動探索と喧嘩しない置き方                                         |
| [`lint-format.md`](./lint-format.md)           | lint/format ツールの選定根拠（なぜ clippy 中心・Oxlint を入れないか）                                                    |
| [`task-runner.md`](./task-runner.md)           | タスクランナーの選定根拠（Make 代替 9 通りの比較。なぜ `./run` か）                                                      |
| [`ci-performance.md`](./ci-performance.md)     | CI 高速化の実測と採否（ARM runner / rust-cache / uv / alpine）                                                           |
| [`container-ops.md`](./container-ops.md)       | CI・ローカルのコンテナ構成（単一 vs compose 分割、1コンテナ1プロセス論）                                                 |
| [`bash-setup.md`](./bash-setup.md)             | macOS で bash 5.x を使う（`./run` の前提）                                                                               |
| [`naming.md`](./naming.md)                     | なぜ `lastshot` という名前か                                                                                             |

## 他スタックとの比較ベンチ

| ドキュメント                                                     | 何が書いてあるか                                | 実装の在処                                                                                                         |
| ---------------------------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| [`bench/rust-vs-node-db.md`](./bench/rust-vs-node-db.md)         | 「DBが律速だから Node も Rust も同じ」は本当か  | ブランチ [`bench-rust-vs-node`](https://github.com/daijinload/eikodan/tree/bench-rust-vs-node)                     |
| [`bench/3stack-heavy-screen.md`](./bench/3stack-heavy-screen.md) | Rust / Next.js / Laravel を重い一覧画面で横並び | ブランチ [`feat/lastshot-3stack-compare`](https://github.com/daijinload/eikodan/tree/feat/lastshot-3stack-compare) |

---

## ドキュメントを足すときのルール

0. **`lastshot/` の外を参照しない。** 実験場は将来消えるので、そこにしかない情報に依存した瞬間に
   このドキュメントは壊れる。必要な事実は**引用して取り込む**（リンクで済ませない）。
1. **このファイルの表に1行足せないなら、新しいファイルを作らない。** 既存ドキュメントの節にする。
2. **ファイル名は `lower-kebab-case.md`。**
3. **推測を書かない。** 実測値・一次ソース・実際に動かした結果を起点にする
   （[`../CLAUDE.md`](../CLAUDE.md) 「このリポの作法」）。検証していないことは「不明」と明記する。
4. **不採用の判断は [`decisions/`](./decisions/) に残す。** 消さない。同じ検討を二度しないため。
5. **比較用の実装はマージしない。** ブランチに残して、ドキュメントからブランチへリンクする
   （既存の 2 つのベンチがこの形）。
