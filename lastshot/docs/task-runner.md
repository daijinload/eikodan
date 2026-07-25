# タスクランナーの選定根拠 — なぜ `./run`（自作 bash）か

lastshot のタスクは `make` でも `just` でもなく **`./run`（bash 関数 + ディスパッチャ）**で回している。
その判断の記録。

> 出自: `task-runners/` showcase（Make 代替 8 ツール + 自作 bash の計 9 通りに**まったく同じ 5 タスク**
> （`hello` / `build` / `test`=build に依存 / `greet`=引数つき / `clean`）を実装して横並び比較した実験場）。
> 結論は全てここに取り込み済み。

## 結論

**lastshot は `./run`（自作 bash ディスパッチャ）を採る。**
**一般論として迷ったら `just`** ── Make の正統な後継で学習コストが一番低く、単一バイナリ、引数とデフォルト値が素直。
だが lastshot は「**ツールを一切増やしたくない**」を優先した。

`run` は `set -euo pipefail` で fail-fast、`once` ヘルパで依存の重複排除、実行コマンドの表示（既定 ON・`-q` で抑止）まで
入って**追加インストール不要**。`./run` 1本で dev / test / db-migrate / lint / css-check まで賄える。

**`make` は「どこにでもある」以外の理由で新規採用する必要はない**（タブ必須・行ごとに別シェル起動・文字列処理の癖）。

## 比較表

| ツール | 定義ファイル | 形式 | 依存解決 | 引数渡し | 実体 | インストール |
|---|---|---|---|---|---|---|
| **make** | `Makefile` | Make 独自(タブ必須) | ネイティブ(前提条件) | `make t VAR=val` | C / 大抵プリイン | 標準(macOS は 3.81 と古い) |
| **just** | `Justfile` | Make 風を現代化 | `test: build` | `just t Alice`(位置・既定値) | Rust / 単一バイナリ | `brew install just` |
| **Task** | `Taskfile.yml` | YAML | `deps: [build]`(既定で並列) | `task t VAR=val` / `-- args` | Go / 単一バイナリ | `brew install go-task` |
| **mise** | `mise.toml` | TOML(+スクリプト) | `depends` | `usage` spec → `$usage_*` | Rust / 単一バイナリ | `brew install mise` |
| **mage** | `magefile.go` | **Go コード** | `mg.Deps(Build)` | 型付き関数引数 | Go(要 go.mod) | `brew install mage` |
| **cargo-make** | `Makefile.toml` | TOML | `dependencies` | `${1}` / `${@}` | Rust / cargo 統合 | `cargo install cargo-make` |
| **mask** | `maskfile.md` | **Markdown** | 組み込みなし(`$MASK` で自己呼出) | 見出しの `(name)` | Rust / 単一バイナリ | `brew install mask` |
| **xc** | `README.md` | **Markdown** | `Requires: build` | `$1`(位置) | Go / 単一バイナリ | `go install .../xc/cmd/xc@latest` |
| **bash** | `run`(自作・採用) | **シェル関数 + ディスパッチャ** | 関数呼出 + `once` で重複排除 | `$1` / `${1:-既定}` | bash のみ | **不要**（`./run`） |

## 選び方（一般論）

```
追加インストールが一切できない         → make
ツールを増やしたくない(最小・自作)     → bash 関数ディスパッチャ   ★lastshot はこれ
とにかく素直なコマンドランナーが欲しい  → just                     ★まず試すならこれ
CI / クロスプラットフォーム / 増分実行  → Task
ツールのバージョン管理ごと一本化        → mise
タスクのロジックが複雑・Go プロジェクト → mage
Rust(cargo)で多機能に                  → cargo-make
README をそのまま実行可能にしたい       → xc / mask
```

## 各ツールの性格（要点だけ）

- **just** — Make の構文を素直にコマンドランナー専用へ作り直した決定版。タブ不要・引数とデフォルト値・
  `just --list`・`.env` 読み込み。**学習コスト最小**。増分ビルド（ファイル依存）はしない純粋なランナー。
- **Task** — YAML 定義で CI と相性が良い。`sources`/`generates` で増分実行できる（ビルドツール的）。
  YAML の冗長さと既定でコマンドがエコーされるのが好み次第。
- **mise** — タスクランナー兼 **asdf 代替のツールバージョン管理 + env 管理**。「必要な CLI を mise で固定 →
  タスクは just」の併用も筋が良い。tasks は新しめで引数まわりの仕様が変遷中（`arg()` → `usage` 方式）。
- **mage** — タスクを **Go コードで書く**。型付き引数・並列が普通のコードで書けるが、Go ランタイム + go.mod +
  初回コンパイルが要り軽いタスクには重い。
- **cargo-make** — Rust/cargo 統合。CI 用の組み込みタスクが豊富だが機能過多で冗長・起動がやや遅い。
- **mask / xc** — **Markdown がそのままタスク定義**（xc は README.md 自体）。「動くドキュメント」になるのが強み。
  mask は依存解決が組み込みでない（本文で `$MASK build` を呼ぶ）・失敗を握りつぶす弱点がある。
- **bash 関数ディスパッチャ（採用）** — `run` に関数を並べ、末尾の `"${@:-help}"` で `./run build` → 関数 build を呼ぶ。
  弱点は bash 依存（**macOS 標準は 3.2 と古く `declare -A` 不可**→ 文字列方式で回避。
  5.x 推奨の手順は [`./bash-setup.md`](./bash-setup.md)）と、自分で少し書くこと。

## 検証環境

macOS (Apple Silicon) / make 3.81 / just 1.52.0 / Task 3.51.1 / mise 2026.5.16 /
mage 1.17.2 / cargo-make 0.37.24 / mask 0.11.7 / xc 0.9.0。
bash 版は macOS 標準の bash 3.2.57 で動作確認（連想配列を使わず 3.2/POSIX sh 互換）。
