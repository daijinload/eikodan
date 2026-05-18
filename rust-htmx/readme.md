# rust-htmx プロジェクト

Rust + HTMX + MiniJinja + DaisyUI で組んだ TODO CRUD サンプル。
親リポジトリ「eikodan」（理想のWebシステムを模索する曳光弾）の第一弾。
package by featureでcrate分けてビルド高速化を狙う予定だがサンプルなのでベタ置きしている。

## インストール

前提：macOS / Linux / WSL2。外部CDN（DaisyUI / Tailwind / HTMX）で配信するので npm 等フロント側のツールチェーンは不要。

```bash
# 1. Rust toolchain（未インストールの場合のみ。rustup 公式 https://www.rust-lang.org/tools/install ）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# → インストール後は新しいシェルを開き直す（公式手順）
rustc --version   # 動作確認

# 2. リポジトリ取得
git clone git@github.com:daijinload/eikodan.git
cd eikodan/rust-htmx

# 3. 依存取得＋ビルド（初回のみ数分かかる）
cargo build

# 4. （任意）Rust ソース変更時に自動再ビルドさせたい場合のみ（cargoの拡張機能なのでcargo.tomlには書けない）
cargo install cargo-watch

# 5. （macOS のみ）リンカ lld の導入
#    .cargo/config.toml が /opt/homebrew/opt/lld/bin/ld64.lld を参照しているので、未インストールだとビルドが失敗する
#    使いたくない場合は .cargo/config.toml を削除すれば標準リンカ（Apple ld）で動く
brew install llvm lld
```

## 起動

```bash
# 普通に起動する
cargo run
# hotreloadで動かす（事前に cargo install cargo-watch）
cargo watch -x run
```

## 技術スタック

| レイヤ | 採用 | 役割 |
|---|---|---|
| Web Server | Rust ([axum](https://github.com/tokio-rs/axum) 0.8) | HTTP・ルーティング |
| テンプレート | [MiniJinja](https://github.com/mitsuhiko/minijinja) 2 + [minijinja-autoreload](https://crates.io/crates/minijinja-autoreload) | Jinja2互換、サーバ側のテンプレ更新を検知 |
| Client | [HTMX](https://htmx.org/) 2.0.10（CDN） | フラグメント差し替えで SPA 風挙動 |
| UI | [DaisyUI](https://daisyui.com/) 5 + [Tailwind v4](https://tailwindcss.com/) browser（CDN） | クラス名ベースのコンポーネント |
| 開発時ブラウザ自動リロード | [tower-livereload](https://crates.io/crates/tower-livereload) + [notify](https://crates.io/crates/notify) | dev限定。テンプレ保存でビルド無しホットリロード |

## プロジェクト構成

```
rust-htmx/
├── Cargo.toml
├── src/
│   ├── main.rs        bootstrap, AppState, app() factory, テスト
│   ├── controller.rs  Axum ハンドラ層（HTTP 入出力）
│   ├── usecase.rs     業務フロー層（バリデーション、エラー定義）
│   ├── service.rs     永続化層（インメモリ BTreeMap）
│   └── model.rs       Todo 構造体
└── templates/
    ├── base.html              共通レイアウト（CDN <link>/<script>）
    ├── index.html             一覧ページ
    └── partials/
        ├── todo_row.html      1行（表示用フラグメント）
        └── todo_edit_row.html 1行（編集フォームフラグメント）
```

## アーキテクチャ（3層）

依存方向は **controller → usecase → service → model** の一方向：

```
[HTTP Request]
      │
      ▼
controller   …… Form/Path/State 受け取り、テンプレ描画、StatusCode 返却
      │
      ▼
usecase      …… 空タイトルバリデーション、UseCaseError 定義
      │
      ▼
service      …… BTreeMap への CRUD（Arc<RwLock> で共有）
      │
      ▼
model        …… Todo { id, title, done }
```

下位レイヤは上位を知らない。エラーは `UseCaseError`（`EmptyTitle` / `NotFound`）を controller で `StatusCode` にマップ。

## ルーティング

| メソッド | パス | 役割 | レスポンス |
|---|---|---|---|
| GET | `/` | 一覧ページ | フル HTML（`index.html`） |
| POST | `/todos` | 新規作成 | 行フラグメント |
| GET | `/todos/{id}` | 1件表示（編集キャンセル用） | 行フラグメント |
| PUT | `/todos/{id}` | 更新 | 行フラグメント |
| DELETE | `/todos/{id}` | 削除 | 空 200 |
| GET | `/todos/{id}/edit` | 編集フォーム取得 | 編集行フラグメント |
| POST | `/todos/{id}/toggle` | 完了/未完了トグル | 行フラグメント |

HTMX 側は `hx-target` / `hx-swap="outerHTML"` で行単位の差し替え。

## 起動・テスト

```bash
cd rust-htmx
cargo run    # http://127.0.0.1:3000/
cargo test   # ハンドラ結合テスト 6 件
```

ポートが詰まる場合：

```bash
lsof -ti :3000 | xargs kill
```

## ホットリロード（2層構造）

「テンプレを保存するだけで、cargo を止めずにブラウザが自動リロードされる」状態を作るために 2 つの仕組みを組み合わせている。

### 1. サーバ側テンプレ更新 — minijinja-autoreload

- `AutoReloader` が `templates/` を監視
- ファイル変更時、次の HTTP リクエスト時に Environment を再構築（`acquire_env()` 経由）
- **cargo の再起動は不要**

### 2. ブラウザ自動リロード — tower-livereload + notify

- `LiveReloadLayer` が HTML レスポンスに小さな JS を**自動注入**
- JS はサーバへの long-poll で「reload 通知」を待ち受け
- `notify` で `templates/` を監視するタスクが、変更検知時に `Reloader::reload()` を呼ぶ
- → ブラウザが自動でページをリロード

#### dev 限定ガード

`src/main.rs` の `with_live_reload` は以下でガードされている：

```rust
#[cfg(all(debug_assertions, not(test)))]
```

- `debug_assertions` → リリースビルドでは無効化（JS 注入されない）
- `not(test)` → テストでは watcher を起動しない（テスト分離のため）

#### HTMX フラグメントへの注入回避

- ページ本体ロード時に script が1回入る → ブラウザ常駐 → リロード機能が確立
- フラグメントは「ページの一部を差し替えるだけ」なので、自前で script を持つ必要がない（重複・無駄になるだけ）
- だからスキップしている＝機能を止めているのではなく、不要な重複を消しているだけ

tower-livereload はデフォルトで `Content-Type: text/html` のレスポンスすべてに注入するため、HTMX フラグメント応答にも `<script>` が混入してしまう。
これを避けるため `request_predicate` で `HX-Request` ヘッダ付きのリクエストはスキップ：

```rust
fn not_htmx_predicate<T>(req: &axum::http::Request<T>) -> bool {
    !req.headers().contains_key("hx-request")
}
```

検証結果：

| リクエスト | スクリプト注入 |
|---|---|
| `GET /`（フルページ） | ◯ |
| HTMX フラグメント取得（`HX-Request: true` 付き） | × |

### Rust 側コードの変更について

Rust ソース（`src/*.rs`）の変更は**ビルド＋再起動が必要**。これを自動化するには `cargo-watch` を使う：

```bash
cargo install cargo-watch  # 一度だけ
cargo watch -x run         # src/*.rs 保存で自動ビルド＆再起動
```

- src を保存 → cargo-watch が検知 → 旧プロセス kill → `cargo run` 実行
- インクリメンタル再ビルドは約 0.3 秒（`debug = false` 効果込み）
- 再起動後、ブラウザ側は tower-livereload の long-poll 切断を検知して自動リロード

ビルドを伴わない hot-patch（Dioxus の subsecond 等）も2026時点で存在するが、Axum 適用はまだ実験的なので導入していない。

## ビルド最適化（dev profile）

`Cargo.toml` の `[profile.dev]` で **`debug = false`** を有効化済み。

```toml
[profile.dev]
debug = false
strip = "debuginfo"
```

- インクリメンタル再ビルド **約 0.3 秒**
- 出典: [Rust Performance Book](https://nnethercote.github.io/perf-book/build-configuration.html) — debuginfo を切ると 20〜40% 短縮
- **トレードオフ**: lldb 等のステップ実行デバッガが使えなくなる。`println!`/`dbg!`/panic backtrace（関数名のみ）は使える
- panic に行番号が必要なときは `debug = "line-tables-only"` に切り替え（Cargo.toml にコメント済み）

### リンカ切替（lld）

`.cargo/config.toml` で `aarch64-apple-darwin` のリンカを `lld`（`ld64.lld`）に切替済み。

```toml
[target.aarch64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=/opt/homebrew/opt/lld/bin/ld64.lld"]
```

本プロジェクトでの計測結果（macOS 26.4.1 / Apple Silicon / Rust 1.91.1 / `cargo clean` 後）：

| ビルド種別 | 標準（Apple `ld`） | lld |
|---|---|---|
| フルビルド | 14.47s | 14.71s |
| 差分ビルド #1（`touch src/main.rs`） | 0.94s | 0.74s |
| 差分ビルド #2（同上） | 0.73s | 0.63s |

- **差分ビルドで 100〜200ms 短縮、フルビルドは誤差〜わずかに遅い**
- 依存数が少なく、Apple の新リンカ（Xcode 15+ の `ld_prime`）も既に高速なため Linux ほどの劇的効果は出ない
- 依存が増えれば効きやすくなる想定で残置。不要なら `.cargo/config.toml` を削除すれば標準リンカに戻る
- `mold` は macOS 非対応、`sold`（macOS 版 mold）は作者がアーカイブ済みのため選択肢外

## テスト

`src/main.rs` の `#[cfg(test)] mod tests` に集約。`tower::ServiceExt::oneshot` を使い、ルータに直接 Request を流す（公式 `axum/examples/testing` のパターン）。

カバー範囲（6 ケース）：

1. `GET /` が `200 OK` で `<table` を含む HTML を返す
2. `POST /todos` のレスポンスに作成タイトルが含まれる
3. 作成後の `GET /` 一覧に反映されている
4. `POST /todos/{id}/toggle` で `checked` / `line-through` クラスに切り替わる
5. `PUT /todos/{id}` で title が書き換わる
6. `DELETE /todos/{id}` 後の一覧から消える

```bash
cargo test
```
