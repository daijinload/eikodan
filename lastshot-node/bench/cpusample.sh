#!/usr/bin/env bash
# サーバプロセスの CPU 使用率を一定間隔でサンプリングして outfile に追記する。
# kill されるまでループするので、bench.sh が oha 計測の間だけ起動→停止する。
#
#   cpusample.sh <pgrep-pattern> <outfile> [interval-sec]
#
# 各行 = pattern にマッチした全 pid の %cpu 合計（マルチスレッド/cluster は 100% 超もあり。
# 例: 750 = 7.5 コアぶん）。summary.mjs がこれを平均し CPU秒/1k req を出す。
set -uo pipefail
pattern="$1"
outfile="$2"
interval="${3:-0.5}"
: >"$outfile"
while true; do
  pids=$(pgrep -f "$pattern" 2>/dev/null | tr '\n' ',' | sed 's/,$//')
  if [ -n "$pids" ]; then
    ps -o %cpu= -p "$pids" 2>/dev/null | awk '{s+=$1} END{print s+0}' >>"$outfile"
  fi
  sleep "$interval"
done
