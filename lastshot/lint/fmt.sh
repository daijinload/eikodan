#!/usr/bin/env bash
# lastshot の fmt 一括書き込み（push 前用）。check.sh の対（あちらは --check の読み取り専用）。
#
# 使い分け:
#   dev ループ中 … 触ったクレート/ファイルだけ個別に（例: cargo fmt -p <crate>）。
#                  全クレート一括は「触っていないクレートまで」再ビルドを誘発するため避ける。
#   push 前      … これ（./run fmt）で一括整形 → ./run lint を緑にする。どうせ release ビルド/
#                  最終確認で全体を1回ビルドするので、その流れに相乗りさせれば追加コストはない。
#
# 補足: cargo fmt は「差分のあるファイルだけ」書き戻す（整形済みは1バイトも触らない＝mtime据え置き
#       ＝そのクレートは再ビルドされない）。なので二度目以降の ./run fmt は実質ノーコスト。
#
# 使い方:  ./run fmt   （= bash lint/fmt.sh。どこから叩いてもよい）
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1 # lastshot ルートへ

LINT=lint
OXFMT="$LINT/.lint-tools/node_modules/.bin/oxfmt"
[[ -x "$OXFMT" ]] || OXFMT="$(command -v oxfmt || true)"

step() {
  echo
  echo "==> $*"
}

step "[1/5] rustfmt（cargo fmt --all）"
cargo fmt --all

step "[2/5] oxfmt（--write / browser・.lint-tools は除外）"
if [[ -n "$OXFMT" ]]; then
  "$OXFMT" -c "$LINT/.oxfmtrc.json" . '!browser/**' '!**/.lint-tools/**'
else
  echo "skip: oxfmt 無し（./run lint-setup）"
fi

step "[3/5] buf format -w"
(cd crates/schema/proto && buf format -w)

step "[4/5] shfmt -i 2 -ci -w"
shfmt -i 2 -ci -w run assets/*.sh "$LINT"/*.sh

step "[5/5] sqlfluff fix"
sqlfluff fix --config "$LINT/.sqlfluff" migrations/*.sql

echo
echo "done. 確認は ./run lint（fmt 系は緑になるはず。clippy 指摘は別途手で直す）"
