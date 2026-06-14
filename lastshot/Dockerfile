# lastshot app のコンテナイメージ（compose の app サービス / 結合確認用）。
# マルチステージ: builder で release ビルド + 本番CSS生成 → runtime に最小限だけ載せる。
#
# 方針（dev=nightly / 本番=stable）:
# - 本番は stable でビルドする。RUSTUP_TOOLCHAIN=stable で rust-toolchain.toml(nightly) を上書きする
#   （nightly + rust-src の取得を避け、安定版で焼く）。dev は rust-toolchain.toml の nightly のまま。
# - ただし Cargo.toml の `cargo-features = ["codegen-backend"]` 等 nightly 専用行は stable cargo が
#   パース時点で蹴る（env/CLI でも上書き不可）。assets/strip-nightly.sh で事前に物理削除する。
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

# 本番は stable でビルド（rust-toolchain.toml の nightly を env で上書き）＋ 追加ツール無しの素ビルド。
# RUSTFLAGS="" が .cargo/config.toml の target rustflags（mold / -Zthreads）を、
# CARGO_BUILD_RUSTC_WRAPPER="" が sccache を、それぞれ無効化する。
ENV RUSTUP_TOOLCHAIN=stable \
    RUSTFLAGS="" \
    CARGO_BUILD_RUSTC_WRAPPER=""

# 本番CSS（/static/app.css）を生成。setup-css.sh が Linux 用 Tailwind スタンドアロンを取得する。
RUN bash assets/setup-css.sh \
 && ./assets/tailwindcss -i assets/input.css -o crates/app/static/app.css --minify

# stable では nightly 専用行が manifest を蹴るので、ビルド直前に剥がす（ephemeral コピーなので破壊的でよい）。
RUN bash assets/strip-nightly.sh

# app だけを release ビルド（stable）。
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
