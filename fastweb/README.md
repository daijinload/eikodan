# fastweb

「ビルドを避けて開発する」ことに全振りした、Rust + HTMX + MiniJinja + daisyUI の曳光弾。
**変更の7〜8割をノービルドで反映、Rustを触っても数秒、最後だけ release ビルド**という三層ループを実現する。

## なぜ速いか
- **画面まわり（テンプレHTML / CSS / HTMX属性）は Rust の再コンパイルを要求しない。** MiniJinja が
  実行時にテンプレを読み、保存し直すだけで反映される。
- **日常CSSは CDN（ブラウザJIT）でビルドゼロ。** watchもCLIも常駐させない＝摩擦ゼロ。テンプレ保存→自動リロードのみ。
  CDNはDOMから何でも生成するため「devで動くが本番パージで崩れる」クラスを見逃すが、その検査は**push前のゲートに集約**する
  （`assets/check-css.sh` = CLIクリーンビルドで追加/削除パージ確定 + semgrep で危険パターン静的検出）。
  本番/最終確認だけ CLI 生成CSS（`/static/app.css`）に切替（`CSS=built` で起動、release は常にこちら）。
  CSSモードは実行時グローバルで切替し**コンパイルプロファイルに紐付けない**ので、debug↔最終確認の往復で再ビルドしない。
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

bash assets/setup-css.sh   # CSSビルド一式を取得（Node不要・最終確認/本番ゲート用）。Tailwind v4 スタンドアロンCLI +
                           # daisyUI v5 単体ファイルを assets/ に落とす（いずれも .gitignore 済み）。
                           # ※日常開発はCDNなので未取得でも動く。push前ゲート（check-css.sh）に必要。
brew install semgrep       # push前ゲートのパージ崩れ検査に使用（or: pipx install semgrep）。
```
- **incremental は既定ON のまま使う**（`CARGO_INCREMENTAL=0` を設定しない）。日常ループは incremental が、
  フル再ビルドの依存は sccache が、それぞれ別々に効く。
- sccache のキャッシュは依存をビルドし直すとき（`cargo clean` 後・新規checkout 等）に効く。初回はキャッシュを
  作る側なので通常通りの時間がかかる。`sccache --show-stats` でヒット状況を確認できる。
- **git worktree 等で複数環境を並列起動するときは sccache を共有する**（`SCCACHE_DIR` を上書きせず既定
  `~/Library/Caches/Mozilla.sccache` のまま）。`target/` は環境ごとに別・sccache は共有 ── 重い依存を
  1回ビルドすれば全環境で使い回せる。多数回すなら `export SCCACHE_CACHE_SIZE=30G`（既定10GiB）。

## 動かす
日常開発は **CDNで1プロセスだけ**（CSSウォッチャは不要）:
```sh
cargo run -p app        # or: bacon run / bacon serve（ソケット引き継ぎで接続が切れない。要 systemfd）
# → http://127.0.0.1:3000  [css: cdn] 。「押してね」ボタンが HTMX 部分更新。
#   PORT=3001 等で上書き可（複数worktreeを並列起動して :3000 が衝突するとき）。
```
テンプレ(`crates/*/templates/*.html`)を保存し直すと、Rust再ビルド無しでブラウザが自動リロードする。
CSSはCDN（ブラウザJIT）が即生成するのでビルドもwatchも不要。

**最終確認だけ**、本番と同じCLI生成CSSを目視する（再ビルドなし＝同じdebugバイナリ）:
```sh
bash assets/check-css.sh          # クリーンビルド（パージ確定）+ semgrep。通ったら↓
CSS=built cargo run -p app        # /static/app.css を配信して目視（[css: built] と出る）
```

確認はフルビルドを待たず、触っているクレートだけに絞る:
```sh
bacon                            # 保存で cargo check が即返る（型エラーを最速で拾う）
cargo check -p <feature>         # 触っているクレートの型だけ確認
cargo nextest run -p <feature>   # 触っているクレートの単体テストだけ
```

### push前のCSSゲート（パージ崩れの確定検査）
日常はCDNなので「本番パージで消えるクラス」は開発中に見えない。**push（手元を離れる節目）の前に1回**だけ
ゲートを回して確定検査する。pre-commit は使わない（軽量開発と最終確認を明確に分ける）:
```sh
bash assets/check-css.sh
#  1) クリーンビルド（--watch なし）= app.css をフル生成。追加も削除も毎回パージ確定。
#  2) semgrep（assets/semgrep/）= 黙って消える危険パターンを静的検出:
#       ・class 属性内の動的合成クラス（text-{{ color }}-500 等）
#       ・.rs 内のクラス文字列（source(none) の走査外で無スタイルになる）
```
release ビルドも必ずCLIクリーン生成を通るので、最終成果物は常に正しい。

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
| CSSクラスを変更（日常・CDN） | **即時**（ブラウザJIT。ビルドもwatchも無し） |
| push前ゲートのCLIクリーン生成 | 約 50〜60ms（`check-css.sh` の app.css フル生成。Rustビルドは走らない） |
| connect-rpc(protobuf codegen)込みの再リンク | 約 6s |

## 構成
```
crates/
  app/            bin。ルーター組み立て・起動・ライブリロード・/static配信だけの薄い層
                  static/app.css ← CLI生成のCSS（.gitignore済み。最終確認/本番でのみ使用）
  webcore/        共有コア（AppState + MiniJinjaローダ）。安定したものだけ
  feature-hello/  1機能 = ハンドラ + templates/ + テスト（葉クレート）
  rpc/            connect-rpc（proto + build.rs codegen + ハンドラ）
assets/           CSSゲート一式: input.css・semgrep/・check-css.sh(追跡) +
                  tailwindcss/daisyui.mjs(取得・.gitignore)
tests-http/       起動済みサーバーをHTTPで叩くブラックボックステスト（ワークスペース外）
```
機能を増やすときは `crates/feature-xxx` を切り、`app` で `.merge(feature_xxx::routes())` と
テンプレートディレクトリを1行足すだけ。詳しい運用ルールは `CLAUDE.md`。

## 設定の置き場
- リンカ・並列フロントエンド・sccache(`rustc-wrapper`): `.cargo/config.toml`
- nightly 固定: `rust-toolchain.toml`
- プロファイル（opt-level 非対称 / Cranelift）: `Cargo.toml` の `[profile.dev]`
- CSS生成（Tailwind入力 / パージ対象 / daisyUIテーマ）: `assets/input.css`
- CSSモード切替（CDN ⇄ CLI生成）: 実行時グローバル `css_built`（`crates/app/src/main.rs` → base.html）
- push前ゲート（クリーンビルド + semgrep）: `assets/check-css.sh` / `assets/semgrep/`

> **Cranelift を切りたい場合**: `Cargo.toml` の `codegen-backend` 行（2箇所）と
> 先頭の `cargo-features = ["codegen-backend"]` を消せば LLVM のみに戻る。
> 巨大クレートではコード生成が速くなるが、現状の小規模ではリンク律速で差は小さい。
