#!/usr/bin/env bash
# lastshot の lint/fmt ツール一式（初回だけ）。種別ごとに最適なツールが違うので1本では賄えない
# （根拠と単体デモは ../../lint-format/ showcase）。ここは lastshot の実ファイルに当てる本番配線。
#   rustfmt / clippy … ツールチェーン同梱（rustup component で確認。設定は ../rustfmt.toml = workspace ルート）
#   oxfmt            … Rust以外（TOML/YAML/MD/CSS/HTML/JSON）の整形。napi 製で node が要るため
#                      lint/.lint-tools（.gitignore 済み）にローカル固定する。設定は ./.oxfmtrc.json
#   buf              … proto の lint / format（単一バイナリ）。設定は ../crates/schema/proto/buf.yaml
#   shfmt/shellcheck … shell の format / lint（単一バイナリ）
#   sqlfluff         … SQL の lint / format（Python 製。brew が依存ごと配布）。設定は ./.sqlfluff
#
# 使い方:  ./run lint-setup   （= bash lint/setup.sh。どこから叩いてもよい）
set -euo pipefail
cd "$(dirname "$0")" # lint/ へ

OXFMT_VERSION=0.54.0

echo "==> rustfmt / clippy（toolchain 同梱を確認）"
rustup component add rustfmt clippy >/dev/null 2>&1 || true
cargo fmt --version
cargo clippy --version

echo "==> oxfmt（node 製; lint/.lint-tools にローカル固定）"
if ! command -v npm >/dev/null 2>&1; then
  echo "ERROR: npm が無い。Node を入れてから再実行（oxfmt は napi 製で node が要る）。" >&2
  exit 1
fi
npm install --prefix .lint-tools "oxfmt@${OXFMT_VERSION}" --no-audit --no-fund
.lint-tools/node_modules/.bin/oxfmt --version

echo "==> buf / shfmt / shellcheck / sqlfluff（brew 配布）"
missing=()
for t in buf shfmt shellcheck sqlfluff; do
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
    echo "  sqlfluff:   https://docs.sqlfluff.com/ (or: pipx install sqlfluff)" >&2
    exit 1
  fi
fi

echo "done. ゲートは:  ./run lint   （= bash lint/check.sh）"
