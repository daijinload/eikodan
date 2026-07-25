# lastshot docs — 知見インデックス

eikodan の各サブプロジェクト（実験場）で試した結果のうち、**lastshot に効く「今の結論」**をここに集約する。

- **使い方・セットアップ** → [`../README.md`](../README.md)
- **AIエージェント運用ルール** → [`../CLAUDE.md`](../CLAUDE.md)
- **ここ（`docs/`）** → なぜそうしたか・実測値・採らなかった理由

> 実験場（`fastweb/` `pg-bench/` `rust-htmx/` `subsecond-demo/` `connectweb/` `playwright-sample/`）は
> **再現用のコードが残っている場所**。数字を疑ったときはそちらへ。ドキュメントは全部ここに移してある。

---

## まずここから

| ドキュメント | 何が書いてあるか |
|---|---|
| [`fast-rust.md`](./fast-rust.md) | **Rust 開発を加速するためにやったこと（総まとめ）。他のドキュメントへの目次も兼ねる** |
| [`decisions/`](./decisions/) | **採用/不採用の判断記録。「これ試したっけ？」はここを見る** |

## 速度・ビルド

| ドキュメント | 何が書いてあるか | 一次資料の在処 |
|---|---|---|
| [`fast-rust.md`](./fast-rust.md) | 高速化施策の総覧と、効かなかったものの理由 | lastshot |
| [`cold-start.md`](./cold-start.md) | Rust 変更が画面に出るまでの実測。cold start の正体（codesign / systemfd / リンカ） | lastshot |
| [`hot-reload.md`](./hot-reload.md) | 保存→ブラウザ反映の仕組み（テンプレ監視 + CSS の2系統） | `../../fastweb/` |
| [`build-speed.md`](./build-speed.md) | ビルド速度の実測台帳（リンカ・並列フロント・sccache・cranelift・opt-level・LTO） | `../../fastweb/` |
| [`postgres.md`](./postgres.md) | in-memory Postgres 方式の横並び比較（unix ソケットが最速） | `../../pg-bench/` |

## 設計・運用

| ドキュメント | 何が書いてあるか | 一次資料の在処 |
|---|---|---|
| [`htmx-vs-spa.md`](./htmx-vs-spa.md) | HTMX を選んだ理由。SPA との比較・白フラッシュの正体 | `../../rust-htmx/` |
| [`ci-performance.md`](./ci-performance.md) | CI 高速化の実測と採否（ARM runner / rust-cache / uv / alpine） | lastshot |
| [`container-ops.md`](./container-ops.md) | CI・ローカルのコンテナ構成（単一 vs compose 分割、1コンテナ1プロセス論） | lastshot |
| [`bash-setup.md`](./bash-setup.md) | macOS で bash 5.x を使う（`./run` の前提） | lastshot |
| [`naming.md`](./naming.md) | なぜ `lastshot` という名前か | lastshot |

## 他スタックとの比較ベンチ

| ドキュメント | 何が書いてあるか | 実装の在処 |
|---|---|---|
| [`bench/rust-vs-node-db.md`](./bench/rust-vs-node-db.md) | 「DBが律速だから Node も Rust も同じ」は本当か | ブランチ [`bench-rust-vs-node`](https://github.com/daijinload/eikodan/tree/bench-rust-vs-node) |
| [`bench/3stack-heavy-screen.md`](./bench/3stack-heavy-screen.md) | Rust / Next.js / Laravel を重い一覧画面で横並び | ブランチ [`feat/lastshot-3stack-compare`](https://github.com/daijinload/eikodan/tree/feat/lastshot-3stack-compare) |

---

## ドキュメントを足すときのルール

1. **このファイルの表に1行足せないなら、新しいファイルを作らない。** 既存ドキュメントの節にする。
2. **ファイル名は `lower-kebab-case.md`。**
3. **推測を書かない。** 実測値・一次ソース・実際に動かした結果を起点にする
   （[`../CLAUDE.md`](../CLAUDE.md) 「このリポの作法」）。検証していないことは「不明」と明記する。
4. **不採用の判断は [`decisions/`](./decisions/) に残す。** 消さない。同じ検討を二度しないため。
5. **比較用の実装はマージしない。** ブランチに残して、ドキュメントからブランチへリンクする
   （既存の 2 つのベンチがこの形）。
