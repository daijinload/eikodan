# lastshot — 最終進化形態（全部入りの本番実装）

`eikodan`（曳光弾＝試し撃ち）の各サブプロジェクトで個別に検証した要素を、**1つに統合した本番実装**。
名前の由来は [`NAMING.md`](./NAMING.md)（曳光弾を重ねて、最後にこれで撃ち抜く一発＝last shot）。

サンプル画面は **DB保存カウンター**（数字＋「+1」ボタンだけの超シンプル構成）。だが裏側は
全要素入り ── スキーマファースト・Postgres 永続化・ホットリロード・CSSゲートが効いている。

## 何を統合したか（各サブプロジェクトの結論を集約）

| 要素 | 出自 | lastshot での形 |
|---|---|---|
| ノービルドUI（axum + HTMX + MiniJinja + daisyUI）+ ホットリロード + package-by-feature + ビルド最適化（nightly/lld/-Zthreads/Cranelift/opt非対称/sccache） | [fastweb](../fastweb/) | 土台 |
| スキーマファースト（`.proto` 単一真実 → 1つの生成型で HTML描画 / `<!-- view-data -->` 埋め込み / Connect API を同源駆動） | [connectweb](../connectweb/) | 土台 |
| Postgres 永続化（unix ソケット最速 / `query!` マクロ不使用でビルド速度維持） | [pg-bench](../pg-bench/) | `crates/db` + service 層 |
| サンプル題材（カウンター） | [subsecond-demo](../subsecond-demo/) | `crates/feature-counter`（HTMX + DB化） |
| CSS最終確認ゲート（Tailwind CLI フルパージ + semgrep） | fastweb `assets/` | `assets/` |
| タスクランナー（bash 関数ディスパッチャ） | [task-runners](../task-runners/) | `./run` |
| 開発環境（macOS bash 5.x） | [bash-setup.md](../bash-setup.md) | 下記セットアップから参照 |

> 除外: **subsecond ホットパッチ**（axum 素組には非対応＝Dioxus 移行が要る。記録は `NAMING.md` 系の議論のみ）。
> **rust-htmx** は fastweb が上位版なので取り込まない。

## 設計の核（スキーマ＝単一の真実）

```
        .proto（唯一の真実 / ビュー専用 CounterView）
           │ buffa+connectrpc で生成（serde 標準装備 = proto3 JSON）
           ▼
   サービス層 get_count(pool) / increment(pool) -> CounterView   ← ロジックの本体
    │                                  │
  HTML経路(feature-counter)        API経路(rpc)
   render_view で描画 +            同じ get_count/increment を呼ぶだけ
   同じ instance を <!-- view-data --> 埋め込み   POST /counter.v1.CounterService/{GetCount,Increment}
```

`CounterView`（proto 生成型）を画面・埋め込みJSON・API が共有する。**データ取得は1回・出口は複数**
なので、画面の値と埋め込みJSONとAPIの値はズレようがない。値は Postgres の `counter` テーブル（1行）に保存し、
**サーバを再起動しても残る**。

## セットアップ（初回だけ）

```sh
# 1) Rust ツールチェイン: nightly + rust-src（rust-toolchain.toml が指定。rustup が自動で入れる）
# 2) ビルド補助
brew install sccache      # 必須。.cargo/config.toml が rustc-wrapper に指定している
brew install protobuf     # schema/build.rs の codegen が protoc を使う
# 3) Postgres（ネイティブ＝開発の既定。pg-bench の結論で unix ソケット最速）
brew install postgresql@17
# 4) bash 5.x 推奨（macOS 既定の 3.2 でも ./run は動くが 5.x が望ましい）→ ../bash-setup.md
# 5) CSS ゲート用バイナリ（最終確認のときだけ。日常はCDNなので不要）
./run css-setup           # Tailwind CLI + daisyUI を assets/ に取得（.gitignore 済み）
```

## 動かす

```sh
./run db-start            # ネイティブ Postgres を起動（常駐ログインにしない=run）
./run db-setup            # DB 作成 + migrations/schema.sql・seed.sql 適用（冪等）
./run dev                 # = cargo run -p app
# → http://127.0.0.1:3000 にカウンター。「+1」で増え、再起動しても値が残る（DB永続化）。
#   view-source 末尾の <!-- view-data --> に「その画面が使った正確な値」が JSON で入っている。
```

### Connect API（HTMLと同一ポートで同居 / 同じ service 層を共有）

```sh
curl -X POST http://127.0.0.1:3000/counter.v1.CounterService/GetCount \
  -H "Content-Type: application/json" -d '{}'
# => {"value":3}     ← 画面の数字・埋め込み view-data と一致する
curl -X POST http://127.0.0.1:3000/counter.v1.CounterService/Increment \
  -H "Content-Type: application/json" -d '{}'
# => {"value":4}
```

> 注: proto3 JSON は 0 値フィールドを省略する。`value` が 0 のときレスポンスは `{}` になる。

## 開発ループ（3層）

1. **テンプレ・CSS・HTMX属性** → 保存で即反映（Rustビルドゼロ。作業の7〜8割）。`./run dev` 起動中にテンプレを直すだけ。
2. **ハンドラ・サービス層（get_count 等）** → 該当クレートだけ数秒で再ビルド。`./run watch`（bacon）が裏で回る前提。
3. **スキーマ（.proto）** → `schema` クレートで codegen が走る（proto を触ったときだけ）。

```sh
./run watch               # bacon: 保存で check → 再ビルド → 再起動（ソケット維持）
./run check               # cargo check --workspace（型エラーを最速で拾う）
```

## CSS（日常はCDN・最終確認だけビルド）

CSSモードは実行時グローバル `css_built` で切替（`base.html`）。`cfg!(debug_assertions)` には紐付けない
＝ debug↔最終確認を往復しても Rust 再ビルドが走らない（プロジェクトの肝）。

```sh
# 日常: 何もしない（debug 既定 = CDN でブラウザJIT、ビルド/watch 不要）
./run css-check           # push前ゲート: Tailwind CLI フルパージ生成 + semgrep（動的合成クラス検出）
CSS=built cargo run -p app  # 最終目視: CLI生成の /static/app.css を配信（release は常にこちら）
```

## テスト

```sh
./run test-http           # 起動済みサーバへ HTTP ブラックボックステスト（tests-http/ = ワークスペース外）
./run browser-setup       # ブラウザテストの依存取得（初回のみ: npm + Chromium）
./run browser             # ブラウザ駆動テスト（Playwright / browser/ = ワークスペース外）
./run test                # まとめて（http +（あれば）browser）
```

> `browser/` は HTMX の実 swap・DOM 表示・`<!-- view-data -->`・Connect API JSON の**一致**を
> 実ブラウザで突き合わせる（詳細は [`browser/README.md`](./browser/README.md)）。tests-http/ と同じく
> 「サーバは別で起動しておく」前提（アプリ本体をビルド/同梱しない＝疎結合）。

## CI / コンテナ

```sh
./run up                  # app + postgres を docker compose 同一網で起動（DATABASE_URL=TCP）
./run down                # = docker compose down -t 0（即 SIGKILL / 使い捨て前提）
```

- `compose.yml` + `Dockerfile`: app（release / 本番CSS入り）と postgres:17 を同一網で起動。接続は
  本番/CI と同じ `DATABASE_URL` の TCP。データボリュームは持たない（使い捨て＝down で消え、up で
  migrations が冪等に再適用）。コンテナは dev 機向け最適化（sccache/mold/-Zthreads）を env で無効化した
  素ビルド（追加ツール不要・nightly + protoc のみ）。
- CI: `.github/workflows/lastshot-ci.yml`（**リポジトリ直下**。GitHub は monorepo でも root の
  `.github/workflows/` しか実行しないため。`lastshot/**` だけを対象に path フィルタ）。中身は
  build(release) → CSSゲート → 起動 → `test-http` → ブラウザE2E をネイティブで一気通し。
- 「compose分割 vs 全部入りsingle」の CI環境比較は計測フェーズに保留中（[`../container-ops.md`](../container-ops.md)）。
  まずは「緑になる CI を1本」通す段階。

## 構成

```
lastshot/
  run                  bash 関数ディスパッチャ（タスクランナー。./run help で一覧）
  rust-toolchain.toml  nightly + rust-src
  .cargo/config.toml   lld + -Zthreads + sccache（ビルド高速化フラグ）
  Cargo.toml           workspace（opt非対称 / Cranelift）
  bacon.toml           check / run / serve
  assets/              CSSゲート（input.css / setup-css.sh / check-css.sh / semgrep/）
  migrations/          schema.sql（counter テーブル）+ seed.sql（id=1, value=0）
  crates/
    app/               bin。ルーター組み立て・起動・ライブリロード・/static/app.css 配信のみ
    webcore/           共有コア（AppState = MiniJinjaローダ + DBプール）。render / render_view / render_view_fragment
    schema/            .proto + build.rs codegen = 単一の真実。HTML/JSON/API が共有する生成型
    db/                Postgres 接続プール（薄い）。query! マクロは使わない
    feature-counter/   1機能 = get_count/increment（service層）+ HTMLルート + templates/（依存は webcore/schema/db だけ）
    rpc/               Connect API の薄い殻。service層を呼んで同じ CounterView を返す
  tests-http/          起動済みサーバを HTTP で叩くブラックボックステスト（ワークスペース外）
  browser/             ブラウザ駆動E2E（Playwright / HTMX swap・view-data・API の一致検証 / ワークスペース外）
  Dockerfile           app の release イメージ（本番CSS入り / 素ビルド）
  compose.yml          app + postgres 同一網（TCP接続 / 使い捨て）
  （CI は monorepo 直下の ../.github/workflows/lastshot-ci.yml。GitHub は root の .github しか実行しないため）
```

高速化フラグの位置は fastweb と同じ（リンカ/threads/sccache=`.cargo/config.toml`、nightly=`rust-toolchain.toml`、
opt非対称/Cranelift=`Cargo.toml`）。実測根拠は [`fastweb/BENCHMARK.md`](../fastweb/BENCHMARK.md)。
