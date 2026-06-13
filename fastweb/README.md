# fastweb

「ビルドを避けて開発する」ことに全振りした、Rust + HTMX + MiniJinja + daisyUI の曳光弾。
nightly のビルド高速化フラグと package-by-feature で、**変更の7〜8割をノービルドで反映、
Rustを触っても数秒、最後だけ release ビルド**という三層ループを実現する。

## スタックと「速い」理由
- **MiniJinja**（実行時テンプレート）+ **HTMX** + **daisyUI/Tailwind v4(CDN)**
  → 画面まわりの変更は Rust の再コンパイルを一切要求しない。
- **package by feature**: 機能ごとに葉クレートを切り、`cargo check -p <feature>` が一瞬。
- **nightly チューニング**: lld リンカ / 並列フロントエンド(`-Zthreads`) / Cranelift バックエンド /
  依存だけ `opt-level=3`・自分のコードは `opt-level=0`。
- **sccache**（コンパイルキャッシュ）: 重い依存をキャッシュから返し、フル再ビルドを短縮。
  incremental は既定ONのまま併用するので、日常の差分ビルド/起動の速さはそのまま（→ BENCHMARK.md ④）。
- **connect-rpc** を同じ axum・同じポートに同居（型付きAPIが必要なとき用）。

## セットアップ（初回だけ）
```sh
brew install sccache    # 必須。.cargo/config.toml が rustc-wrapper に指定しているため、
                        # 未導入だと cargo が "could not execute process sccache" で失敗する。
```
- **incremental は既定のON のまま使う**（`CARGO_INCREMENTAL=0` を設定しないこと）。日常ループは
  incremental が、フル再ビルドの依存は sccache が、それぞれ別々に効く。
- sccache のキャッシュは依存をビルドし直すとき（`cargo clean` 後・新規checkout 等）に効く。初回は
  キャッシュを作る側なので通常通りの時間がかかる。`sccache --show-stats` でヒット状況を確認できる。
- **git worktree 等で複数環境を並列起動するときは sccache を共有する**（`SCCACHE_DIR` を上書きせず既定
  `~/Library/Caches/Mozilla.sccache` のまま）。`target/` は環境ごとに別・sccache は共有 ── 重い依存を
  1回ビルドすれば全環境で使い回せる。内容ハッシュ鍵＆並行アクセス前提なので衝突しない。多数回すなら
  `export SCCACHE_CACHE_SIZE=30G`（既定10GiB）で上限を上げる。

## 実測（Apple Silicon, nightly）
| 操作 | 時間 |
|---|---|
| 初回フルビルド | 約 12s |
| feature のコードを1箇所変更 → 再ビルド | **0.4s**（依存元 + lld リンク込み） |
| テンプレート/CSS/HTMX属性だけ変更 | **0.02s**（= 実質ビルドゼロ） |
| connect-rpc(protobuf codegen)込みの再リンク | 約 6s |

## 動かす
```sh
cargo run -p app
# → http://127.0.0.1:3000 を開く。「押してみる」ボタンが HTMX 部分更新。
# テンプレ(crates/*/templates/*.html)を保存し直すと、再ビルド無しでブラウザが自動リロード。
```

### connect-rpc デモ（HTMLと同一ポートで同居）
```sh
curl -X POST http://127.0.0.1:3000/greet.v1.GreetService/Greet \
  -H "Content-Type: application/json" -d '{"name":"world"}'
# => {"greeting":"Hello, world!"}
```

## 開発ループ（推奨）
```sh
bacon            # 保存で cargo check が即返る（型エラーを最速で拾う）
bacon run        # サーバーを起動し、Rust変更で自動再起動
bacon serve      # ↑をソケット引き継ぎ再起動に（接続が切れない。要 `cargo install systemfd`）
cargo nextest run -p <feature>   # 触っているクレートの単体テストだけ
```

## HTTPテスト（別プロジェクト・起動は外部）
`tests-http/` は親ワークスペースから切り離した独立クレート（アプリをリンクしない）。
先にサーバーを起動してから叩く:
```sh
cargo run -p app &              # 起動
cd tests-http && cargo nextest run
```

## 構成
```
crates/
  app/            bin。ルーター組み立て・起動・ライブリロードだけの薄い層
  webcore/        共有コア（AppState + MiniJinjaローダ）。安定したものだけ
  feature-hello/  1機能 = ハンドラ + templates/ + テスト（葉クレート）
  rpc/            connect-rpc（proto + build.rs codegen + ハンドラ）
tests-http/       起動済みサーバーをHTTPで叩くブラックボックステスト（ワークスペース外）
```
機能を増やすときは `crates/feature-xxx` を切り、`app` で `.merge(feature_xxx::routes())` と
テンプレートディレクトリを1行足すだけ。詳しい運用ルールは `CLAUDE.md`。

## 高速化フラグの位置
- リンカ・並列フロントエンド・sccache(`rustc-wrapper`): `.cargo/config.toml`
- nightly 固定: `rust-toolchain.toml`
- プロファイル（opt-level 非対称 / Cranelift）: `Cargo.toml` の `[profile.dev]`

> **Cranelift を切りたい場合**: `Cargo.toml` の `codegen-backend` 行（2箇所）と
> 先頭の `cargo-features = ["codegen-backend"]` を消せば LLVM のみに戻る。
> 巨大クレートではコード生成が速くなるが、現状の小規模ではリンク律速で差は小さい。
