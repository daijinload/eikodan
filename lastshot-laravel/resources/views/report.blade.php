{{-- 受注明細レポート(重い一覧画面・3スタック比較用)。
     lastshot / lastshot-next と同じ画面・同じインライン CSS。N 行を @foreach で SSR する。 --}}
<!doctype html>
<html lang="ja">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>受注明細レポート — lastshot-laravel</title>
    <style>
      body { font-family: system-ui, sans-serif; margin: 0; padding: 1.5rem; background: #f3f4f6; color: #111827; }
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
    </style>
  </head>
  <body>
    <h1>受注明細レポート</h1>
    <p class="summary"><b>{{ $totalRows }}</b> 件 / 合計金額 <b>{{ $totalAmount }}</b> 円</p>
    <table>
      <thead>
        <tr>
          <th>ID</th><th>受注番号</th><th>顧客</th><th>状態</th>
          <th>数量</th><th>単価</th><th>金額</th>
        </tr>
      </thead>
      <tbody>
        @foreach ($rows as $r)
        <tr>
          <td class="num">{{ $r->id }}</td>
          <td>{{ $r->order_no }}</td>
          <td>{{ $r->customer }}</td>
          <td><span class="status {{ $r->status }}">{{ $r->status }}</span></td>
          <td class="num">{{ $r->qty }}</td>
          <td class="num">{{ $r->unit_price }}</td>
          <td class="num">{{ $r->amount }}</td>
        </tr>
        @endforeach
      </tbody>
    </table>
  </body>
</html>
