#!/usr/bin/env bash
# lint/fmt ツール一式（初回だけ）。
#   rustfmt / clippy … ツールチェーン同梱（rustup component で確認）
#   oxfmt            … Rust以外(TOML/YAML/MD/CSS/HTML)の整形。napi製で node が要るため
#                      .lint-tools（.gitignore済み）にローカル固定インストールする。
#   buf              … proto の lint / format（単一バイナリ）
#   shfmt/shellcheck … shell の format / lint（単一バイナリ）
#
# 使い方:  bash setup-lint.sh   (lint-format/ 直下・どこからでも可)
set -euo pipefail
cd "$(dirname "$0")" # lint-format/ へ

OXFMT_VERSION=0.54.0

echo "==> rustfmt / clippy（toolchain 同梱を確認）"
rustup component add rustfmt clippy >/dev/null 2>&1 || true
cargo fmt --version
cargo clippy --version

echo "==> oxfmt（node製; .lint-tools にローカル固定）"
if ! command -v npm >/dev/null 2>&1; then
  echo "ERROR: npm が無い。Node を入れてから再実行（oxfmt は napi 製で node が要る）。" >&2
  exit 1
fi
npm install --prefix .lint-tools "oxfmt@${OXFMT_VERSION}" --no-audit --no-fund
.lint-tools/node_modules/.bin/oxfmt --version

echo "==> buf / shfmt / shellcheck（単一バイナリ）"
missing=()
for t in buf shfmt shellcheck; do
  command -v "$t" >/dev/null 2>&1 || missing+=("$t")
done
if [[ ${#missing[@]} -gt 0 ]]; then
  if command -v brew >/dev/null 2>&1; then
    brew install "${missing[@]}"
  else
    echo "ERROR: 不足: ${missing[*]}。brew の無い環境では各自で導入を:" >&2
    echo "  buf:        https://buf.build/docs/installation" >&2
    echo "  shfmt:      https://github.com/mvdan/sh#shfmt" >&2
    echo "  shellcheck: https://github.com/koalaman/shellcheck#installing" >&2
    exit 1
  fi
fi

echo "done. ゲートは:  bash check-lint.sh"
