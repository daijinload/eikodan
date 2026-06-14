# lastshot-next — Node.js + Next.js 版カウンター

[lastshot 3スタック比較](../lastshot-bench/)の Next.js 実装。lastshot（Rust）と
**全く同じ DB・同じ画面**の DB保存カウンター（数字 ＋「+1」ボタンだけ）を、Next.js の
**素のアーキテクチャ（App Router・RSC + client hydration）**で組んで横並び計測する。

## 画面と経路

- `GET /` — Server Component で現在値を DB から読み、SSR（`app/page.tsx`、`force-dynamic` で
  毎リクエスト DB を読む）。クライアント境界（`app/counter.tsx` の `"use client"`）が
  hydration を生み、ページは React/Next の chunk を読み込む ── これが「1 画面あたりの
  リクエスト数（fan-out）」として観測される、Next.js 側の正体。
- `POST /api/increment` — Route Handler（`app/api/increment/route.ts`）。`UPDATE ... RETURNING`
  で +1 して `{value}` を返す、lastshot の `POST /increment` に対応するクリーンな計測点。
  React を挟まない薄い JSON なので GET より大幅に速い（ベンチ参照）。

ロジックの本体は [`lib/db.ts`](./lib/db.ts) の `getCount` / `increment`。lastshot の
`crates/feature-counter`（`get_count` / `increment`）と同じ SQL を node-postgres で叩く。

## DB は lastshot と共有

`lib/db.ts` は lastshot（`crates/db`）と同じ方針:

- `DATABASE_URL` があればそれを使う（本番/CI の TCP）。
- 無ければネイティブ PG の **unix ソケット（`/tmp`）**へ繋ぐ（pg-bench の結論 = unix ソケット最速）。
  database 名は `PGDATABASE`（worktree ごとに `lastshot_dan3` 等）、ロールは OS ユーザー（trust）。

スキーマ（`counter` テーブル）は **lastshot の Flyway が所有**する。こちらは
マイグレーションせず既存の 1 行（id=1）を読み書きするだけ。`./run` が `PGDATABASE` を
lastshot と同じ値に揃えて export するので、何もしなくても同じ DB を共有する。

## セットアップと起動

```sh
./run setup           # npm install（初回のみ）
./run prod            # next build → next start（本番相当・1 Node プロセス。計測の既定はこれ）
./run prod-cluster    # next build → PM2 cluster × N ワーカ(既定15・同一ポート共有)。追加実験用
./run dev             # next dev（オンデマンド compile。計測には使わない）
```

`prod-cluster` は [`../lastshot-bench/`](../lastshot-bench/) の「Next.js を多重化したら lastshot との
RPS 差は縮むか」追加検証のために用意したもの。`ecosystem.config.cjs` + `cluster-start.cjs` で
next を programmatic に起動し、Node の cluster モジュール経由で 1 ポートを N ワーカが共有する。
`INSTANCES`（既定 15 = 論理コア数）と `PG_POOL_MAX`（既定 4。15×4=60 < `max_connections=100`）は
env で上書き可。本来の比較（素の `next start`）は `./run prod` のままにしてある。

ポートは worktree スロットで `3100 + slot`（`eikodan`→3100 / `dan3`→3103）。lastshot=3000 番台 /
laravel=3200 番台 と衝突しない。`./run url` で URL を表示。前提: ネイティブ PostgreSQL が起動済みで、
lastshot 側で `./run db-setup` 済み（= `counter` テーブルがある）こと。

```sh
curl http://127.0.0.1:3103/                       # 画面（現在値を SSR）
curl -X POST http://127.0.0.1:3103/api/increment  # => {"value":N}
```

## 計測上の位置づけ

dev（`next dev`）はオンデマンド compile で数字が環境ノイズだらけになるので、**必ず `./run prod`
（`next build` → `next start`）で測る**。serving は Next.js の素の `next start`（1 Node プロセス）。
比較の文脈・結果・落とし穴は [`../lastshot-bench/`](../lastshot-bench/) にまとめてある。
