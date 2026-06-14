<?php

namespace App\Http\Controllers;

use Illuminate\Http\Request;
use Illuminate\Support\Facades\DB;

// 重い一覧画面(3スタック比較)。受注明細を N 行 generate_series で合成して Blade で SSR する。
// lastshot(feature-report) / lastshot-next(getReport) と「同じ SQL・同じ画面」。
// counter と同じく Eloquent は挟まず素の DB ファサードで叩く(合成データなのでモデルも無い)。
class ReportController extends Controller
{
    // GET /report?rows=N — N 行の明細テーブルを SSR。
    public function index(Request $request)
    {
        $rows = (int) $request->query('rows', 1000);
        $rows = max(0, min(100000, $rows));

        $data = DB::select(
            "select
               i::int4                                              as id,
               'ORD-' || lpad(i::text, 7, '0')                      as order_no,
               md5(i::text)                                         as customer,
               (case (i % 4) when 0 then 'paid' when 1 then 'pending'
                             when 2 then 'shipped' else 'cancelled' end) as status,
               (1 + i % 50)::int4                                   as qty,
               (100 + i % 900)::int4                                as unit_price,
               ((1 + i % 50) * (100 + i % 900))::int4               as amount
             from generate_series(1, ?) i
             order by i",
            [$rows]
        );

        $totalAmount = 0;
        foreach ($data as $r) {
            $totalAmount += (int) $r->amount;
        }

        return view('report', [
            'rows' => $data,
            'totalRows' => count($data),
            'totalAmount' => $totalAmount,
        ]);
    }
}
