// Node.js 版 lastshot API（Fastify + postgres.js）。
//
// Rust(lastshot) の crates/feature-bench と **同一のクエリ・同一のレスポンス JSON** を返す。
// 片方を変えたら必ず両方直すこと（パリティが崩れると比較が無意味になる）。
//
// 公平性のため Rust と条件を揃える:
//   - 接続は unix ソケット（/tmp）= lastshot dev の既定と同じ最速経路。
//   - 同じ Postgres・同じ database（PGDATABASE）・同じクエリ。
//   - DB接続予算（pool 総数）を揃える: 単一=POOL_MAX、cluster=各 worker を POOL_MAX/WORKERS。
//
// env:
//   PORT=4002 WORKERS=1 POOL_MAX=16 PGDATABASE=lastshot PGHOST=/tmp PGUSER=$USER NODE_ENV=production
//
//   WORKERS=1   → 単一プロセス（1コア）。Rust の TOKIO_WORKER_THREADS=1 と対になる。
//   WORKERS=8   → node:cluster で 8 fork（全コア）。Rust の TOKIO_WORKER_THREADS=8 と対になる。

import cluster from 'node:cluster';
import os from 'node:os';
import Fastify from 'fastify';
import postgres from 'postgres';

const PORT = Number(process.env.PORT || 4002);
const WORKERS = Number(process.env.WORKERS || 1);
const POOL_MAX = Number(process.env.POOL_MAX || 16);
const PGDATABASE = process.env.PGDATABASE || 'lastshot';
const PGHOST = process.env.PGHOST || '/tmp'; // '/' 始まり = unix ソケットのディレクトリ
const PGPORT = Number(process.env.PGPORT || 5432);
const PGUSER = process.env.PGUSER || os.userInfo().username;

if (WORKERS > 1 && cluster.isPrimary) {
  // 全コアモード: WORKERS 個 fork。cluster がポートを共有し接続を分配する。
  console.log(`[primary] forking ${WORKERS} workers on :${PORT} (pool total=${POOL_MAX})`);
  for (let i = 0; i < WORKERS; i++) cluster.fork();
  cluster.on('exit', (w, code, sig) => {
    console.error(`[primary] worker ${w.process.pid} exited (${code}/${sig})`);
  });
} else {
  start();
}

async function start() {
  // cluster 時は各 worker が POOL_MAX/WORKERS を持ち、総数が Rust と揃う。
  const perProc = WORKERS > 1 ? Math.max(1, Math.round(POOL_MAX / WORKERS)) : POOL_MAX;

  const sql = postgres({
    host: PGHOST, // /tmp → /tmp/.s.PGSQL.5432（unix ソケット）
    port: PGPORT,
    database: PGDATABASE,
    username: PGUSER,
    max: perProc,
    // パスワード無し（trust 認証）。prepare はデフォルト true（=速い）。
    onnotice: () => {},
  });

  const app = Fastify({ logger: false });

  // DBなし。ランタイム+HTTP+JSON の素の天井。
  app.get('/ping', async () => ({ ok: true }));

  // 点取得 1往復（Rust の /db/light と同一SQL）。
  app.get('/db/light', async () => {
    const rows = await sql`select value from counter where id = 1`;
    return { value: rows[0].value };
  });

  // bench_rows 全走査の集約（Rust の /db/heavy と同一SQL）。count は ::int で型を揃える。
  app.get('/db/heavy', async () => {
    const rows = await sql`
      select count(*)::int as count, coalesce(avg(n)::float8, 0.0) as avg
      from bench_rows where s like '%abc%'`;
    return { count: rows[0].count, avg: rows[0].avg };
  });

  // pg_sleep（PG CPU ほぼ0の純待ち）。Rust の /db/sleep と同一。
  app.get('/db/sleep', async (req) => {
    const ms = Number(req.query.ms || 20);
    await sql`select pg_sleep(${ms} / 1000.0)`;
    return { slept: ms };
  });

  await app.listen({ port: PORT, host: '127.0.0.1' });
  if (WORKERS <= 1) console.log(`[single] listening on http://127.0.0.1:${PORT} (pool=${perProc})`);
}
