#!/usr/bin/env bash
# fmt/lint 手動ゲート（samples/ 一式に対して通しで実行。1つでも落ちたら非ゼロ終了）。
# 種別ごとに最適なツールを当てる（1本で全部は賄えない）デモ:
#   1) rustfmt   cargo fmt --all --check               (samples/rust)
#   2) clippy    cargo clippy --all-targets -- -D warnings (samples/rust)
#   3) oxfmt     TOML/YAML/Markdown/CSS/HTML の整形チェック（.oxfmtrc.json）
#   4) buf       proto の lint + format チェック        (samples/proto)
#   5) shell     shfmt -i 2 -d + shellcheck（*.sh, samples/*.sh）
#   6) sql       sqlfluff lint（.sqlfluff / postgres dialect, samples/*.sql）
#
# 使い方:  bash check-lint.sh   (lint-format/ 直下・どこからでも可)
# 整形を当てる（書き込み）には各ツールを個別に:
#   ( cd samples/rust && cargo fmt --all )
#   .lint-tools/node_modules/.bin/oxfmt .
#   ( cd samples/proto && buf format -w )
#   shfmt -i 2 -w ./*.sh samples/*.sh
#   sqlfluff fix samples/*.sql
set -uo pipefail
cd "$(dirname "$0")" || exit 1 # lint-format/ へ（set -e を使わないので明示的に抜ける）

fail=0
step() {
  echo
  echo "==> $*"
}

OXFMT=".lint-tools/node_modules/.bin/oxfmt"
[[ -x "$OXFMT" ]] || OXFMT="$(command -v oxfmt || true)"

step "[1/6] rustfmt（cargo fmt --all --check）"
(cd samples/rust && cargo fmt --all --check) || fail=1

step "[2/6] clippy（-D warnings）"
(cd samples/rust && cargo clippy --all-targets -- -D warnings) || fail=1

step "[3/6] oxfmt --check（TOML/YAML/Markdown/CSS/HTML）"
if [[ -n "$OXFMT" ]]; then
  "$OXFMT" --check . || fail=1
else
  echo "ERROR: oxfmt が無い。bash setup-lint.sh" >&2
  fail=1
fi

step "[4/6] buf（lint + format）"
(cd samples/proto && buf lint && buf format --diff --exit-code) || fail=1

step "[5/6] shell（shfmt -d + shellcheck）"
# -i 2: 2スペース indent（shfmt 既定のタブにしない）
shfmt -i 2 -d ./*.sh samples/*.sh || fail=1
shellcheck ./*.sh samples/*.sh || fail=1

step "[6/6] sql（sqlfluff lint）"
# lint は format 逸脱も含めて検査する（書き込みは sqlfluff fix / format）。
sqlfluff lint samples/*.sql || fail=1

echo
if [[ "$fail" -eq 0 ]]; then
  echo "OK: fmt/lint 全通過"
else
  echo "NG: 上の指摘を解消すること（整形は各ツールの --write/-w / 'buf format -w' / 'shfmt -w' / 'sqlfluff fix' で適用）"
fi
exit "$fail"
