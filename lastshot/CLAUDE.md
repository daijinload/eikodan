# lastshot — AIエージェント運用ルール

`eikodan` の各サブプロジェクトの結論を統合した本番実装。土台は2つ:
- **[fastweb](../fastweb/CLAUDE.md)** = ビルド回避 + package by feature（ノービルド開発の掟）
- **[connectweb](../connectweb/CLAUDE.md)** = スキーマファースト（.proto 単一真実の掟）

両者の `CLAUDE.md` のルールは**原則すべて引き継ぐ**。ここには lastshot 固有の差分（特に **DB 作法**）だけを書く。
構造を作ってもエージェントの行動が局所性を壊すと意味がないので、以下を守ること。

## 大方針（4層の開発ループ）

1. **テンプレ・CSS・HTMX属性の変更** → 保存で即反映（Rustビルドゼロ。作業の7〜8割）。
2. **ハンドラ・サービス層（get_count/increment 等）の変更** → 該当 feature クレートだけ数秒で再ビルド。
3. **スキーマ（.proto）の変更** → `schema` クレートで codegen が走る（proto を触ったときだけ）。
4. **DB スキーマ（migrations）の変更** → `migrations/` に `V<timestamp>__*.sql` を足して `./run db-migrate`
   （まっさらに作り直すなら `./run db-reset`）。Flyway が適用するので Rustビルドには影響しない。

動的言語・スクリプト層は足さない。Rust の型チェックを品質担保装置として全面的に残す。

## スキーマファーストの掟（connectweb から継承）

- **型は .proto に定義する**。手書きの表示用 struct を別に作らない。画面・API・埋め込みJSONは
  同じ生成型（`CounterView`）を共有する。**ビュー専用メッセージ**を切り、画面に出す約束のフィールドだけ載せる
  （「全データ入りの型を一部だけ使う」は情報境界を壊すので禁止）。
- **フルページは `render_view`、HTMXフラグメントは `render_view_fragment`** を使う。同じインスタンスを
  描画と `<!-- view-data -->` 埋め込みの両方に流す（データ取得は1回 ── 描画用と埋め込み用で別取得しない＝ズレる）。
  素のHTML（埋め込み不要）は `render`。view-data は `<script>` でなく**HTMLコメント**＝本番DOM/JSに出さないデバッグ覗き窓。
- **テンプレも埋め込みJSONも camelCase**。buffa の serde は proto3 JSON 準拠（proto の `recent_activities` は
  `view.recentActivities`）。**proto3 JSON は 0 値を省略する**ので、テストやテンプレで「未設定＝0」を考慮する。

## DB 作法（lastshot 固有 ── これが一番の追加点）

- **`db` を触るのは feature の service 層だけ**。`webcore` / `rpc` / `app` から直接クエリを書かない。
  `rpc` も `app` も「service 層関数（get_count/increment）を呼ぶ」だけで、SQL を持たない。
- **`query!` 系マクロは使わない**。`db::sqlx::query(...)`（ランタイムAPI）で書く。理由: コンパイル時に
  DB 接続を要求しないことでビルド速度を保ち、「DB が無くてもコンパイルできる」を守る（fastweb のビルド速度が肝）。
- **service 層の戻り値は proto 生成型（`CounterView` 等）に保つ**。DB の行をそのまま外へ出さず、生成型に詰め直す。
  これで HTML 経路と rpc 経路が同じ型を共有でき、スキーマファーストが DB 越しでも崩れない。
- **`sqlx` は `db` クレートに閉じる**。feature 側は自分の Cargo.toml に sqlx を書かず `db::sqlx` / `db::Row` を使う
  （依存を1か所に集約 ＝ workspace 共通 deps を不用意に増やさない）。
- **接続先**: 開発は既定でネイティブPGの unix ソケット（pg-bench の結論で最速）。本番/CI は `DATABASE_URL` で
  TCP（compose 同一網）に上書き。`db::connect()` がこの分岐を持つ。dev の database 名は `PGDATABASE`
  で上書き可（既定 `lastshot`）。`./run` が worktree 名からスロットを決めて `PORT`/`PGDATABASE`/
  `COMPOSE_PROJECT_NAME` を export し、dan1〜dan4 を並列起動しても衝突しない（README「worktree 並列起動」）。
- **migrations は Flyway で管理する**（docker image `flyway/flyway` で実行 ── ローカルに JRE/CLI は入れない）。
  ファイルは `migrations/V<version>__<desc>.sql`（区切りは `__` 2個）。版数は連番(`V1`,`V2`..)ではなく
  `yyyyMMddHHmmss` の**タイムスタンプ**にする＝ブランチ並行時の版数衝突を避けるため
  （例: `V20260614153000__add_users_index.sql`）。挙動は `flyway.toml` に固定:
  `outOfOrder=true`（タイムスタンプが前後しても適用＝連番前提の「順番どおりでないとエラー」を回避）、
  `validateMigrationNaming=true`（命名ミスを fail-fast）。versioned migration は素の DDL を書く
  （一度しか走らないので `if not exists` 等の冪等ガードは付けない）。
- **migrations の適用**: native=`./run db-migrate`（`host.docker.internal` 経由で native postgres へ。
  DB名は worktree スロットの `$PG_DB` を使うので分離と両立）、compose/CI=`flyway` サービス / CI ステップ
  （app は `service_completed_successfully` を待つので必ず適用後に起動）。DB は使い捨て前提（native は
  `./run db-reset` で作り直し、compose はボリューム無し）。既存DBに後付けで Flyway を入れる場合は履歴が
  無く既存テーブルと衝突するので、一度 `./run db-reset` してまっさらにしてから流す。

## クレート依存の向き（package by feature + schema + db）

- `schema` は土台。**全クレートが共有する**（依存は buffa/connectrpc/serde のみ）。
- `db` も土台。**service 層（feature）と app が共有する**（依存は sqlx のみ、`db` クレートに閉じる）。
- `feature-*` の依存は **webcore / schema / db だけ**。feature 間依存・connectrpc 直依存・`webcore→feature` 逆依存は禁止。
- `rpc` は Connect API の薄い殻。**service 層関数を import して呼ぶだけ**。`rpc → feature-*` はOK、逆は禁止。
  同一プロセスなので**自分自身への gRPC ループバックを張らない**（関数呼び出しで済ます）。
- `app`(bin) は薄い層: ルーター組み立て・起動・ライブリロード・DBプール生成・`/static/app.css` 配信のみ。

## ビルド/codegen を増やさないための禁止事項（fastweb から継承）

- **触る機能のフォルダ以外に書き込まない。** 全ファイルへのフォーマッタ一括適用、workspace 共通 `Cargo.toml`
  （`[workspace.dependencies]` / プロファイル）の不用意な変更は全クレート再ビルドを誘発するので禁止。
- **UIの変更はテンプレートとCSSで完結させる。** Rust に触るのはデータ取得の形が変わるときだけ。HTMLはRustに書かず
  テンプレに置く（`assets/input.css` は `source(none)` でテンプレHTMLだけを走査対象にしている）。
- **Tailwind のクラス名は常に完全形で書く。** `text-{{ color }}-500` のような動的合成は禁止（CLIパージで本番だけ消える。
  日常はCDNなので開発中は再現せず、push前の `assets/check-css.sh`（semgrep）が代わりに捕まえる）。
- **codegen は `schema` に隔離。** build.rs(protoc) が走るのは proto を変えたときだけ。proto を変えたら
  `cargo check -p schema` で確認してから利用側を直す。
- **CSSモードは実行時グローバル `css_built`（base.html）で切替。** `cfg!(debug_assertions)` に紐付けない
  ── debug↔最終確認の往復で Rust 再ビルドを走らせないため。

## イテレーションの回し方

- 確認は **`cargo check -p <feature>`**（+ `cargo clippy -p <feature>`）。フルビルドを待たない。`./run watch`（bacon）前提。
- テストは触っているクレートの単体のみ（`cargo nextest run -p <feature>`）。HTTP越しの統合は `tests-http/`（ワークスペース外）で節目に。
- **push前のCSSゲート**: `./run css-check`（クリーンビルドでパージ確定 + semgrep）。pre-commit は使わず節目に手動で回す。

## このリポの作法

- 推測で埋めない。事実（実データ・一次ソース・実際に動かした結果）を起点に答える。検証できない場合は「不明」と明記する。
  クレートのAPIは docs.rs や `~/.cargo/registry/src/` の実ソースで確認してから使う。
- マージはユーザー（daijinload）が行う。Claude は作成/push まで。force push しない。
- 生成コード（proto codegen 等）はコミットしない（OUT_DIR 生成のまま＝単一バイナリ前提・差分ノイズ回避）。
- プロジェクト知識はリポジトリのドキュメント（README/CLAUDE）に残す（Claude 個人メモリには eikodan の知識を保存しない。ルート `CLAUDE.md` 参照）。
