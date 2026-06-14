#!/usr/bin/env bash
# CSS最終確認ゲート（コミット/push 前の「節目」に1回だけ回す。日常開発では不要）。
#
# 日常はCDNでビルドゼロ。だが CDN はDOMから何でも生成するので「devで動くが本番パージで崩れる」
# クラスを見逃す。それを手元を離れる前に確定検査するのがこのスクリプト:
#   1) クリーンビルド (--watch なし) … app.css をフル生成。追加も削除も毎回パージ確定。
#   2) semgrep                       … パージで黙って消える危険パターンを静的検出。
# どちらかが落ちたら非ゼロ終了する（手動ゲート / CI にそのまま使える）。
#
# 使い方:  bash assets/check-css.sh        （lastshot/ 直下・どこからでも可。./run css-check 推奨）
# 目視確認は通過後に:  CSS=built cargo run -p app   （release往復なし＝再ビルドしない）
set -euo pipefail
cd "$(dirname "$0")/.."   # lastshot/ へ

bin=assets/tailwindcss
in=assets/input.css
out=crates/app/static/app.css

if [[ ! -x "$bin" ]]; then
  echo "ERROR: $bin が無い。先に取得: bash assets/setup-css.sh" >&2
  exit 1
fi

echo "==> [1/2] clean build (purge確定): $out"
"$bin" -i "$in" -o "$out"   # --watch なし = 毎回フルパージ（消したクラスも確実に落ちる）

echo "==> [2/2] semgrep: パージで消える危険パターン検査"
if ! command -v semgrep >/dev/null 2>&1; then
  echo "ERROR: semgrep が無い。導入: uv tool install semgrep  (CIと同手段。or brew install semgrep)" >&2
  exit 1
fi
# --error: 検出があれば非ゼロ終了。crates 配下のテンプレHTMLと .rs を走査。
semgrep scan --config assets/semgrep --error crates

echo "OK: クリーンビルド + semgrep 通過。最終目視は  CSS=built cargo run -p app"
