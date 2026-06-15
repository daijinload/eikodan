#!/usr/bin/env bash
# 本番(stable)ビルド用に Cargo.toml から nightly 専用行を剥がす。
#
# なぜ必要か: dev は nightly（rust-toolchain.toml）。本番は stable でビルドするが、
#   Cargo.toml 冒頭の `cargo-features = ["codegen-backend"]` は stable cargo が
#   パース時点で蹴り（env/CLI でも上書きできない）、`[profile.dev]` 等の
#   `codegen-backend = ...` も「feature が要る」とエラーになる。ファイルから物理的に
#   消すしかないので、本番ビルドの直前にここで剥がす（committed の Cargo.toml は dev=nightly のまま）。
# 前提: Docker builder の ephemeral コピー内で実行する。host で実行すると working tree の
#   Cargo.toml が変わる ── 戻すなら `git restore Cargo.toml`。
# 注: .cargo/config.toml の `-Z threads=8`(nightly rustflag) は Dockerfile の RUSTFLAGS="" が
#   target rustflags ごと上書きするので、ここでは触らない。
set -euo pipefail
cd "$(dirname "$0")/.."   # → lastshot/（Cargo.toml のある場所）

m=Cargo.toml
# fail-loud: 期待行が無ければ Cargo.toml の構造が変わった合図。黙って素通りさせない。
need() { grep -qE "$1" "$m" || { echo "strip-nightly: expected pattern not found in $m: $1" >&2; exit 1; }; }
need '^cargo-features = \["codegen-backend"\]'
need '^codegen-backend = "cranelift"'  # 自前 + 依存とも cranelift に統一済（PR #34）

# nightly 専用の3行（と cargo-features 直前のコメント）を削除。
# sed -i は BSD/GNU で挙動が違うので temp+mv で移植性を確保する。
tmp=$(mktemp)
sed -e '/^# codegen-backend をプロファイルで指定するための nightly opt-in。$/d' \
    -e '/^cargo-features = \["codegen-backend"\]$/d' \
    -e '/^codegen-backend = /d' \
    "$m" > "$tmp"
mv "$tmp" "$m"
echo "strip-nightly: removed nightly-only lines from $m (now stable-parseable)"
