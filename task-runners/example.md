# 各ツールの 設定 → コマンド → 実行結果

8 つの Make 代替ツール + 自作 bash（計 9 通り）に、まったく同じ 5 タスク（`hello` / `build` / `test`=build に依存 / `greet`=引数つき / `clean`）を実装して動かした記録。
実行結果は実際の生出力（各ツール独自のログ表示も含む）。`./demo.sh` で全部まとめて再現できる。

検証環境: macOS (Apple Silicon) / make 3.81 / just 1.52.0 / Task 3.51.1 / mise 2026.5.16 / mage 1.17.2 / cargo-make 0.37.24 / mask 0.11.7 / xc 0.9.0 / bash 3.2.57

---

## 1. make — `make/Makefile`

### 設定

```makefile
NAME ?= World

.PHONY: hello build test greet clean

hello:                 ## あいさつ
	@echo "Hello from make!"

build:                 ## ダミービルド
	@echo "==> building..."

test: build            ## build に依存
	@echo "==> testing..."

greet:                 ## 引数つき(変数 NAME)
	@echo "Hi, $(NAME)!"

clean:                 ## 後片付け
	@echo "==> cleaning..."
```

### コマンドと実行結果

```
$ make hello
Hello from make!
$ make test
==> building...
==> testing...
$ make greet NAME=Alice
Hi, Alice!
```

依存は `test: build`。引数は `VAR=val` 形式で `$(NAME)` に入る。`@` を付けないとコマンド自体もエコーされる。

---

## 2. just — `just/Justfile`

### 設定

```just
# 引数なしで実行したときの既定レシピ
default:
    @just --list

hello:
    @echo "Hello from just!"

build:
    @echo "==> building..."

test: build
    @echo "==> testing..."

greet name="World":
    @echo "Hi, {{name}}!"

clean:
    @echo "==> cleaning..."
```

### コマンドと実行結果

```
$ just hello
Hello from just!
$ just test
==> building...
==> testing...
$ just greet Alice
Hi, Alice!
```

依存は `test: build`、引数は `greet name="World":` の位置引数（デフォルト値つき）。タブ不要で Make より素直。

---

## 3. Task (go-task) — `task/Taskfile.yml`

### 設定

```yaml
version: '3'

vars:
  NAME: World

tasks:
  default:
    cmds:
      - task --list
    silent: true

  hello:
    desc: あいさつ
    cmds:
      - echo "Hello from Task!"

  build:
    desc: ダミービルド
    cmds:
      - echo "==> building..."

  test:
    desc: build に依存
    deps: [build]
    cmds:
      - echo "==> testing..."

  greet:
    desc: 引数つき(変数 NAME)
    cmds:
      - echo "Hi, {{.NAME}}!"

  clean:
    desc: 後片付け
    cmds:
      - echo "==> cleaning..."
```

### コマンドと実行結果

```
$ task hello
task: [hello] echo "Hello from Task!"
Hello from Task!
$ task test
task: [build] echo "==> building..."
==> building...
task: [test] echo "==> testing..."
==> testing...
$ task greet NAME=Alice
task: [greet] echo "Hi, Alice!"
Hi, Alice!
```

依存は `deps: [build]`、引数は `VAR=val` で `{{.NAME}}`。既定で `task: [name] <cmd>` とコマンドをエコーする（消すなら `silent: true`）。

---

## 4. mise — `mise/mise.toml`

### 設定

```toml
[tasks.hello]
description = "あいさつ"
run = 'echo "Hello from mise!"'

[tasks.build]
description = "ダミービルド"
run = 'echo "==> building..."'

[tasks.test]
description = "build に依存"
depends = ["build"]
run = 'echo "==> testing..."'

[tasks.greet]
description = "引数つき(usage で定義、$usage_name で受け取る)"
usage = 'arg "name" default="World"'
run = 'echo "Hi, $usage_name!"'

[tasks.clean]
description = "後片付け"
run = 'echo "==> cleaning..."'
```

### コマンドと実行結果

```
$ mise run hello
[hello] $ echo "Hello from mise!"
Hello from mise!
$ mise run test
[build] $ echo "==> building..."
==> building...
[test] $ echo "==> testing..."
==> testing...
Finished in 8.4ms
$ mise run greet Alice
[greet] $ echo "Hi, $usage_name!"
Hi, Alice!
```

依存は `depends`、引数は `usage` 仕様で定義して `$usage_name` で受け取る（古い `{{arg(...)}}` テンプレートは非推奨）。
初回は `mise trust` が必要。タスクランナーに加えてツールのバージョン管理も兼ねるのが強み。

---

## 5. mage — `mage/magefile.go`（+ `go.mod` / `go.sum`）

### 設定

```go
//go:build mage

package main

import (
	"fmt"

	"github.com/magefile/mage/mg"
)

// Hello はあいさつする。
func Hello() {
	fmt.Println("Hello from mage!")
}

// Build はダミービルド。
func Build() {
	fmt.Println("==> building...")
}

// Test は Build に依存する。
func Test() {
	mg.Deps(Build)
	fmt.Println("==> testing...")
}

// Greet は引数 name を受け取る(型付き)。
func Greet(name string) {
	fmt.Printf("Hi, %s!\n", name)
}

// Clean は後片付け。
func Clean() {
	fmt.Println("==> cleaning...")
}
```

`go.mod`（`go mod init` + `go mod tidy` で生成）:

```
module eikodan/task-runners/mage

go 1.26.3

require github.com/magefile/mage v1.17.2
```

### コマンドと実行結果

```
$ mage hello
Hello from mage!
$ mage test
==> building...
==> testing...
$ mage greet Alice
Hi, Alice!
```

タスクは Go コードで書く。依存は `mg.Deps(Build)`、引数は型付き関数引数 `Greet(name string)`。
シェル非依存でクロスプラットフォーム。反面 go.mod と初回コンパイルが要る。

---

## 6. cargo-make — `cargo-make/Makefile.toml`

### 設定

```toml
[tasks.hello]
description = "あいさつ"
command = "echo"
args = ["Hello from cargo-make!"]

[tasks.build]
description = "ダミービルド"
command = "echo"
args = ["==> building..."]

[tasks.test]
description = "build に依存"
dependencies = ["build"]
command = "echo"
args = ["==> testing..."]

[tasks.greet]
description = "引数つき(${1}、無ければ World)"
script = ['echo "Hi, ${1:-World}!"']

[tasks.clean]
description = "後片付け"
command = "echo"
args = ["==> cleaning..."]
```

### コマンドと実行結果

```
$ makers hello
[cargo-make] INFO - makers 0.37.24
[cargo-make] INFO - 
[cargo-make] INFO - Build File: Makefile.toml
[cargo-make] INFO - Task: hello
[cargo-make] INFO - Profile: development
[cargo-make] INFO - Execute Command: "echo" "Hello from cargo-make!"
Hello from cargo-make!
[cargo-make] INFO - Build Done in 0.09 seconds.
$ makers test
[cargo-make] INFO - makers 0.37.24
[cargo-make] INFO - 
[cargo-make] INFO - Build File: Makefile.toml
[cargo-make] INFO - Task: test
[cargo-make] INFO - Profile: development
[cargo-make] INFO - Execute Command: "echo" "==> building..."
==> building...
[cargo-make] INFO - Execute Command: "echo" "==> testing..."
==> testing...
[cargo-make] INFO - Build Done in 0.09 seconds.
$ makers greet Alice
[cargo-make] INFO - makers 0.37.24
[cargo-make] INFO - 
[cargo-make] INFO - Build File: Makefile.toml
[cargo-make] INFO - Task: greet
[cargo-make] INFO - Profile: development
[cargo-make] INFO - Running Task: greet
Hi, Alice!
[cargo-make] INFO - Build Done in 0.09 seconds.
```

`makers <task>`（または `cargo make <task>`）。依存は `dependencies`、引数は末尾の値が `${1}` に入る。
既定で `[cargo-make] INFO - ...` のログを多めに出す。CI 用の組み込みタスクが豊富で多機能。

---

## 7. mask — `mask/maskfile.md`

### 設定

````markdown
# mask sample

## hello

> あいさつ

```sh
echo "Hello from mask!"
```

## build

> ダミービルド

```sh
echo "==> building..."
```

## test

> build に依存($MASK で自分自身を呼ぶ)

```sh
$MASK build
echo "==> testing..."
```

## greet (name)

> 引数つき(見出しの (name) が変数になる)

```sh
echo "Hi, $name!"
```

## clean

> 後片付け

```sh
echo "==> cleaning..."
```
````

### コマンドと実行結果

```
$ mask hello
Hello from mask!
$ mask test
==> building...
==> testing...
$ mask greet Alice
Hi, Alice!
```

Markdown の見出しがそのままタスク。引数は見出しの `(name)` が変数 `$name` になる。
依存解決は組み込みでないので、本文で `$MASK build`（自分自身）を呼んで表現する。

---

## 8. xc — `xc/README.md`

### 設定

タスクは `Tasks` 見出し（既定）の配下に 1 段下げて書く。

````markdown
# xc sample

## Tasks

### hello

あいさつ

```
echo "Hello from xc!"
```

### build

ダミービルド

```
echo "==> building..."
```

### test

build に依存。

Requires: build

```
echo "==> testing..."
```

### greet

引数つき($1、無ければ World)。

```
echo "Hi, ${1:-World}!"
```

### clean

後片付け

```
echo "==> cleaning..."
```
````

### コマンドと実行結果

```
$ xc hello
+ echo 'Hello from xc!'
Hello from xc!
$ xc test
+ echo '==> building...'
==> building...
+ echo '==> testing...'
==> testing...
$ xc greet Alice
+ echo 'Hi, Alice!'
Hi, Alice!
```

README.md 自体がタスク定義。依存は `Requires: build`、引数は末尾の値が `$1` に入る。
実行時に `+ <cmd>`（シェルの set -x 相当）でコマンドを表示する。

---

## 9. bash（関数ディスパッチャ・自作）— `bash/run`

8 ツールを比較した結論として「ツールを増やさず弱点も無い」最小解を自作したもの。
mask の弱点(途中失敗の握りつぶし・共通依存の二度走り)を fail-fast と `once` で潰し、
実行コマンドは既定で表示(`-q`/`Q=1` で抑止)。macOS 標準の bash 3.2 でも動く。

### 設定

```bash
#!/usr/bin/env bash
# task-runners/bash/run
#
# なぜ作った: Make 代替を 8 つ比較した結論。just/Task 等は「もう一つツールを入れる」、
#   mask は途中の失敗を握りつぶす・共通依存を 2 回走らせる・2022 年で更新停止、と難あり。
#   bash だけで完結し、fail-fast(set -euo pipefail)と依存の重複排除(once)で
#   その弱点を両方潰せる = 追加インストール不要で一番シンプルに正しく動く最小解。
#
# ツールを一切入れない「bash 関数 + ディスパッチャ」パターン。
# 末尾の "${@:-help}" が肝で、`./run build` → 関数 build を呼ぶ。
#
#   ./run hello         # 実行するコマンドを + 付きで表示してから実行(既定ON)
#   ./run test          # 依存は関数を呼ぶだけ(once で重複排除)
#   ./run greet Alice
#   ./run -q build      # -q / Q=1 でコマンド表示を抑止(結果だけ)
#   ./run               # 引数なし → help(一覧)
set -euo pipefail       # fail-fast: 途中で失敗したら即停止・exit!=0

# 依存を「一度だけ」実行するためのメモ化ヘルパ(mask の prep 2回問題を防ぐ)
# 連想配列(declare -A)は bash4+ 限定で macOS 標準の bash3.2 で落ちるため、
# 文字列メンバーシップで判定して 3.2/POSIX sh でも動くようにする。
_done=" "
once() {
  { _x=$-; set +x; } 2>/dev/null            # once 自身はトレースしない(この行も /dev/null へ)
  case "$_done" in *" $1 "*) case "$_x" in *x*) set -x;; esac; return 0;; esac
  _done="$_done$1 "
  case "$_x" in *x*) set -x;; esac           # トレース復帰してから依存タスクを実行
  "$1"
}

hello() {            # あいさつ
  echo "Hello from bash!"
}

build() {            # ダミービルド
  echo "==> building..."
}

test() {             # build に依存
  once build
  echo "==> testing..."
}

greet() {            # 引数つき(無ければ World)
  local name="${1:-World}"
  echo "Hi, $name!"
}

clean() {            # 後片付け
  echo "==> cleaning..."
}

help() {             # タスク一覧
  echo "usage: $0 <task> [args]"
  echo "tasks:"
  grep -E '^[a-z_]+\(\) +\{ +#' "$0" \
    | sed -E 's/\(\) +\{ +# */\t/' | sort | sed 's/^/  /'
}

# 実行するコマンドを + 付き(展開済み)で表示する(既定ON)。-q または Q=1 で抑止。
_trace=1
[ "${Q:-0}" = 1 ] && _trace=0
[ "${1:-}" = "-q" ] && { shift; _trace=0; }
[ "${1:-help}" = "help" ] && _trace=0          # 一覧(help)はトレースしない
[ "$_trace" = 1 ] && { PS4='+ '; set -x; }

# ディスパッチ: 引数が無ければ help、あれば $1 と同名の関数を実行
"${@:-help}"
```

### コマンドと実行結果

```
$ ./run hello
+ hello
+ echo 'Hello from bash!'
Hello from bash!
$ ./run test
+ test
+ once build
+ build
+ echo '==> building...'
==> building...
+ echo '==> testing...'
==> testing...
$ ./run greet Alice
+ greet Alice
+ local name=Alice
+ echo 'Hi, Alice!'
Hi, Alice!
$ ./run -q greet Alice
Hi, Alice!
```

実行コマンドが既定で `+` 付き(変数展開済み)で出るので「何が走ったか」が常に見える。
依存は関数呼出、引数は `$1`、fail-fast は `set -euo pipefail`、依存の重複排除は `once`、抑止は `-q`/`Q=1`。
