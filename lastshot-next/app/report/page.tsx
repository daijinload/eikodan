import { getReport } from "../../lib/db";

// 重い一覧画面(受注明細レポート・3スタック比較用)。毎リクエスト DB を読む。
// lastshot の GET /report / laravel の GET /report と同じ画面・同じ SQL。
// N 行のテーブルを Server Component で SSR する ── React の renderToString が
// 行数ぶん効いてくるのが、Next.js の「大きい画面」での比較対象。
export const dynamic = "force-dynamic";

const DEFAULT_ROWS = 1000;
const MAX_ROWS = 100_000;

// 3スタックで見た目を揃えるためのインライン CSS(lastshot/laravel と同じ)。
const CSS = `
  h1 { font-size: 1.25rem; margin: 0 0 .25rem; }
  .summary { margin: 0 0 1rem; color: #374151; font-variant-numeric: tabular-nums; }
  .summary b { color: #111827; }
  table { border-collapse: collapse; width: 100%; background: #fff; font-size: 13px; }
  th, td { padding: 4px 8px; border-bottom: 1px solid #e5e7eb; text-align: left; white-space: nowrap; }
  th { position: sticky; top: 0; background: #1f2937; color: #fff; }
  td.num { text-align: right; font-variant-numeric: tabular-nums; }
  tr:nth-child(even) td { background: #fafafa; }
  .status { display: inline-block; padding: 1px 6px; border-radius: 4px; font-size: 11px; }
  .paid { background: #dcfce7; color: #166534; }
  .pending { background: #fef9c3; color: #854d0e; }
  .shipped { background: #dbeafe; color: #1e40af; }
  .cancelled { background: #fee2e2; color: #991b1b; }
`;

export default async function ReportPage({
  searchParams,
}: {
  searchParams: Promise<{ rows?: string }>;
}) {
  const sp = await searchParams;
  const n = Math.min(
    Math.max(parseInt(sp.rows ?? String(DEFAULT_ROWS), 10) || DEFAULT_ROWS, 0),
    MAX_ROWS,
  );
  const report = await getReport(n);

  return (
    <>
      <style dangerouslySetInnerHTML={{ __html: CSS }} />
      <h1>受注明細レポート</h1>
      <p className="summary">
        <b>{report.totalRows}</b> 件 / 合計金額 <b>{report.totalAmount}</b> 円
      </p>
      <table>
        <thead>
          <tr>
            <th>ID</th>
            <th>受注番号</th>
            <th>顧客</th>
            <th>状態</th>
            <th>数量</th>
            <th>単価</th>
            <th>金額</th>
          </tr>
        </thead>
        <tbody>
          {report.rows.map((r) => (
            <tr key={r.id}>
              <td className="num">{r.id}</td>
              <td>{r.order_no}</td>
              <td>{r.customer}</td>
              <td>
                <span className={`status ${r.status}`}>{r.status}</span>
              </td>
              <td className="num">{r.qty}</td>
              <td className="num">{r.unit_price}</td>
              <td className="num">{r.amount}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </>
  );
}
