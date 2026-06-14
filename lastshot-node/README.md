# lastshot-node — Rust vs Node.js API速度比較

`lastshot`（Rust: axum + tokio + sqlx）の API を **Node.js（Fastify + postgres.js）に移植**し、
**同じ Postgres・同じクエリ・同じ接続方式**で叩いて速度を比較するための別実装。

## なぜ作ったか

「**DBが律速だから Node.js も Rust も速度は変わらない**」という主張を、実測で検証するため。
主張が本当に成り立つのは **DBがCPUを使い切って律速になる狭い領域だけ**で、実APIの大半が住む
「DBは軽い点取得」領域では言語/ランタイムが効く ── これを **DBの重さを軸にスライド**させて
定量化する。さらに、スループットが収束する重DB領域でも **Rustは同じ負荷を圧倒的に少ない
CPUで・テール(p99)安定で捌く**ことを示し、「同じ速度」という結論の射程を限定する。

## エンドポイント（Rust の `lastshot/crates/feature-bench` と完全一致）

| パス | 中身 | 何を示すか |
| --- | --- | --- |
| `GET /ping` | DBなし `{"ok":true}` | ランタイム+HTTP+JSON の**素の天井**。DBを言い訳にできない。 |
| `GET /db/light` | `select value from counter where id=1`（点取得1往復） | **実APIの大半が住む現実ケース**。 |
| `GET /db/light_pipe` | 上と同一SQL・同一レスポンス。Rust のみ、tokio-postgres の**パイプライン**版 | sqlx(1接続1クエリ)では多コア点SELECTで Node(postgres.js)に負けるが、パイプライン化すると逆転することを示す比較対象。Node 側は `/db/light` が既にパイプライン。 |
| `GET /db/heavy` | `bench_rows`(30万行) を `like '%abc%'` 全走査して集約（≈26ms/PG CPU） | **主張が正しくなる唯一の領域**（収束する境界）。 |
| `GET /db/sleep?ms=N` | `pg_sleep(N/1000)`（PG CPUほぼ0の純待ち） | ロック/IO待ちで遅い場合＝両者収束を正直に提示。 |

> 片方のSQL/レスポンスを変えたら**必ず両方**直すこと（パリティが崩れると比較が無意味）。

## 公平性コントロール（同僚に却下されないための前提）

- **同一 Postgres**（`pg-bench` でチューニング済み: `fsync=off` 等）・**同一クエリ**・**同一接続 = unix ソケット `/tmp`**（`DATABASE_URL` は未設定＝両者ソケット）。
- **DB接続予算を固定**: total pool = `POOL_MAX`(既定16)。単一プロセス=16、cluster(W worker)=各 `16/W` で総和16。→ light は app律速、heavy は DB律速になる pool 設計。
  - パイプライン版(`/db/light_pipe`)は別途 tokio-postgres を `PIPE_CLIENTS`(既定 `POOL_MAX`)本張る。sqlx プールと**同居すると合算が PG の `max_connections` を超えうる**（例: sqlx64 + pipe64 = 128 > 既定100 で `too many clients`）。本リポの計測機は `max_connections=500`（`ALTER SYSTEM` で引上げ済み・要 PG 再起動）。`show max_connections;` で確認し、`POOL_MAX + PIPE_CLIENTS + 余裕` を下回らないこと。
- **並列モデルは両方測る（肝）**:
  - **単一プロセス**: Node 1プロセス vs Rust `TOKIO_WORKER_THREADS=1`（1コア対1コア＝ランタイム純比較。最もクリーン）。
  - **全コア**: Node `cluster` W worker vs Rust `TOKIO_WORKER_THREADS=W`（コア数を揃える。macOSは taskset が無いので worker数で揃える）。
- **本番ビルド**: Rust `--release` / Node `NODE_ENV=production`。
- **ウォームアップ**後に計測（V8 JIT・pool充填・PGプランキャッシュ）。
- **既知の制約**: 負荷生成(oha)が同一マシン同居。全コアモードは oha+PG とコアを奪い合うため「方向性」。最もクリーンなのは単一プロセスモード。CPU は `ps %cpu` サンプルの平均なので近似。

## 動かし方

```sh
# 0) 前提: ネイティブ Postgres 起動 + bench_rows migration 適用（lastshot 側で1回）
cd ../lastshot
./run db-start && ./run db-setup && ./run db-migrate   # bench_rows(30万行) を含む

# 1) Node 依存
cd ../lastshot-node
npm install

# 2) フルベンチ（Rust release ビルド → 計測 → REPORT.md 生成まで一気通し）
cd bench
./bench.sh
node summary.mjs        # 再集計だけしたいとき

# 短縮実行（軸を絞る）
DURATION=5 CONNS="1 64 256" ENDPOINTS="ping light heavy" ./bench.sh
```

サーバ単体で動かす場合:

```sh
# Rust（lastshot）
cd ../lastshot && POOL_MAX=16 TOKIO_WORKER_THREADS=8 PORT=4001 ./target/release/app
# Node
PORT=4002 WORKERS=8 POOL_MAX=16 PGDATABASE=lastshot_dan1 NODE_ENV=production node server.mjs
```

## 構成

```
lastshot-node/
  server.mjs        Fastify + postgres.js（/ping /db/light /db/heavy /db/sleep）。WORKERS で cluster 切替
  package.json      fastify + postgres
  bench/
    bench.sh        オーケストレータ（release起動→warmup→oha matrix→CPUサンプル→停止→集計）
    cpusample.sh    サーバ pid群の %cpu を 0.5s 間隔でサンプリング
    summary.mjs     results/*.json + *.cpu → REPORT.md（両言語横並び表）
    REPORT.md       計測結果と結論（コミット対象）
    results/        生データ（.gitignore）
```

## 結論

→ 計測結果と読み解きは [`bench/REPORT.md`](./bench/REPORT.md)。
