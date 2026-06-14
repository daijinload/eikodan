//! report 機能。3スタック比較の「重い一覧画面」（受注明細レポート）。
//!
//! 業務でありがちな「ページングなしの明細グリッド」を模した、N 行のテーブルを 1 リクエストで
//! SSR する画面。行数を増やすほど「1画面あたりのサーバ描画コスト」が効いてきて、同じ DB・
//! 同じ SQL でもアプリ層（言語ランタイム × アーキテクチャ）の差が大きく開く ── を見せる。
//!
//! データは **マイグレーションせず** Postgres の `generate_series` で合成する（3スタック同一 SQL）。
//! counter と違い view-data 埋め込みは外す（数千行の JSON 再直列化は非現実的に重く、比較では
//! 「描画そのもの」を測りたい）。`render_view_plain` を使う。

use axum::{extract::Query, extract::State, response::Html, routing::get, Router};
use db::{PgPool, Row};
use schema::{ReportRow, ReportView};
use std::collections::HashMap;
use webcore::AppState;

/// このfeatureが持つテンプレートディレクトリ（コンパイル時に絶対パス化）。
pub const TEMPLATE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/templates");

/// 既定の行数（`?rows=N` で上書き）。業務の「ちょっと重い一覧」相当。
const DEFAULT_ROWS: i32 = 1000;
/// 行数の上限（暴走防止）。
const MAX_ROWS: i32 = 100_000;

/// N 行の受注明細を生成して読む（サービス層）。3スタックとも同じ SQL。
/// `generate_series(1, $1)` で行を合成し、各列を計算する。0 値が出ないよう
/// 値域を作ってあるので proto3 JSON の 0 値省略に引っかからない。
pub async fn get_report(pool: &PgPool, rows: i32) -> ReportView {
    let recs = db::sqlx::query(
        "select \
           i::int4                                              as id, \
           'ORD-' || lpad(i::text, 7, '0')                      as order_no, \
           md5(i::text)                                         as customer, \
           (case (i % 4) when 0 then 'paid' when 1 then 'pending' \
                         when 2 then 'shipped' else 'cancelled' end) as status, \
           (1 + i % 50)::int4                                   as qty, \
           (100 + i % 900)::int4                                as unit_price, \
           ((1 + i % 50) * (100 + i % 900))::int4               as amount \
         from generate_series(1, $1) i \
         order by i",
    )
    .bind(rows)
    .fetch_all(pool)
    .await
    .expect("select report rows");

    let mut out = Vec::with_capacity(recs.len());
    let mut total_amount: i64 = 0;
    for r in &recs {
        let amount: i32 = r.get("amount");
        total_amount += amount as i64;
        out.push(ReportRow {
            id: r.get::<i32, _>("id"),
            order_no: r.get::<String, _>("order_no"),
            customer: r.get::<String, _>("customer"),
            status: r.get::<String, _>("status"),
            qty: r.get::<i32, _>("qty"),
            unit_price: r.get::<i32, _>("unit_price"),
            amount,
            ..Default::default()
        });
    }

    ReportView {
        total_rows: out.len() as i32,
        total_amount,
        rows: out,
        ..Default::default()
    }
}

/// このfeatureのHTMLルート群。binはこれを `.merge()` するだけ。
pub fn routes() -> Router<AppState> {
    Router::new().route("/report", get(index))
}

/// GET /report?rows=N — N 行の明細テーブルを SSR（埋め込みJSONなし）。
async fn index(State(state): State<AppState>, Query(q): Query<HashMap<String, String>>) -> Html<String> {
    let rows = q
        .get("rows")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(DEFAULT_ROWS)
        .clamp(0, MAX_ROWS);
    let view = get_report(state.pool(), rows).await;
    state.render_view_plain("report/page.html", &view)
}
