# lastshot app のコンテナイメージ（compose の app サービス / 結合確認用）。
# マルチステージ: builder で release ビルド + 本番CSS生成 → runtime に最小限だけ載せる。
#
# 方針:
# - nightly + rust-src は rust-toolchain.toml に従い rustup が自動導入する（イメージは stable だが
#   初回 cargo 実行時に切替わる）。`cargo-features = ["codegen-backend"]` のため nightly は必須。
# - dev 機向け最適化（sccache / mold / -Zthreads）はコンテナの単発クリーンビルドでは無価値なので
#   env で無効化し、追加ツール無しの素ビルドにする（実測確認済み: RUSTFLAGS="" が .cargo/config.toml の
#   target rustflags を置換、CARGO_BUILD_RUSTC_WRAPPER="" が sccache を無効化する）。
# - protoc は schema/build.rs の codegen に必須なので builder に入れる。
# - release は css_built=true（base.html が /static/app.css を参照）なので、本番CSSを生成して載せる。

# ---- builder ----
FROM rust:bookworm AS builder
# protoc: schema の codegen。curl は full イメージに同梱（Tailwind 取得に使う）。
RUN apt-get update \
 && apt-get install -y --no-install-recommends protobuf-compiler \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# 追加ツール無しの素ビルドにする（上記方針）。
ENV RUSTFLAGS="" \
    CARGO_BUILD_RUSTC_WRAPPER=""

# 本番CSS（/static/app.css）を生成。setup-css.sh が Linux 用 Tailwind スタンドアロンを取得する。
RUN bash assets/setup-css.sh \
 && ./assets/tailwindcss -i assets/input.css -o crates/app/static/app.css --minify

# app だけを release ビルド。
RUN cargo build --release -p app

# ---- runtime ----
FROM debian:bookworm-slim AS runtime
# TLS は使わない（sqlx は default-features オフ）。ca-certificates だけ最小で入れておく。
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
# バイナリは CARGO_MANIFEST_DIR(=/app/crates/*) をコンパイル時に焼き込んでいるので、
# テンプレ/CSS を「同じ絶対パス」に置く（webcore はディスクから直読みする）。
COPY --from=builder /app/target/release/app                       /app/app
COPY --from=builder /app/crates/app/templates                     /app/crates/app/templates
COPY --from=builder /app/crates/feature-counter/templates         /app/crates/feature-counter/templates
COPY --from=builder /app/crates/app/static                        /app/crates/app/static

# コンテナでは外から到達できるよう 0.0.0.0 で listen（dev 既定の 127.0.0.1 を上書き）。
ENV HOST=0.0.0.0 \
    PORT=3000
EXPOSE 3000
CMD ["/app/app"]
