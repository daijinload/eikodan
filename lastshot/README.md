# lastshot — 最終進化形態（全部入りの本番実装）

`eikodan`（曳光弾＝試し撃ち）の各サブプロジェクトで個別に検証した要素を、**1つに統合した本番実装**。
名前の由来は [`docs/naming.md`](./docs/naming.md)（曳光弾を重ねて、最後にこれで撃ち抜く一発＝last shot）。

サンプル画面は **DB保存カウンター**（数字＋「+1」ボタンだけの超シンプル構成）。だが裏側は
全要素入り ── スキーマファースト・Postgres 永続化・ホットリロード・CSSゲートが効いている。

> **この README は「使い方」。** なぜそうしたか・実測値・採らなかった理由は
> **[`docs/`](./docs/) に集約**してある（入口は [`docs/README.md`](./docs/README.md)）。
> AIエージェント向けの運用ルールは [`CLAUDE.md`](./CLAUDE.md)。

## 何を統合したか（各サブプロジェクトの結論を集約）

| 要素                                                                                                                                                      | 出自                                       | lastshot での形                         |
| --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ | --------------------------------------- |
| ノービルドUI（axum + HTMX + MiniJinja + daisyUI）+ ホットリロード + package-by-feature + ビルド最適化（nightly/lld/-Zthreads/Cranelift/dev=opt0/sccache） | fastweb（実験場）                          | 土台                                    |
| スキーマファースト（`.proto` 単一真実 → 1つの生成型で HTML描画 / `<!-- view-data -->` 埋め込み / Connect API を同源駆動）                                 | connectweb（実験場）                       | 土台                                    |
| Postgres 永続化（unix ソケット最速 / `query!` マクロ不使用でビルド速度維持）                                                                              | pg-bench（実験場）                         | `crates/db` + service 層                |
| サンプル題材（カウンター）                                                                                                                                | subsecond-demo（実験場）                   | `crates/feature-counter`（HTMX + DB化） |
| CSS最終確認ゲート（Tailwind CLI フルパージ + semgrep）                                                                                                    | fastweb `assets/`                          | `assets/`                               |
| タスクランナー（bash 関数ディスパッチャ）                                                                                                                 | task-runners（実験場）                     | `./run`                                 |
| 開発環境（macOS bash 5.x）                                                                                                                                | [docs/bash-setup.md](./docs/bash-setup.md) | 下記セットアップから参照                |

> 除外: **subsecond ホットパッチ**（axum 素組には非対応＝Dioxus 移行が要る。判断の記録は
> [`docs/decisions/0001-subsecond-hotpatch.md`](./docs/decisions/0001-subsecond-hotpatch.md)）。
> **rust-htmx** は fastweb が上位版なので取り込まない。
> 他に採らなかったものの一覧は [`docs/decisions/`](./docs/decisions/)。

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
# 3) Postgres（ネイティブ＝開発の既定。unix ソケットが最速 → docs/postgres.md）
brew install postgresql@17
# 4) Docker Desktop（マイグレーション = Flyway を docker image で実行。compose / CI も同じ）
#    起動しておくだけ（`./run db-migrate` が host.docker.internal 経由で native PG に繋ぐ）
# 5) bash 5.x 推奨（macOS 既定の 3.2 でも ./run は動くが 5.x が望ましい）→ ./docs/bash-setup.md
# 6) CSS ゲート用（最終確認のときだけ。日常はCDNなので不要）
uv tool install semgrep   # css-check の semgrep（CIと同手段。uv未導入なら brew install semgrep）
./run css-setup           # Tailwind CLI + daisyUI を assets/ に取得（.gitignore 済み）
```

## 動かす

```sh
./run db-start            # ネイティブ Postgres を起動（常駐ログインにしない=run）
./run db-setup            # DB 作成 + Flyway で migrations 適用（docker image 経由 / 冪等）
./run dev                 # = cargo run -p app
# → http://127.0.0.1:3000 にカウンター（worktree dan2 等では自動で 3002 等／下記「worktree 並列起動」）。
#   「+1」で増え、再起動しても値が残る（DB永続化）。
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

## マイグレーション（Flyway）

DB スキーマは **Flyway** で管理する。CLI/JRE はローカルに入れず **docker image（`flyway/flyway`）** で実行する
（compose / CI と同じ版）。native dev は `host.docker.internal` 経由で native postgres に繋ぐので、Docker Desktop を
起動しておくだけでよい（postgres 側の設定変更は不要）。接続先の database 名は worktree スロット（`$PG_DB`）に追従する。

```sh
./run db-migrate          # migrations/ を適用（未適用ぶんだけ。docker image 経由）
./run db-info             # 適用状況の一覧（flyway info）
./run db-reset            # DBを作り直して全migrationを流し直す（使い捨て前提）
```

**ファイル名は `V<version>__<desc>.sql`（区切りは `__` 2個）**。版数は連番（`V1`,`V2`..）ではなく
`yyyyMMddHHmmss` の**タイムスタンプ**にする ── ブランチを並行で切ったときの版数衝突を避けるため。

```
migrations/V20260614153000__init_counter.sql    # counter テーブル
migrations/V20260614153500__seed_counter.sql    # 初期行 id=1, value=0
```

挙動は `flyway.toml` に固定（接続情報は env で実行時に渡す）:

- `outOfOrder = true` ── タイムスタンプはブランチ合流で前後しうる。適用済みより小さい版数が後から現れても、
  無視せず適用する（連番前提の「順番どおりでないとエラー」を回避）。
- `validateMigrationNaming = true` ── 命名規約から外れた `.sql` があれば fail-fast で気付ける。

> versioned migration は一度しか走らないので素の DDL を書く（`create table if not exists` のような冪等ガードは
> 付けない）。既存DBに後付けで Flyway を入れる場合は履歴が無く既存テーブルと衝突するので、
> 一度 `./run db-reset` してまっさらにしてから流す。

## 開発ループ（4層）

1. **テンプレ・CSS・HTMX属性** → 保存で即反映（Rustビルドゼロ。作業の7〜8割）。`./run dev` 起動中にテンプレを直すだけ。
2. **ハンドラ・サービス層（get_count 等）** → 該当クレートだけ数秒で再ビルド。`./run watch`（bacon）が裏で回る前提。
3. **スキーマ（.proto）** → `schema` クレートで codegen が走る（proto を触ったときだけ）。
4. **DBスキーマ（migrations）** → `migrations/` に `V<timestamp>__*.sql` を足して `./run db-migrate`（上記「マイグレーション」）。Rustビルドには影響しない。

```sh
./run watch               # bacon（既定=check）: 保存で型チェックを回す。サーバ起動/再起動はしない
./run dev                 # サーバ起動（保存後に手で叩き直す）。bacon run/serve で再起動運用も可
./run check               # cargo check --workspace（型エラーを最速で拾う）
./run release             # 本番相当(stable + release + 本番CSS)でローカル起動（後述「3つのモード」）
```

### 4つのモード（CSS × ツールチェイン × プロファイル）

| モード        | コマンド              | CSS                                     | ツールチェイン/プロファイル                         | 用途                                                  |
| ------------- | --------------------- | --------------------------------------- | --------------------------------------------------- | ----------------------------------------------------- |
| 高速開発      | `./run dev`           | CDN（ブラウザJIT・ビルドゼロ）          | nightly / debug                                     | 日常の作業（7〜8割）                                  |
| CSSビルド開発 | `CSS=built ./run dev` | CLI生成 `/static/app.css`               | nightly / debug                                     | 本番CSSの最終目視（往復で再ビルドしない）             |
| **リリース**  | `./run release`       | CLI生成（minify・release は常にこちら） | **stable / release**（Rust既定 opt-3 のみ）         | 本番に出すのと同じ build をローカルで実行             |
| デプロイ相当  | `./run release-max`   | 同上                                    | **stable / release-max**（opt-3 + LTO=fat + cgu=1） | 最大最適化での動作確認・比較（ビルドは約3.2倍かかる） |

`./run release` / `./run release-max` は **dev=nightly のまま本番だけ stable** にするための入口（Docker と同じ仕組み）:
`assets/tailwindcss` で本番CSSを minify 生成 → `assets/strip-nightly.sh` で Cargo.toml の nightly 専用行を
一時的に剥がす（`trap` で必ず復元） → `RUSTUP_TOOLCHAIN=stable RUSTFLAGS="" cargo run --release -p app`
（`release-max` は `--profile release-max`）。成果物は `target/release/` と `target/release-max/` に分かれて共存するので、
同条件で並べて比較できる。DB は dev と同じ native（`./run db-setup` 済み前提）。`./run css-setup` でTailwindを取得していること。

> Rust 変更の反映は再ビルド + プロセス再起動で約1秒（体感の端から端は ~1.2〜1.3s）。
> cold start の正体・短縮策（codesign / systemfd / リンカ）の実測は [`docs/cold-start.md`](./docs/cold-start.md)。
> 設計とビルドツール両面の高速化施策の総まとめは [`docs/fast-rust.md`](./docs/fast-rust.md)。
> `release` と `release-max` の実測差（throughput +2.0% / バイナリ -34% / build 約3.2倍）は
> [`docs/build-speed.md` ⑥](./docs/build-speed.md)。

## CSS（日常はCDN・最終確認だけビルド）

CSSモードは実行時グローバル `css_built` で切替（`base.html`）。`cfg!(debug_assertions)` には紐付けない
＝ debug↔最終確認を往復しても Rust 再ビルドが走らない（プロジェクトの肝）。

```sh
# 日常: 何もしない（debug 既定 = CDN でブラウザJIT、ビルド/watch 不要）
./run css-check           # push前ゲート: Tailwind CLI フルパージ生成 + semgrep（動的合成クラス検出）
CSS=built cargo run -p app  # 最終目視: CLI生成の /static/app.css を配信（release は常にこちら）
```

## lint / format（push前ゲート）

種別ごとに最適なツールを当てる（Rust=rustfmt/clippy・他=oxfmt・proto=buf・shell=shfmt/shellcheck・SQL=sqlfluff。
1本では賄えない）。配線とルールは [`lint/`](./lint/) に自己完結（選定根拠は [`docs/lint-format.md`](./docs/lint-format.md)）。

```sh
./run lint-setup          # 初回のみ: oxfmt を lint/.lint-tools にローカル固定、不足分は brew
./run lint                # 通しゲート: 全種別を fmt/lint チェック（読み取り専用 = CI 兼用）
./run fmt                 # push前の一括整形（書き込み）→ ./run lint を緑にする
```

> `./run css-check` と同じく**節目に手動**で回す（pre-commit は使わない）。整形の使い分けは
> **dev ループ中は触ったクレートだけ**（`cargo fmt -p <crate>`。触っていないクレートまで再ビルドさせない）、
> **push 前は `./run fmt` で一括**（どうせ release ビルドするので相乗り）。`cargo fmt` は差分のある
> ファイルだけ書き戻すので整形済み分は再ビルドされない（詳細 [`lint/README.md`](./lint/README.md)）。

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

## ポートと DB 接続（既定値）

何も指定しないとき（本流 `eikodan`）に使うポートは 3 つだけ。

| 何                        | ポート     | 誰が使うか                                                     |
| ------------------------- | ---------- | -------------------------------------------------------------- |
| app（HTML + Connect API） | **3000**   | `./run dev` / `./run release` / `./run up`（compose の公開側） |
| ネイティブ Postgres       | **5432**   | マシンに 1 つだけ常駐。全 worktree で共有                      |
| `./run ci` の使い捨て PG  | **5433〜** | `./run ci` の実行中だけ。終了時に `docker rm -f` で消える      |

**worktree で動くのは app のポートだけ**（`dan2` → 3002。下記「worktree 並列起動」）。

### 経路ごとの接続方式

| 経路                  | app の居場所      | DB                   | 接続              | DB ポート               |
| --------------------- | ----------------- | -------------------- | ----------------- | ----------------------- |
| `./run dev`（日常）   | ホスト            | ネイティブ（共有）   | **unix ソケット** | 使わない                |
| `./run up`（compose） | コンテナ          | コンテナ             | TCP（同一網）     | ホストに公開しない      |
| `./run ci`（CI 再現） | ホスト（release） | コンテナ（使い捨て） | TCP（localhost）  | **5432+slot**（5433〜） |

- **dev が unix ソケットなのは速度のため。** Mac ではホスト→コンテナが VM 越えになり、
  実測 218,701 tps に対し Docker 経由 18,241 tps（約 1/12。pgbench c8 SELECT →
  [`docs/postgres.md`](./docs/postgres.md)）。`db::connect()` が `DATABASE_URL` の有無で分岐する。
- **`./run ci` だけ DB をホストに公開する**のは、app がコンテナの外（ホストの release バイナリ）に
  いるため。native PG の 5432 と他 worktree の ci に当たらないよう app ポートと同じ幅ずらす。

## worktree 並列起動（ポート/DB 自動分離）

`git worktree` で `dan1`〜`dan4` を分けて同時に動かしてもぶつからない。**`./run` が worktree の
ディレクトリ名を分離キーにして**、`PORT`・`PGDATABASE`・`COMPOSE_PROJECT_NAME` を揃えて
export する（手で割り当てる必要も、worktree ごとの設定ファイルも無い）。

- **DB 名・compose プロジェクト名** … worktree 名をそのまま使う（`dan2` → `lastshot_dan2`）。
- **ポート** … **worktree 名の末尾数字**がそのまま下位桁になる（`dan2` → 3002、`dan12` → 3012）。

| worktree          | app ポート | dev DB（database 名） | compose プロジェクト | `./run ci` の使い捨て PG |
| ----------------- | ---------- | --------------------- | -------------------- | ------------------------ |
| `eikodan`（本流） | 3000       | `lastshot`            | `lastshot`           | 5432                     |
| `dan1`            | 3001       | `lastshot_dan1`       | `lastshot_dan1`      | 5433                     |
| `dan2`            | 3002       | `lastshot_dan2`       | `lastshot_dan2`      | 5434                     |
| `dan3`            | 3003       | `lastshot_dan3`       | `lastshot_dan3`      | 5435                     |
| `dan4`            | 3004       | `lastshot_dan4`       | `lastshot_dan4`      | 5436                     |

### 名前の末尾には数字を付ける（付いていないと止まる）

末尾に数字が無い worktree 名（例 `feat-search`）は**エラーで止まる**。

```
$ ./run dev
worktree 名 'feat-search' の末尾に数字が無いのでポートが決まりません。
  対処1: 数字付きの名前で worktree を切る (例: dan5 → PORT=3005)
  対処2: この場限りなら明示する    (例: PORT=3009 ./run dev)
```

**「空いているポートを自動で取る」形は採らない。** 自動割り当てだと `dan1` のポートが起動のたびに
変わって「今どれだっけ？」になる。ポートは worktree 名から**必ず同じ数値に決まる**ほうがよい
（→ [`docs/decisions/README.md`](./docs/decisions/README.md) #13）。

> 本流の判定はディレクトリ名 `eikodan`。別名でクローンしている場合は `MAIN_WORKTREE=<名前>` を渡す
> （`MAIN_WORKTREE=myrepo ./run dev` → 3000 / `lastshot`）。
> 末尾数字が同じ worktree を 2 つ作る（`dan2` と `feat2`）とポートは衝突するが、DB 名は名前ベースなので
> 分かれたままで、衝突は起動時の bind エラーですぐ分かる。

### 初期セットアップ（worktree を増やしたとき）

Postgres は全 worktree で 1 つを共有するので、`db-start` は **マシンに 1 回だけ**でよい。
あとは各 worktree で `db-setup` を流すと、その worktree 用の database（例 `lastshot_dan2`）が作られる。

```sh
# 〔マシンに1回〕ネイティブ Postgres を起動（本流 eikodan 等どこか1つで）
./run db-start

# 〔新しい worktree ごとに1回〕この worktree 用の DB を作成＋Flyway で migrations 適用（冪等）
git worktree add ../dan5 -b feat/xxx   # 名前の末尾に数字を付ける（= ポート 3005）
cd ../dan5/lastshot
./run db-setup        # → database `lastshot_dan5` を作成（PGDATABASE は ./run が自動で決める）
./run dev             # → http://127.0.0.1:3005（ポートも名前から自動で決まる）
```

> 既存の `lastshot` 1 個で動かしていた worktree も、上の `./run db-setup` を一度流せば
> `lastshot_danN` に切り替わる（旧 `lastshot` は本流 eikodan が使い続ける）。
> やり直したいときは `./run db-reset`（その worktree の DB だけ作り直す。カウント値もリセット）。

- **DB は 1つのネイティブ Postgres を共有し、database 名だけ分ける**（`./run db-setup` が
  `lastshot_dan2` を作る）。ポートでは分けない（上記「ポートと DB 接続」）。
- **compose も同じ機構**。postgres を publish せずプロジェクト名で分離するので、`./run up` を
  複数 worktree で同時に立てても衝突しない。
- **テスト並列**: worktree ごとにポートも DB も別なので、`dan1` と `dan2` で同時に
  `./run dev` → `./run test-http` を回しても干渉しない（`tests-http` は `BASE_URL` を読む）。
- 各値は外から渡せば優先（例: `PORT=3009 ./run dev`）。1つの worktree 内で nextest が
  **テスト同士**を並列化するための「テストごと別DB」は、`tests-http` の「1サーバ＝1DB」ブラックボックス
  設計と噛み合わないので採らない（テストは順序非依存に書く）。

## CI / コンテナ

```sh
./run ci                  # CI(lastshot-ci.yml)をローカルで一気通し（手元でCIを再現）
./run up                  # app + postgres を docker compose 同一網で起動（DATABASE_URL=TCP）
./run down                # = docker compose down -t 0（即 SIGKILL / 使い捨て前提）
```

- `./run ci`: CI と同じ流れ（build(release) → CSSゲート → 起動 → `test-http` → ブラウザE2E）を
  手元で一気通しする。接続は CI と同じ **TCP**（`DATABASE_URL` 優先 ＝ 開発既定の unix ソケットでなく
  compose 同一網作法）。postgres は使い捨てコンテナを **worktree ごとのポート**（`5432+slot`）で立て、
  migration は CI と同じ Flyway（`db-migrate` を `--network=host` で使い捨て pg に向ける）で適用。
  終了時に `trap` で必ずアプリ停止＋コンテナ破棄（`docker rm -f` ＝ 即 SIGKILL）。アプリの停止は
  PID を覚えず **`$PORT` を LISTEN しているプロセスを `kill -9`**（前回の残骸も掃除できる。起動前にも
  実行するので、ポートを塞いだ別サーバ相手にテストが緑になる事故が起きない）。dan1〜dan4 を並列で
  回しても衝突しない。内部の段階（`build-release` / `ci-db-up` / `ci-migrate` / `ci-app-start` /
  `ci-app-wait` / `ci-app-stop` / `ci-db-down`）も個別タスクとして手で叩ける（`./run help` 参照）。
- `compose.yml` + `Dockerfile`: app（release / 本番CSS入り）と postgres:17 と flyway を同一網で起動。
  起動順は postgres(healthy) → flyway(migrate して exit) → app（app は `service_completed_successfully` を待つ）。
  接続は本番/CI と同じ `DATABASE_URL` の TCP。データボリュームは持たない（使い捨て＝down で消え、up で
  flyway が migrations を流し直す）。コンテナは dev 機向け最適化（sccache/mold/-Zthreads）を env で無効化した
  素ビルド（追加ツール不要・stable + protoc のみ）。**dev=nightly / 本番=stable**: Dockerfile が
  `RUSTUP_TOOLCHAIN=stable` で rust-toolchain.toml(nightly) を上書きし、`assets/strip-nightly.sh` で
  Cargo.toml の nightly 専用行（`cargo-features`/`codegen-backend`）を剥がして stable でビルドする
  （nightly フラグはこの構造ではビルド速度にほぼ寄与せず、本番は再現性重視で stable に倒す）。
- CI: `.github/workflows/lastshot-ci.yml`（**リポジトリ直下**。GitHub は monorepo でも root の
  `.github/workflows/` しか実行しないため。`lastshot/**` だけを対象に path フィルタ）。中身は
  build(release) → CSSゲート → 起動 → `test-http` → ブラウザE2E をネイティブで一気通し。migration は
  `./run db-migrate`（postgres は service コンテナなので `--network=host` + localhost で繋ぐ＝ローカルと同じ Flyway）。
- 「compose分割 vs 全部入りsingle」の CI環境比較は計測フェーズに保留中（[`docs/container-ops.md`](./docs/container-ops.md)）。
  まずは「緑になる CI を1本」通す段階。

### CI の高速化方針（ARM 実機計測で取捨選択）

> **測り方・実数・採否理由の詳細は [`docs/ci-performance.md`](./docs/ci-performance.md)**（調査ログ）。ここは要約。

runner は `ubuntu-24.04-arm`（ローカル Apple Silicon・arm64 Docker と**アーキ一致**。public repo で無料）。
setup 区間は**推測せず ARM 実機ベンチで効果を測って**取捨選択した:

- **採用**: semgrep を `pipx`→`uv tool install`（導入 ~16s→~2.5s）/ `apt-get update` 省略（失敗時のみ
  update→retry で自己回復）/ postgres `17`→`17-alpine`（コンテナ初期化 ~14s→~10s）。
- **採用キャッシュ① `Swatinem/rust-cache`（cargo+target, build ~76s→~15s）**＝本命。`nextest` も
  `taiki-e/install-action` で導入済みキャッシュ。
- **採用キャッシュ② Playwright Chromium + npm（`actions/cache` で `~/.cache/ms-playwright` +
  `browser/node_modules`）**。ARM 実機ベンチで browser-setup 21s→6s + restore 5s ＝ 正味 ~10s 短縮、
  保存(Post)はキー命中時 0s（payload ~120MB と小さく rust-cache の 10GB 枠も圧迫しない）。本命の狙いは
  **Chromium DL のネットワーク変動（時々30-40sに跳ねて CI 総時間を暴れさせる）を消すこと**。テスト実行
  自体（npm test）は cache 有無に関わらず安定 ~4s。キーは `package-lock.json` の hash 連動＝playwright 更新で自動失効。
  （当初は「payload 小で相殺」と却下したが、再計測で Chromium DL のばらつきが大きいと分かり採用に変更。）
- **足さないと決めたキャッシュ（実機で逆効果/無駄と確認）**: semgrep の pip cache（重さは wheel DL でなく
  venv 展開なので pip cache では縮まない＝uv で解決）/ pipx venv cache（warm でも当たらない）/
  **rustup toolchain cache（保存48s ≫ 復元4s、巨大で rust-cache の 10GB 枠を圧迫）** / apt cache（payload 小で restore 相殺）。
- 原則: **「少数の太いキャッシュ」だけ**。細かいキャッシュは restore/保存コストで相殺〜逆効果になる。
- 総 CI 時間は約2分で **x86 時とほぼ同等**（大半が arch 非依存の apt/コンテナ初期化/build/test と run毎の
  ばらつき）。ARM 化の主目的は速度ではなく**ローカルとのアーキ一致**。semgrep の scan は
  `--config assets/semgrep` のローカルルールで実行時のレジストリ DL は無い（ローカル実測 0.9s）。

## 構成

```
lastshot/
  run                  bash 関数ディスパッチャ（タスクランナー。./run help で一覧）
  rust-toolchain.toml  nightly + rust-src
  .cargo/config.toml   lld + -Zthreads + sccache（ビルド高速化フラグ）
  Cargo.toml           workspace（dev=全クレート opt-level 0 / Cranelift）
  bacon.toml           check / run / serve
  rustfmt.toml         Rust 整形ルール（直下に置く唯一の lint 設定 = cargo fmt の自動探索アンカー）
  assets/              CSSゲート（input.css / setup-css.sh / check-css.sh / semgrep/）+ strip-nightly.sh（本番stable化）
  lint/                fmt/lint ゲート（setup.sh / check.sh / fmt.sh / .oxfmtrc.json / .sqlfluff）。proto の buf.yaml は schema 側に同居
  migrations/          Flyway versioned migrations（V<timestamp>__<desc>.sql / 連番でなくタイムスタンプ版数）
  flyway.toml          Flyway 挙動設定（outOfOrder=true / validateMigrationNaming=true / locations）
  crates/
    app/               bin。ルーター組み立て・起動・ライブリロード・/static/app.css 配信のみ
    webcore/           共有コア（AppState = MiniJinjaローダ + DBプール）。render / render_view / render_view_fragment
    schema/            .proto + build.rs codegen = 単一の真実。HTML/JSON/API が共有する生成型
    db/                Postgres 接続プール（薄い）。query! マクロは使わない
    feature-counter/   1機能 = get_count/increment（service層）+ HTMLルート + templates/（依存は webcore/schema/db だけ）
    rpc/               Connect API の薄い殻。service層を呼んで同じ CounterView を返す
  tests-http/          起動済みサーバを HTTP で叩くブラックボックステスト（ワークスペース外）
  browser/             ブラウザ駆動E2E（Playwright / HTMX swap・view-data・API の一致検証 / ワークスペース外）
  Dockerfile           app の release イメージ（本番CSS入り / 素ビルド / 本番=stable。strip-nightly.sh 同梱）
  compose.yml          app + postgres + flyway 同一網（TCP接続 / 使い捨て / app は flyway 完了を待つ）
  （CI は monorepo 直下の ../.github/workflows/lastshot-ci.yml。GitHub は root の .github しか実行しないため）
```

高速化フラグの置き場所は3か所に分かれている（リンカ/threads/sccache=`.cargo/config.toml`、nightly=`rust-toolchain.toml`、
dev=全クレート opt-level 0 / Cranelift=`Cargo.toml`）。これらは **dev(nightly) 向け**で、**本番(Docker)は stable で焼く**
（`RUSTUP_TOOLCHAIN=stable` + `assets/strip-nightly.sh`。上記「4つのモード」「CI / コンテナ」参照）。
実測根拠は [`docs/build-speed.md`](./docs/build-speed.md)。
