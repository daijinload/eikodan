# fastweb

「ビルドを避けて開発する」ことに全振りした、Rust + HTMX + MiniJinja + daisyUI の曳光弾。
**変更の7〜8割をノービルドで反映、Rustを触っても数秒、最後だけ release ビルド**という三層ループを実現する。

## なぜ速いか
- **画面まわり（テンプレHTML / CSS / HTMX属性）は Rust の再コンパイルを要求しない。** MiniJinja が
  実行時にテンプレを読み、保存し直すだけで反映される。
- **CSSは開発中も本番と同じ Tailwind CLI で生成する（開発＝本番）。** CDNのブラウザJITは使わない ──
  実行時生成だと「未使用クラスを消すパージ」を通らず、本番ビルドでだけ崩れるクラスを開発中に見逃すため。
  テンプレ保存 → スタンドアロンの `tailwindcss --watch` が数十msでCSSを再生成 → 自動リロード（**Rustビルドは走らない**）。
- **package by feature**: 機能ごとに葉クレートを切り、`cargo check -p <feature>` が一瞬。
- **nightly チューニング**: lld リンカ / 並列フロントエンド(`-Zthreads`) / Cranelift バックエンド /
  依存だけ `opt-level=3`・自分のコードは `opt-level=0`。
- **sccache**（コンパイルキャッシュ）: 重い依存をキャッシュから返す。incremental は既定ONのまま併用するので、
  日常の差分ビルド/起動の速さはそのまま（→ BENCHMARK.md ④）。
- **connect-rpc** を同じ axum・同じポートに同居（型付きAPIが必要なとき用）。

> 監視 → 生成 → リロードの**具体的な流れとキャッシュの扱い**は [HOTRELOAD.md](HOTRELOAD.md)。

## セットアップ（初回だけ）
```sh
brew install sccache    # 必須。.cargo/config.toml が rustc-wrapper に指定しているため、
                        # 未導入だと cargo が "could not execute process sccache" で失敗する。

bash assets/setup-css.sh   # CSSビルド一式を取得（Node不要）。Tailwind v4 スタンドアロンCLI +
                           # daisyUI v5 単体ファイルを assets/ に落とす（いずれも .gitignore 済み）。
./assets/tailwindcss -i assets/input.css -o crates/app/static/app.css   # 初回に1度だけ生成
```
- **incremental は既定ON のまま使う**（`CARGO_INCREMENTAL=0` を設定しない）。日常ループは incremental が、
  フル再ビルドの依存は sccache が、それぞれ別々に効く。
- sccache のキャッシュは依存をビルドし直すとき（`cargo clean` 後・新規checkout 等）に効く。初回はキャッシュを
  作る側なので通常通りの時間がかかる。`sccache --show-stats` でヒット状況を確認できる。
- **git worktree 等で複数環境を並列起動するときは sccache を共有する**（`SCCACHE_DIR` を上書きせず既定
  `~/Library/Caches/Mozilla.sccache` のまま）。`target/` は環境ごとに別・sccache は共有 ── 重い依存を
  1回ビルドすれば全環境で使い回せる。多数回すなら `export SCCACHE_CACHE_SIZE=30G`（既定10GiB）。

## 動かす
開発時は **2プロセスを常駐**させる（互いに独立。CSSウォッチャ ＋ アプリ）:
```sh
# ① CSSウォッチャ: テンプレ保存で app.css を再生成（Rustループとは独立。保存→数十ms）
./assets/tailwindcss -i assets/input.css -o crates/app/static/app.css --watch

# ② アプリ: 起動（Rustを触る日は bacon に任せると再ビルド〜再起動まで面倒を見る）
cargo run -p app        # or: bacon run / bacon serve（ソケット引き継ぎで接続が切れない。要 systemfd）
# → http://127.0.0.1:3000 。「押してね」ボタンが HTMX 部分更新。
#   PORT=3001 等で上書き可（複数worktreeを並列起動して :3000 が衝突するとき）。
```
テンプレ(`crates/*/templates/*.html`)を保存し直すと、Rust再ビルド無しでCSSが再生成されブラウザが自動リロードする
（CSSは app 自身が `/static/app.css` で配信）。

確認はフルビルドを待たず、触っているクレートだけに絞る:
```sh
bacon                            # 保存で cargo check が即返る（型エラーを最速で拾う）
cargo check -p <feature>         # 触っているクレートの型だけ確認
cargo nextest run -p <feature>   # 触っているクレートの単体テストだけ
```

### 節目のクリーンビルド（パージ確定）
`--watch` の差分ビルドは追記的で、消したクラスがそのセッション中は `app.css` に残る（→ [HOTRELOAD.md](HOTRELOAD.md)）。
**コミット/納品の前**と**パージに関わるリファクタ後**（クラスを動的合成や `.rs` に追い出した等）は、
`--watch` なしで1回フル生成してパージを確定させる（=このネイティブコマンドを節目で叩く）:
```sh
./assets/tailwindcss -i assets/input.css -o crates/app/static/app.css
```
release ビルドはこのクリーン生成を必ず通るので、最終成果物は常に正しい。

### connect-rpc デモ（HTMLと同一ポートで同居）
```sh
curl -X POST http://127.0.0.1:3000/greet.v1.GreetService/Greet \
  -H "Content-Type: application/json" -d '{"name":"world"}'
# => {"greeting":"Hello, world!"}
```

### HTTPテスト（別プロジェクト・起動は外部）
`tests-http/` は親ワークスペースから切り離した独立クレート（アプリをリンクしない）。
先にサーバーを起動してから叩く:
```sh
cargo run -p app &
cd tests-http && cargo nextest run
```

## 実測（Apple Silicon, nightly）
| 操作 | 時間 |
|---|---|
| 初回フルビルド | 約 12s |
| feature のコードを1箇所変更 → 再ビルド | **0.4s**（依存元 + lld リンク込み） |
| テンプレート/HTMX属性だけ変更 | **0.02s**（= Rustビルドゼロ。自動リロードのみ） |
| CSSクラスを変更 | **約 0.06s**（tailwind --watch のCSS再生成。Rustビルドは走らない） |
| connect-rpc(protobuf codegen)込みの再リンク | 約 6s |

## 構成
```
crates/
  app/            bin。ルーター組み立て・起動・ライブリロード・/static配信だけの薄い層
                  static/app.css ← CLI生成のCSS（.gitignore済み。--watch が生成）
  webcore/        共有コア（AppState + MiniJinjaローダ）。安定したものだけ
  feature-hello/  1機能 = ハンドラ + templates/ + テスト（葉クレート）
  rpc/            connect-rpc（proto + build.rs codegen + ハンドラ）
assets/           CSSビルド一式: input.css(追跡) + tailwindcss/daisyui.mjs(取得・.gitignore)
tests-http/       起動済みサーバーをHTTPで叩くブラックボックステスト（ワークスペース外）
```
機能を増やすときは `crates/feature-xxx` を切り、`app` で `.merge(feature_xxx::routes())` と
テンプレートディレクトリを1行足すだけ。詳しい運用ルールは `CLAUDE.md`。

## 設定の置き場
- リンカ・並列フロントエンド・sccache(`rustc-wrapper`): `.cargo/config.toml`
- nightly 固定: `rust-toolchain.toml`
- プロファイル（opt-level 非対称 / Cranelift）: `Cargo.toml` の `[profile.dev]`
- CSS生成（Tailwind入力 / パージ対象 / daisyUIテーマ）: `assets/input.css`

> **Cranelift を切りたい場合**: `Cargo.toml` の `codegen-backend` 行（2箇所）と
> 先頭の `cargo-features = ["codegen-backend"]` を消せば LLVM のみに戻る。
> 巨大クレートではコード生成が速くなるが、現状の小規模ではリンク律速で差は小さい。
