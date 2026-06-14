//! counter 機能。DB に保存した値を読む/増やすサービス層 + HTMLルート + テンプレート。
//!
//! サンプルは「数字 + +1 ボタンだけ」の超シンプルなカウンター（subsecond-demo の
//! カウンターを HTMX + DB 永続化に置き換えたもの）。`get_count` / `increment` が
//! ロジックの本体（= 単一の真実）。HTML 経路はここで直接呼び、Connect API 経路
//! （`rpc` クレート）も**同じ関数**を呼ぶ。値は Postgres の `counter` テーブル（1行）に
//! 永続化するので、プロセスを再起動しても残る。

use axum::{
    extract::State,
    response::Html,
    routing::{get, post},
    Router,
};
use db::{PgPool, Row};
use schema::CounterView;
use webcore::AppState;

/// このfeatureが持つテンプレートディレクトリ（コンパイル時に絶対パス化）。
pub const TEMPLATE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/templates");

/// 現在のカウント値を返す（サービス層）。protobuf も axum も HTTP も知らない素の関数。
/// 返すのはスキーマ生成型 [`CounterView`] ── これが描画にもAPIにもそのまま流れる。
pub async fn get_count(pool: &PgPool) -> CounterView {
    let row = db::sqlx::query("select value from counter where id = 1")
        .fetch_one(pool)
        .await
        .expect("select counter value");
    CounterView {
        value: row.get::<i32, _>("value"),
        ..Default::default()
    }
}

/// 値を +1 して、増えた後の値を返す。`UPDATE ... RETURNING` で 1 往復に収める。
pub async fn increment(pool: &PgPool) -> CounterView {
    let row = db::sqlx::query("update counter set value = value + 1 where id = 1 returning value")
        .fetch_one(pool)
        .await
        .expect("increment counter value");
    CounterView {
        value: row.get::<i32, _>("value"),
        ..Default::default()
    }
}

/// このfeatureのHTMLルート群。binはこれを `.merge()` するだけ。
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/increment", post(increment_fragment))
}

/// トップ = カウンター画面（数字 + 「+1」ボタンだけ）。
/// 生成型インスタンスを1つ渡すと、描画 + 末尾への JSON 埋め込みが同時に走る。
async fn index(State(state): State<AppState>) -> Html<String> {
    let view = get_count(state.pool()).await;
    state.render_view("counter/page.html", &view)
}

/// HTMX 部分更新: +1 して数字だけを差し替えるフラグメント。
/// 同じ生成型インスタンスを `<!-- view-data ... -->` コメントでも埋め込む（デバッグ用）。
async fn increment_fragment(State(state): State<AppState>) -> Html<String> {
    let view = increment(state.pool()).await;
    state.render_view_fragment("counter/count.html", &view)
}
