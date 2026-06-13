#!/usr/bin/env bash
# CSSビルド一式（初回だけ）。Node不要 ── Tailwind v4 スタンドアロンCLI + daisyUI v5 の単体ファイルを落とす。
# これらは .gitignore 済み（プラットフォーム依存バイナリ + リリース成果物）。
#
# 使い方:  bash assets/setup-css.sh   (リポジトリ直下=fastweb/ で実行)
set -euo pipefail
cd "$(dirname "$0")"   # assets/ に移動

# OS/Arch に合う Tailwind バイナリ名を決める
uname_s="$(uname -s)"; uname_m="$(uname -m)"
case "$uname_s/$uname_m" in
  Darwin/arm64) bin="tailwindcss-macos-arm64" ;;
  Darwin/x86_64) bin="tailwindcss-macos-x64" ;;
  Linux/aarch64) bin="tailwindcss-linux-arm64" ;;
  Linux/x86_64) bin="tailwindcss-linux-x64" ;;
  *) echo "unsupported platform: $uname_s/$uname_m" >&2; exit 1 ;;
esac

echo "downloading $bin ..."
curl -fsSL -o tailwindcss "https://github.com/tailwindlabs/tailwindcss/releases/latest/download/$bin"
chmod +x tailwindcss

echo "downloading daisyUI bundle ..."
curl -fsSL -O "https://github.com/saadeghi/daisyui/releases/latest/download/daisyui.mjs"
curl -fsSL -O "https://github.com/saadeghi/daisyui/releases/latest/download/daisyui-theme.mjs"

echo "done. generate once with:"
echo "  ./assets/tailwindcss -i assets/input.css -o crates/app/static/app.css"
