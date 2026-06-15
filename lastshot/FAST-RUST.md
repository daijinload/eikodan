# FAST-RUST — Rust 開発を加速するためにやったこと（総まとめ）

eikodan の各サブプロジェクト（fastweb / connectweb / pg-bench / subsecond-demo）の試行を踏まえて
lastshot で採った高速化施策の一覧。実測根拠と「効かなかった理由」は各リンク先に。

## 1. 設計で稼ぐ（そもそもビルドしない）

1. **作業の7〜8割はビルドしない設計** — テンプレ / CSS / HTMX 属性の変更は保存で即反映（Rust ビルドゼロ）。
   詳細は [`fastweb/HOTRELOAD.md`](../fastweb/HOTRELOAD.md)。
2. **触った feature だけ再ビルド（package by feature）** — workspace 共通 `Cargo.toml` は触らない
   （`[workspace.dependencies]` やプロファイルを弄ると全クレート再ビルドが走る）。方針は [`CLAUDE.md`](./CLAUDE.md)。
3. **`sqlx::query!` 不使用 + codegen を `schema` クレートに隔離** — コンパイル時に DB 接続を要求せず、
   protoc は proto を触った時だけ走る。dev のビルド時依存を最小化。詳細は [`CLAUDE.md`](./CLAUDE.md) の「DB 作法」。
4. **Rust 変更時は約1秒が底（通常開発の増分ビルドの話）** — 「ファイル1個 touch して `cargo build -p app`」の
   増分ビルドが ~0.6〜0.74s + cold start 0.285s + ブラウザ再描画 ＝ 体感 ~1.2〜1.3s。フルビルドではない。
   参考: フルビルド（`cargo clean` 後）は **~13s**（sccache が重い依存を返すため）、no-op は **~0.1s**。
   実測内訳は [`COLD-START.md`](./COLD-START.md)。
5. **cold start は macOS のバイナリ検証** — `codesign -f -s -` で約 100ms 短縮（**唯一効いた起動側の施策**）。
   詳細は [`COLD-START.md` §①](./COLD-START.md)。

## 2. ビルドツール側で稼ぐ（ビルドが走る時に速く）

1. **nightly + `-Z threads=8`（並列フロントエンド）** — **最終確認・本番ビルドは stable で**やる。
   `./run release` と Dockerfile が `RUSTUP_TOOLCHAIN=stable` + `assets/strip-nightly.sh` で nightly 専用行
   （`cargo-features` / `codegen-backend`）を剥がして stable に倒す。**dev=nightly / 本番=stable** が掟。
   なお `-Z threads` の効きは現状ほぼ無いが、クレートが育った時の保険として残す。詳細は [`COLD-START.md` §④](./COLD-START.md)。
2. **lld（macOS）／ mold（Linux）でリンク高速化** — `.cargo/config.toml` の target rustflags で配線。
   ただし「もっと速いリンカ」が更に効くわけではない（lld ≈ apple-ld）。詳細は [`COLD-START.md` §③](./COLD-START.md)。
3. **sccache で重い依存をキャッシュ** — `opt-level=3` でビルドされる重物（axum / tokio / buffa…）をキャッシュから返す。
   `cargo clean` / 新規 checkout / deps が変わるブランチ切替を短縮。実測は [`fastweb/BENCHMARK.md`](../fastweb/BENCHMARK.md) ④。
4. **incremental は既定 ON のまま** — 差分ビルドの本体は `incremental`（`CARGO_INCREMENTAL=0` にしない）。
   sccache とは別レイヤなので併用する。

## 3. 採らなかったもの（効かないと実測で確認）

- **ホットパッチ（subsecond / dioxus）** — 関数本体しか差し替えられず、構造変更（フィールド追加・シグネチャ変更・
  スキーマ変更）で結局フルビルドに戻る。AI 高速開発の「黙ってフォールバック」は事故のもと。
  詳細は [`COLD-START.md`](./COLD-START.md) ホットパッチ節。
- **systemfd で速さを稼ぐ** — 速さは変わらない、接続断を消すだけ。しかも現状の `bacon.toml` 配置では
  接続断も消えていない。詳細は [`COLD-START.md` §②](./COLD-START.md)。
- **`-Z threads=N` の N を増やす** — 15コア機でも N=12,16 は逆に悪化（cargo のクレート並列とオーバーサブスクライブ）。
  詳細は [`COLD-START.md` §④](./COLD-START.md)。
- **速いリンカへの置き換え** — lld が既に最速で詰めしろ無し。`wild` は Linux 専用。詳細は [`COLD-START.md` §③](./COLD-START.md)。
