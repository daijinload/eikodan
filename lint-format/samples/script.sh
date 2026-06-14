#!/usr/bin/env bash
# shfmt + shellcheck が整形/検査する shell サンプル。
set -euo pipefail

greet() {
  local name="${1:-world}"
  echo "Hello, ${name}!"
}

for n in lint format; do
  greet "$n"
done
