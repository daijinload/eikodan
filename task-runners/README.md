# task-runners

「Makefile の代わりに何を使うか」を、有名どころ 8 ツール + 自作の bash ディスパッチャ(計 9 通り)で**まったく同じタスクを書いて横並び比較**する。

各ツールのフォルダに同じ 5 タスクを実装済み:

| タスク | 内容 | 見せたい差分 |
|---|---|---|
| `hello` | あいさつを表示 | 最小のタスク定義 |
| `build` | ダミービルド | 単純なコマンド |
| `test`  | `==> testing...` | **build への依存解決** |
| `greet` | `Hi, <name>!` | **引数渡し + デフォルト値** |
| `clean` | 後片付け | — |

```sh
./demo.sh        # 9 通り全部で hello / test / greet を実行(全部同じ出力になる)
```

## 一行結論

**迷ったら `just`。** Make の正統な後継で、学習コストが一番低く・単一バイナリ・引数とデフォルト値が素直。
CI やクロスプラットフォームを重視するなら **Task**、ツールのバージョン管理ごと一本化したいなら **mise**、
README を「動くドキュメント」にしたいなら **xc / mask**、複雑なロジックを型安全に書きたい Go 系なら **mage**。
**`make` は「どこにでもある」以外の理由で新規採用する必要はない**(タブ問題・行ごとに別シェル・文字列処理の癖)。

このリポジトリ(Rust 中心・高速開発重視)なら **just が無難**。`mise` は既に入っていてツール固定も任せられるので、
「プロジェクトに必要な CLI を mise で固定 → タスクは just」の併用も筋が良い。

さらに **「ツールを一切増やしたくない」なら bash の関数 + ディスパッチャ(本リポジトリの [`bash/run`](./bash/run))が最小解**。
`set -euo pipefail` で fail-fast、`once` ヘルパで依存の重複排除を入れてあるので、mask の弱点(失敗の握りつぶし・共通依存の二度走り)も無い。追加インストール不要で約 30 行・全部読める。

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
| **xc** | `README.md` | **Markdown** | `Requires: build` | `$1`(位置) | Go / 単一バイナリ | `go install github.com/joerdav/xc/cmd/xc@latest` |
| **bash** | `run`(自作) | **シェル関数 + ディスパッチャ** | 関数呼出 + `once` で重複排除 | `$1` / `${1:-既定}` | bash のみ | **不要**(`./run`) |

### 「引数つき greet」だけ並べると差が見える

```sh
make:        greet:                  →  @echo "Hi, $(NAME)!"          # make greet NAME=Alice
just:        greet name="World":     →  @echo "Hi, {{name}}!"         # just greet Alice
Task:        greet: {cmds:[...]}     →  echo "Hi, {{.NAME}}!"         # task greet NAME=Alice
mise:        usage='arg "name" ...'  →  echo "Hi, $usage_name!"       # mise run greet Alice
mage:        func Greet(name string) →  fmt.Printf("Hi, %s!\n", name) # mage greet Alice
cargo-make:  script=['...']          →  echo "Hi, ${1:-World}!"       # makers greet Alice
mask:        ## greet (name)         →  echo "Hi, $name!"             # mask greet Alice
xc:          ### greet               →  echo "Hi, ${1:-World}!"       # xc greet Alice
bash:        greet() { ... }         →  echo "Hi, ${1:-World}!"       # ./run greet Alice
```

## それぞれの性格と向き

- **make** — どの環境にもある基準。本来は「ファイルのタイムスタンプ依存で増分ビルド」するツールで、コマンドランナーとして使うと `.PHONY` だらけ・タブ必須・行ごとに別シェル起動・文字列処理が独特、と摩擦が多い。新規なら避けてよいが「追加インストール不要」は唯一無二。
- **just** — Make の構文を素直にコマンドランナー専用へ作り直した決定版。タブ不要、引数とデフォルト値、`just --list`、`.env` 読み込み、わかりやすいエラー。**学習コスト最小**。増分ビルド(ファイル依存)はしない純粋なランナー。
- **Task** — YAML 定義で **CI と相性が良い**。`sources`/`generates` でファイル更新チェックして増分実行できる(=ビルドツール的)、`includes` で分割、クロスプラットフォーム。YAML の冗長さと既定でコマンドがエコーされるのが好み次第。
- **mise** — タスクランナーであると同時に **asdf 代替のツールバージョン管理 + env 管理**を一本化できるのが最大の強み。プロジェクトに必要な node/go/just 等を固定しつつタスクも回せる。tasks は新しめで、引数まわりの仕様が変遷中(古い `arg()` テンプレートは非推奨 → `usage` 方式へ)。
- **mage** — タスクを **Go コードで書く**。条件分岐・ループ・型付き引数・並列(`mg.Deps`)が全部 Go の普通のコードで書け、シェル非依存でクロスプラットフォーム。反面 Go ランタイム・go.mod・初回コンパイルが要り、軽いタスクには重い。Go プロジェクト向き。
- **cargo-make** — Rust/cargo 統合(`cargo make`)。CI 用の組み込みタスクが非常に豊富で、条件分岐・プラットフォーム別・duckscript と多機能。`makers` 単体でも使える。機能過多で冗長・起動が他よりやや遅い。
- **mask** — `maskfile.md` の **Markdown 見出しがそのままタスク**。コードブロックの言語を変えれば bash/python/node/ruby を混在でき、説明文が自然に書ける。依存解決は組み込みでない(本文で `$MASK build` を呼ぶ)。
- **xc** — **README.md 自体がタスク定義**。別ファイル不要で、書いた手順がそのまま「動くドキュメント」になる。`Requires:` で依存、末尾引数が `$1`。非常にマイナーだがコンセプトは強い。
- **bash(関数ディスパッチャ・自作)** — `run` に関数を並べ、末尾の `"${@:-help}"` で `./run build` → 関数 build を呼ぶだけ。**ツール追加ゼロ**で、依存は関数呼出、引数は `$1`、`set -euo pipefail` で fail-fast、`once` で依存の重複排除、実行コマンドの表示(既定 ON・`-q`/`Q=1` で抑止、一覧は非トレース)まで入る。弱点は bash 依存(macOS 標準は 3.2 と古く `declare -A` 不可 → 文字列方式で回避)と、自分で少し書くこと。実用のショートカット用途では最もシンプル。

## 選び方

```
追加インストールが一切できない         → make
ツールを増やしたくない(最小・自作)     → bash 関数ディスパッチャ(bash/run)
とにかく素直なコマンドランナーが欲しい  → just            ★まず試すならこれ
CI / クロスプラットフォーム / 増分実行  → Task
ツールのバージョン管理ごと一本化        → mise
タスクのロジックが複雑・Go プロジェクト → mage
Rust(cargo)で多機能に                  → cargo-make
README をそのまま実行可能にしたい       → xc / mask
```

## 各ツールを単体で動かす

```sh
cd make       && make greet NAME=Alice
cd just       && just greet Alice
cd task       && task greet NAME=Alice
cd mise       && mise run greet Alice      # 初回は `mise trust` が要る
cd mage       && mage greet Alice          # go.mod あり。GOPATH/bin を PATH に
cd cargo-make && makers greet Alice        # `cargo make greet Alice` でも可
cd mask       && mask greet Alice
cd xc         && xc greet Alice            # GOPATH/bin を PATH に
cd bash       && ./run greet Alice         # ツール不要。既定で実行コマンドも表示(-q で抑止)
```

> mage / xc は `go install` 先 (`$(go env GOPATH)/bin`) に入る。PATH に無ければ
> `export PATH="$PATH:$(go env GOPATH)/bin"` を足す(`demo.sh` は自動で足している)。

## 検証環境

macOS (Apple Silicon) / make 3.81 / just 1.52.0 / Task 3.51.1 / mise 2026.5.16 /
mage 1.17.2 / cargo-make 0.37.24 / mask 0.11.7 / xc 0.9.0。
bash 版は macOS 標準の bash 3.2.57 で動作確認(連想配列を使わず 3.2/POSIX sh 互換)。
