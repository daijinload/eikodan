#!/usr/bin/env bash
# task-runners/demo.sh
# 9 通りの Make 代替(8 ツール + 自作 bash ディスパッチャ)で、同じ hello / test / greet を順に実行する。
# どれも同じ出力(Hello / building→testing / Hi, Alice!)になることを横並びで確認できる。
#
#   ./demo.sh
#
# mage / xc は go install 先 (GOPATH/bin) に入るので PATH に足しておく。
set -u
export PATH="$PATH:$(go env GOPATH 2>/dev/null)/bin"
cd "$(dirname "$0")"

section() { printf '\n\033[1m########## %s ##########\033[0m\n' "$1"; }

section "make"
( cd make       && make hello       && make test       && make greet NAME=Alice )

section "just"
( cd just       && just hello       && just test       && just greet Alice )

section "Task (go-task)"
( cd task       && task hello       && task test       && task greet NAME=Alice )

section "mise"
( cd mise       && mise trust -q 2>/dev/null; mise run hello && mise run test && mise run greet Alice )

section "mage"
( cd mage       && mage hello       && mage test       && mage greet Alice )

section "cargo-make"
( cd cargo-make && makers hello     && makers test     && makers greet Alice )

section "mask"
( cd mask       && mask hello       && mask test       && mask greet Alice )

section "xc"
( cd xc         && xc hello         && xc test         && xc greet Alice )

section "bash (関数ディスパッチャ・自作)"
( cd bash       && ./run hello      && ./run test      && ./run greet Alice )
( cd bash       && echo '-- ./run -q greet Alice (-q で実行コマンドを抑止) --' && ./run -q greet Alice )

printf '\n\033[1m全ツール完了。\033[0m\n'
