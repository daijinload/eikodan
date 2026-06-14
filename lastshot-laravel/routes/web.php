<?php

use App\Http\Controllers\CounterController;
use App\Http\Controllers\ReportController;
use Illuminate\Support\Facades\Route;

// DB保存カウンター(lastshot / lastshot-next と同じ画面・同じ DB)。
//   GET  /          現在値を SSR(Blade)
//   POST /increment +1 して JSON で返す(DB書込往復 / CSRF 対象外)
Route::get('/', [CounterController::class, 'index']);
Route::post('/increment', [CounterController::class, 'increment']);

// 重い一覧画面(3スタック比較)。受注明細を N 行 SSR(?rows=N)。
Route::get('/report', [ReportController::class, 'index']);
