<?php

namespace App\Http\Controllers;

use Illuminate\Support\Facades\DB;

// counter 機能。lastshot(crates/feature-counter)と「同じ SQL」を素の DB ファサードで叩く。
// Eloquent モデルは挟まない ── counter テーブルは lastshot の Flyway が所有する 1 行
// (id=1)で、created_at 等も無い。get_count / increment と同じ往復に揃える。
class CounterController extends Controller
{
    // GET / — 現在値を SSR(lastshot の get_count と同じ select)。
    public function index()
    {
        $value = DB::selectOne('select value from counter where id = 1')->value;

        return view('counter', ['value' => $value]);
    }

    // POST /increment — +1 して増えた後の値を JSON で返す。
    // UPDATE ... RETURNING で 1 往復(= lastshot の increment と同じ)。
    public function increment()
    {
        $value = DB::selectOne('update counter set value = value + 1 where id = 1 returning value')->value;

        return response()->json(['value' => $value]);
    }
}
