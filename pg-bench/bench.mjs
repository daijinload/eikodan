// アプリ(Node)から各バックエンドに接続し、同一スキーマ・同一ワークロードを叩いて
// 実効スループット/レイテンシを測る。バックエンドごとに別プロセスで起動して公平を期す。
//
// 使い方:
//   node bench.mjs <backend> <label> [connString]
//     backend    : pglite | pgmem | pg
//     label      : 結果ファイル名 (results/<label>.json)
//     connString : pg の時の接続文字列 (省略時は $PG_CONN)
//
import { performance } from 'node:perf_hooks';
import { mkdirSync, writeFileSync } from 'node:fs';

const backend = process.argv[2];
const label = process.argv[3] || backend;
const conn = process.argv[4] || process.env.PG_CONN;

// ---- ワークロード規模 -------------------------------------------------------
const N_USERS = 10_000;
const N_EVENTS = 20_000;
const SINGLE_INSERTS = 3_000; // 1行ずつ commit。サーバ型では WAL/fsync コストの主戦場
const POINT_SELECTS = 10_000; // PK 等値
const IDX_SELECTS = 10_000; // 索引付き列の等値
const UPDATES = 5_000;
const JOINS = 1_000; // JOIN+集約

const SCHEMA = [
  `DROP TABLE IF EXISTS events`,
  `DROP TABLE IF EXISTS users`,
  `CREATE TABLE users (id serial primary key, name text not null, email text not null, age int)`,
  `CREATE INDEX idx_users_email ON users(email)`,
  `CREATE TABLE events (id serial primary key, user_id int not null, kind text not null, payload text, ts bigint)`,
  `CREATE INDEX idx_events_user ON events(user_id)`,
];

// ---- アダプタ: 全部 node-postgres 風の query(text, params)->rows に揃える ----
async function makePglite() {
  const { PGlite } = await import('@electric-sql/pglite');
  const t0 = performance.now();
  const db = await PGlite.create(); // 引数なし = メモリ上 (memory://)
  const initMs = performance.now() - t0;
  return {
    name: 'PGlite (本物PGをWASM化 / in-memory)',
    initMs,
    async query(text, params) {
      return (await db.query(text, params)).rows;
    },
    async end() {
      await db.close();
    },
  };
}

async function makePgmem() {
  const { newDb } = await import('pg-mem');
  const t0 = performance.now();
  const mem = newDb();
  const { Pool } = mem.adapters.createPg();
  const pool = new Pool();
  const initMs = performance.now() - t0;
  return {
    name: 'pg-mem (PGをJS再実装 / in-memory)',
    initMs,
    async query(text, params) {
      return (await pool.query(text, params)).rows;
    },
    async end() {
      await pool.end();
    },
  };
}

async function makePg(connStr) {
  if (!connStr) throw new Error('pg backend には connString が必要');
  const pg = await import('pg');
  const t0 = performance.now();
  const client = new pg.default.Client({ connectionString: connStr });
  await client.connect();
  const initMs = performance.now() - t0; // プロセスからの接続確立コスト
  return {
    name: `Postgres server`,
    initMs,
    async query(text, params) {
      return (await client.query(text, params)).rows;
    },
    async end() {
      await client.end();
    },
  };
}

function makeAdapter() {
  if (backend === 'pglite') return makePglite();
  if (backend === 'pgmem') return makePgmem();
  if (backend === 'pg') return makePg(conn);
  throw new Error(`unknown backend: ${backend}`);
}

// ---- 計測ユーティリティ -----------------------------------------------------
async function timed(label, k, fn) {
  for (let i = 0; i < Math.min(50, k); i++) await fn(i); // ウォームアップ
  const lat = new Float64Array(k);
  const t0 = performance.now();
  for (let i = 0; i < k; i++) {
    const s = performance.now();
    await fn(i);
    lat[i] = performance.now() - s;
  }
  const totalMs = performance.now() - t0;
  const sorted = Array.from(lat).sort((a, b) => a - b);
  const pct = (p) => sorted[Math.min(sorted.length - 1, Math.floor((p / 100) * sorted.length))];
  return {
    label,
    k,
    totalMs: +totalMs.toFixed(2),
    opsPerSec: Math.round(k / (totalMs / 1000)),
    p50ms: +pct(50).toFixed(4),
    p95ms: +pct(95).toFixed(4),
    p99ms: +pct(99).toFixed(4),
  };
}

const rnd = (n) => 1 + Math.floor(Math.random() * n);

async function main() {
  const db = await makeAdapter();
  const phases = {};

  // スキーマ
  let t0 = performance.now();
  for (const stmt of SCHEMA) await db.query(stmt);
  phases.schemaMs = +(performance.now() - t0).toFixed(2);

  // シード (バルクINSERT)
  t0 = performance.now();
  const CHUNK = 500;
  for (let start = 1; start <= N_USERS; start += CHUNK) {
    const end = Math.min(N_USERS, start + CHUNK - 1);
    const rows = [];
    const vals = [];
    let p = 1;
    for (let i = start; i <= end; i++) {
      rows.push(`($${p++},$${p++},$${p++})`);
      vals.push(`User ${i}`, `user${i}@ex.com`, 18 + (i % 60));
    }
    await db.query(`INSERT INTO users(name,email,age) VALUES ${rows.join(',')}`, vals);
  }
  for (let start = 1; start <= N_EVENTS; start += CHUNK) {
    const end = Math.min(N_EVENTS, start + CHUNK - 1);
    const rows = [];
    const vals = [];
    let p = 1;
    for (let i = start; i <= end; i++) {
      rows.push(`($${p++},$${p++},$${p++},$${p++})`);
      vals.push(rnd(N_USERS), 'click', `{"i":${i}}`, 1700000000000 + i);
    }
    await db.query(`INSERT INTO events(user_id,kind,payload,ts) VALUES ${rows.join(',')}`, vals);
  }
  phases.bulkSeedMs = +(performance.now() - t0).toFixed(2);
  phases.bulkSeedRowsPerSec = Math.round((N_USERS + N_EVENTS) / (phases.bulkSeedMs / 1000));

  // 1行ずつ INSERT (= 都度コミット)
  phases.singleInsert = await timed('single INSERT', SINGLE_INSERTS, async (i) => {
    await db.query(`INSERT INTO users(name,email,age) VALUES ($1,$2,$3)`, [
      `New ${i}`,
      `new${i}@ex.com`,
      30,
    ]);
  });

  // PK 等値 SELECT
  phases.pointSelect = await timed('point SELECT (PK)', POINT_SELECTS, async () => {
    await db.query(`SELECT id,name,email,age FROM users WHERE id=$1`, [rnd(N_USERS)]);
  });

  // 索引付き列 SELECT
  phases.indexedSelect = await timed('indexed SELECT (email)', IDX_SELECTS, async () => {
    await db.query(`SELECT id,name FROM users WHERE email=$1`, [`user${rnd(N_USERS)}@ex.com`]);
  });

  // UPDATE
  phases.update = await timed('UPDATE by PK', UPDATES, async () => {
    await db.query(`UPDATE users SET age=$1 WHERE id=$2`, [rnd(80), rnd(N_USERS)]);
  });

  // JOIN + 集約
  phases.joinAgg = await timed('JOIN+aggregate', JOINS, async () => {
    await db.query(
      `SELECT count(*) FROM events e JOIN users u ON u.id=e.user_id WHERE u.age > $1`,
      [rnd(40)],
    );
  });

  await db.end();

  const result = {
    label,
    backend,
    name: db.name,
    node: process.version,
    initMs: +db.initMs.toFixed(2),
    config: { N_USERS, N_EVENTS, SINGLE_INSERTS, POINT_SELECTS, IDX_SELECTS, UPDATES, JOINS },
    phases,
  };

  mkdirSync(new URL('./results', import.meta.url), { recursive: true });
  writeFileSync(
    new URL(`./results/${label}.json`, import.meta.url),
    JSON.stringify(result, null, 2),
  );

  // コンソール出力
  console.log(`\n=== ${db.name}  [${label}] ===`);
  console.log(`init: ${result.initMs} ms   schema: ${phases.schemaMs} ms`);
  console.log(
    `bulk seed: ${phases.bulkSeedMs} ms  (${phases.bulkSeedRowsPerSec.toLocaleString()} rows/s)`,
  );
  for (const key of ['singleInsert', 'pointSelect', 'indexedSelect', 'update', 'joinAgg']) {
    const ph = phases[key];
    console.log(
      `${ph.label.padEnd(24)} ${ph.opsPerSec.toLocaleString().padStart(10)} ops/s  ` +
        `p50=${ph.p50ms}ms p99=${ph.p99ms}ms`,
    );
  }
}

main().catch((e) => {
  console.error(`[${label}] FAILED:`, e.message);
  process.exit(1);
});
