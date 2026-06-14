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
# 5) CSS ゲート用（最終確認のときだけ。日常はCDNなので不要）
uv tool install semgrep   # css-check の semgrep（CIと同手段。uv未導入なら brew install semgrep）
./run css-setup           # Tailwind CLI + daisyUI を assets/ に取得（.gitignore 済み）
```

## 動かす

```sh
./run db-start            # ネイティブ Postgres を起動（常駐ログインにしない=run）
./run db-setup            # DB 作成 + migrations/schema.sql・seed.sql 適用（冪等）
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

## 開発ループ（3層）

1. **テンプレ・CSS・HTMX属性** → 保存で即反映（Rustビルドゼロ。作業の7〜8割）。`./run dev` 起動中にテンプレを直すだけ。
2. **ハンドラ・サービス層（get_count 等）** → 該当クレートだけ数秒で再ビルド。`./run watch`（bacon）が裏で回る前提。
3. **スキーマ（.proto）** → `schema` クレートで codegen が走る（proto を触ったときだけ）。

```sh
./run watch               # bacon（既定=check）: 保存で型チェックを回す。サーバ起動/再起動はしない
./run dev                 # サーバ起動（保存後に手で叩き直す）。bacon run/serve で再起動運用も可
./run check               # cargo check --workspace（型エラーを最速で拾う）
```

> Rust 変更の反映は再ビルド + プロセス再起動で約1秒（体感の端から端は ~1.2〜1.3s）。
> cold start の正体・短縮策（codesign / systemfd / リンカ）の実測は [`COLD-START.md`](./COLD-START.md)。

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

## worktree 並列起動（ポート/DB 自動分離）

`git worktree` で `dan1`〜`dan4` を分けて同時に動かしてもぶつからない。**`./run` が worktree の
ディレクトリ名から“スロット”を決め**、`PORT`・`PGDATABASE`・`COMPOSE_PROJECT_NAME` を揃えて
export する（手で割り当てる必要も、worktree ごとの設定ファイルも無い）。

| worktree | app ポート | dev DB（database 名） | compose プロジェクト |
|---|---|---|---|
| `eikodan`（本流） | 3000 | `lastshot` | `lastshot` |
| `dan2` | 3002 | `lastshot_dan2` | `lastshot_dan2` |
| `dan4` | 3004 | `lastshot_dan4` | `lastshot_dan4` |

### 初期セットアップ（worktree を増やしたとき）

Postgres は全 worktree で 1 つを共有するので、`db-start` は **マシンに 1 回だけ**でよい。
あとは各 worktree で `db-setup` を流すと、その worktree 用の database（例 `lastshot_dan2`）が作られる。

```sh
# 〔マシンに1回〕ネイティブ Postgres を起動（本流 eikodan 等どこか1つで）
./run db-start

# 〔新しい worktree ごとに1回〕この worktree 用の DB を作成＋schema/seed 適用（冪等）
cd ../dan2/lastshot   # 例: dan2 worktree へ
./run db-setup        # → database `lastshot_dan2` を作成（PGDATABASE は ./run が自動で決める）
./run dev             # → http://127.0.0.1:3002（ポートも自動でスロット割り当て）
```

> 既存の `lastshot` 1 個で動かしていた worktree も、上の `./run db-setup` を一度流せば
> `lastshot_danN` に切り替わる（旧 `lastshot` は本流 eikodan が使い続ける）。
> やり直したいときは `./run db-reset`（その worktree の DB だけ作り直す。カウント値もリセット）。

- **DB は 1つのネイティブ Postgres を共有**（unix ソケットなので“DBポート”自体が無い）。中の
  **database 名だけ** worktree ごとに分ける（`./run db-setup` が `lastshot_dan2` を作る）。
- **dev も compose も同じ機構**。compose は postgres を publish しないうえ、プロジェクト名で
  ネットワーク/コンテナが分離されるので、`./run up` を複数 worktree で同時に立てても衝突しない。
- **テスト並列**: worktree ごとにポートも DB も別なので、`dan1` と `dan2` で同時に
  `./run dev` → `./run test-http` を回しても干渉しない（`tests-http` は `BASE_URL` を読む）。
- 各値は外から渡せば優先（例: `PORT=3009 ./run dev`）。1つの worktree 内で nextest が
  **テスト同士**を並列化するための「テストごと別DB」は、`tests-http` の「1サーバ＝1DB」ブラックボックス
  設計と噛み合わないので採らない（テストは順序非依存に書く）。

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

### CI の高速化方針（ARM 実機計測で取捨選択）

runner は `ubuntu-24.04-arm`（ローカル Apple Silicon・arm64 Docker と**アーキ一致**。public repo で無料）。
setup 区間は**推測せず ARM 実機ベンチで効果を測って**取捨選択した:

- **採用**: semgrep を `pipx`→`uv tool install`（導入 ~16s→~2.5s）/ `apt-get update` 省略（失敗時のみ
  update→retry で自己回復）/ postgres `17`→`17-alpine`（コンテナ初期化 ~13s→~9s）。
- **本命キャッシュは `Swatinem/rust-cache`（cargo+target, build ~76s→~15s）の1個だけ**。`nextest` も
  `taiki-e/install-action` で導入済みキャッシュ。
- **足さないと決めたキャッシュ（実機で逆効果/無駄と確認）**: semgrep の pip cache（重さは wheel DL でなく
  venv 展開なので pip cache では縮まない＝uv で解決）/ pipx venv cache（warm でも当たらない）/
  **rustup toolchain cache（保存48s ≫ 復元4s、巨大で rust-cache の 10GB 枠を圧迫）** / Playwright・npm・apt
  cache（payload 小で restore 相殺）。
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
