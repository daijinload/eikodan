#!/usr/bin/env bash
# lastshot のテスト配置規約を機械的に検査する（読み取り専用）。
# 規約とその理由は ../docs/testing.md。ここはその結論を実ファイルに当てる配線。
#
#   1) src/ にテストコードを置かない
#      … crates/*/src/ に #[test] / #[cfg(test)] が現れないこと。
#        実装とテストが同じ diff に並ぶのを構造的に禁止する（testing.md「なぜ分離するか」）。
#   2) 仕様テストには仕様書が隣にある
#      … spec_<name>.<ext> があれば spec_<name>.md も同じディレクトリにあること。
#   3) 迷子の仕様書を残さない
#      … spec_<name>.md があれば spec_<name>.<ext> も同じディレクトリにあること。
#   4) browser/tests に spec_ 以外のテストを置かない
#      … playwright.config.ts の testMatch を spec_*.ts に絞っている＝プレフィックスが無いと
#        **エラーにならず黙って実行されない**ので、ここで弾く。
#
# 使い方: ./run spec-check （= bash lint/check-spec-pairs.sh。どこから叩いてもよい）
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1 # lastshot ルートへ（set -e は使わず明示的に抜ける）

fail=0
step() {
  echo
  echo "==> $*"
}

step "[1/4] src/ にテストコードが無いこと"
# 実装ファイルに #[test] / #[cfg(test)] が混ざっていないか。
# どうしても private を直接テストする必要が出たら、testing.md の逃げ道
# （#[path] で本体を tests/ 側に置く）を使い、この検査を通せる形にする。
if hits=$(grep -rnE '^\s*#\[(cfg\(test\)|test)\]' crates/*/src/ 2>/dev/null) && [[ -n "$hits" ]]; then
  echo "NG: 実装ファイルにテストコードがある（tests/ へ移すこと）:"
  echo "$hits"
  fail=1
else
  echo "OK"
fi

# 検査対象を「ディレクトリ:仕様テストの拡張子」で持つ。Rust は .rs、browser は Playwright なので .ts。
# glob だけで済ませる（mapfile は bash4+ 依存なので使わない）。存在しない分は下のループで弾く。
targets=()
for dir in crates/*/tests tests-http/tests; do targets+=("$dir:rs"); done
targets+=("browser/tests:ts")

step "[2/4] spec_* のテストに対応する spec_*.md があること"
missing_md=0
for target in "${targets[@]}"; do
  dir="${target%:*}"
  ext="${target##*:}"
  [[ -d "$dir" ]] || continue
  for src in "$dir"/spec_*."$ext"; do
    [[ -e "$src" ]] || continue # glob 不一致（該当なし）はスキップ
    md="${src%."$ext"}.md"
    [[ -f "$md" ]] && continue
    echo "NG: 仕様書が無い: $src → $md を書くこと"
    missing_md=1
  done
done
[[ "$missing_md" -eq 0 ]] && echo "OK" || fail=1

step "[3/4] spec_*.md に対応するテストがあること"
missing_src=0
for target in "${targets[@]}"; do
  dir="${target%:*}"
  ext="${target##*:}"
  [[ -d "$dir" ]] || continue
  for md in "$dir"/spec_*.md; do
    [[ -e "$md" ]] || continue
    src="${md%.md}.$ext"
    [[ -f "$src" ]] && continue
    echo "NG: 仕様に対応するテストが無い: $md → $src を書くこと"
    missing_src=1
  done
done
[[ "$missing_src" -eq 0 ]] && echo "OK" || fail=1

step "[4/4] browser/tests のテストが全て spec_ で始まること"
# testMatch が '**/spec_*.ts' なので、プレフィックスの無いファイルは黙って実行対象から外れる
# （ブラウザ層に unit_ は置かない ── 純粋ロジックのユニットは Rust 側の担当）。
stray_ts=0
if [[ -d browser/tests ]]; then
  for ts in browser/tests/*.ts; do
    [[ -e "$ts" ]] || continue
    [[ "$(basename "$ts")" == spec_* ]] && continue
    echo "NG: testMatch から外れて実行されない: $ts → spec_<name>.ts に改名すること"
    stray_ts=1
  done
fi
[[ "$stray_ts" -eq 0 ]] && echo "OK" || fail=1

echo
if [[ "$fail" -eq 0 ]]; then
  echo "OK: テスト配置規約に違反なし"
else
  echo "NG: 上の指摘を解消すること（規約の理由は docs/testing.md）"
fi
exit "$fail"
