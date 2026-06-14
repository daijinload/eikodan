#!/usr/bin/env bash
# lastshot の fmt/lint 手動ゲート（読み取り専用。書き込みはしない）。
# push 前に節目で回す想定（css-check と同じ運用。pre-commit は使わない）。
# 種別ごとに最適なツールを当てる（1本で全部は賄えない。根拠は ../../lint-format/）:
#   1) rustfmt   cargo fmt --all --check                   （../rustfmt.toml / workspace 全体）
#   2) clippy    cargo clippy --all-targets -- -D warnings  （workspace 全体）
#   3) oxfmt     TOML/YAML/MD/CSS/HTML/JSON の整形チェック  （./.oxfmtrc.json / repo 全体・.gitignore 尊重）
#   4) buf       proto の lint + format チェック            （../crates/schema/proto, buf.yaml）
#   5) shell     shfmt -i 2 -ci -d + shellcheck            （run, assets/*.sh, lint/*.sh）
#   6) sql       sqlfluff lint                             （./.sqlfluff / postgres, migrations/*.sql）
#
# 使い方:  ./run lint   （= bash lint/check.sh。どこから叩いてもよい）
# 整形（書き込み）の対は lint/fmt.sh（= ./run fmt）。使い分け:
#   dev ループ中 … 触ったクレート/ファイルだけ個別に（cargo fmt -p <crate> など。
#                  触っていないクレートまで再ビルドさせないため。CLAUDE.md 参照）。
#   push 前      … ./run fmt で一括整形 → この ./run lint を緑にする。
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1 # lastshot ルートへ（set -e は使わず明示的に抜ける）

LINT=lint
OXFMT="$LINT/.lint-tools/node_modules/.bin/oxfmt"
[[ -x "$OXFMT" ]] || OXFMT="$(command -v oxfmt || true)"

fail=0
step() {
  echo
  echo "==> $*"
}

step "[1/6] rustfmt（cargo fmt --all --check）"
cargo fmt --all --check || fail=1

step "[2/6] clippy（-D warnings）"
cargo clippy --all-targets -- -D warnings || fail=1

step "[3/6] oxfmt --check（TOML/YAML/MD/CSS/HTML/JSON・.gitignore 尊重）"
# browser/ は自己完結の別関心事（Playwright）なので対象外。'!' 除外は cwd 基準で効く
# （.oxfmtrc.json の ignorePatterns は設定ファイルの場所基準になり root からだと外しにくい）。
if [[ -n "$OXFMT" ]]; then
  "$OXFMT" --check -c "$LINT/.oxfmtrc.json" . '!browser/**' '!**/.lint-tools/**' || fail=1
else
  echo "ERROR: oxfmt が無い。./run lint-setup" >&2
  fail=1
fi

step "[4/6] buf（lint + format）"
(cd crates/schema/proto && buf lint && buf format --diff --exit-code) || fail=1

step "[5/6] shell（shfmt -d + shellcheck）"
# -i 2: 2スペース indent（shfmt 既定のタブにしない）。-ci: case 分岐も字下げ（既定の左寄せにしない）。
shfmt -i 2 -ci -d run assets/*.sh "$LINT"/*.sh || fail=1
shellcheck run assets/*.sh "$LINT"/*.sh || fail=1

step "[6/6] sql（sqlfluff lint）"
# lint は整形逸脱も含めて検査する（書き込みは sqlfluff fix）。
sqlfluff lint --config "$LINT/.sqlfluff" migrations/*.sql || fail=1

echo
if [[ "$fail" -eq 0 ]]; then
  echo "OK: fmt/lint 全通過"
else
  echo "NG: 上の指摘を解消すること（整形は触ったファイルにだけ個別に当てる。先頭コメント参照）"
fi
exit "$fail"
