// Postgres 接続プール。lastshot(crates/db) と「全く同じ DB」を共有する。
//
// 方針(lastshot と揃える):
//  - DATABASE_URL があればそれを使う(本番/CI の TCP)。
//  - 無ければ開発既定としてネイティブ PG の unix ソケット(/tmp)へ繋ぐ
//    (pg-bench の結論=unix ソケット最速)。DB 名は PGDATABASE(worktree ごとに
//    lastshot_dan3 等)。ロールは OS ユーザー(initdb 既定 / trust 認証=パスワード空)。
//
// node-postgres は host がスラッシュ始まりのパスだと unix ソケットに繋ぐ。
import { Pool } from "pg";

function makePool(): Pool {
  // PM2 cluster で N ワーカ起動するときは PG_POOL_MAX を絞る(N × max が
  // PostgreSQL の max_connections=100 を超えないように)。
  const max = parseInt(process.env.PG_POOL_MAX ?? "8", 10) || 8;
  const url = process.env.DATABASE_URL;
  if (url) return new Pool({ connectionString: url, max });
  return new Pool({
    host: process.env.PGHOST ?? "/tmp", // unix ソケットのディレクトリ
    user: process.env.PGUSER ?? process.env.USER,
    database: process.env.PGDATABASE ?? "lastshot",
    max,
  });
}

// next start の worker は単一プロセスだが、dev のホットリロードで再評価されても
// プールを作り直さないよう globalThis にぶら下げて使い回す。
const g = globalThis as unknown as { __lastshotPool?: Pool };
export const pool: Pool = g.__lastshotPool ?? (g.__lastshotPool = makePool());

// 現在値を返す(= lastshot の get_count と同じ SQL)。
export async function getCount(): Promise<number> {
  const r = await pool.query<{ value: number }>(
    "select value from counter where id = 1",
  );
  return r.rows[0].value;
}

// +1 して増えた後の値を返す。UPDATE ... RETURNING で 1 往復(= lastshot の increment)。
export async function increment(): Promise<number> {
  const r = await pool.query<{ value: number }>(
    "update counter set value = value + 1 where id = 1 returning value",
  );
  return r.rows[0].value;
}

// 重い一覧画面(3スタック比較)用。受注明細を N 行 generate_series で合成して読む。
// 3スタックとも同じ SQL(lastshot feature-report / laravel ReportController と一致)。
export type ReportRow = {
  id: number;
  order_no: string;
  customer: string;
  status: string;
  qty: number;
  unit_price: number;
  amount: number;
};

export async function getReport(
  rows: number,
): Promise<{ totalRows: number; totalAmount: number; rows: ReportRow[] }> {
  const r = await pool.query<ReportRow>(
    `select
       i::int4                                              as id,
       'ORD-' || lpad(i::text, 7, '0')                      as order_no,
       md5(i::text)                                         as customer,
       (case (i % 4) when 0 then 'paid' when 1 then 'pending'
                     when 2 then 'shipped' else 'cancelled' end) as status,
       (1 + i % 50)::int4                                   as qty,
       (100 + i % 900)::int4                                as unit_price,
       ((1 + i % 50) * (100 + i % 900))::int4               as amount
     from generate_series(1, $1) i
     order by i`,
    [rows],
  );
  let totalAmount = 0;
  for (const row of r.rows) totalAmount += row.amount;
  return { totalRows: r.rows.length, totalAmount, rows: r.rows };
}
