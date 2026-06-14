//! ベンチ用エンドポイント群。Rust(lastshot) vs Node.js の API 速度比較のためだけに
//! 存在する薄い feature。DBの「重さ」を軸にスライドさせて、言語/ランタイム差がどこで
//! 効くか（=「DB律速だから言語差は出ない」がどこまで本当か）を測る。
//!
//! Node 側（`../../lastshot-node/server.mjs`）と **同一のクエリ・同一のレスポンス JSON**
//! を返すこと。片方を変えたら必ず両方直す（パリティが崩れると比較が無意味になる）。
//!
//! - `GET /ping`        … DBなし。ランタイム+HTTP+JSONの素の天井。
//! - `GET /db/light`    … 点取得 1往復（counter の値）。実APIの大半が住む現実ケース。
//! - `GET /db/heavy`    … bench_rows 全走査の集約。DB CPU を使い切る = DB律速領域。
//! - `GET /db/sleep`    … pg_sleep。PG CPU ほぼ0の純待ち（ロック/IO待ちの模擬）。

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use db::Row;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::OnceCell;
use tokio_postgres::{Client, NoTls, Statement};
use webcore::AppState;

/// このベンチ feature のルート群。app はこれを `.merge()` するだけ。
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/ping", get(ping))
        .route("/db/light", get(db_light))
        // sqlx 版(/db/light)とSQL/レスポンス同一で、ドライバだけ tokio-postgres の
        // パイプライン版。Node(postgres.js)のパイプラインに対抗できるか測るための比較対象。
        .route("/db/light_pipe", get(db_light_pipe))
        .route("/db/heavy", get(db_heavy))
        .route("/db/sleep", get(db_sleep))
}

/// DBに一切触らない。ランタイム+HTTP+JSON シリアライズの素の天井を測る。
async fn ping() -> Json<Value> {
    Json(json!({ "ok": true }))
}

/// 点取得 1往復。counter テーブルの 1 行を引くだけ（feature_counter::get_count と同じSQL）。
async fn db_light(State(state): State<AppState>) -> Json<Value> {
    let row = db::sqlx::query("select value from counter where id = 1")
        .fetch_one(state.pool())
        .await
        .expect("select counter value");
    Json(json!({ "value": row.get::<i32, _>("value") }))
}

/// bench_rows を全走査して集約する重いクエリ。`like '%abc%'` で全行に LIKE を当てるので
/// 行数ぶんの PG CPU を使う（= DB律速領域を再現）。count は ::int に寄せて Node と型を揃える。
async fn db_heavy(State(state): State<AppState>) -> Json<Value> {
    let row = db::sqlx::query(
        "select count(*)::int as count, coalesce(avg(n)::float8, 0.0) as avg \
         from bench_rows where s like '%abc%'",
    )
    .fetch_one(state.pool())
    .await
    .expect("heavy aggregate");
    Json(json!({
        "count": row.get::<i32, _>("count"),
        "avg": row.get::<f64, _>("avg"),
    }))
}

#[derive(Deserialize)]
struct SleepQuery {
    ms: Option<u64>,
}

/// pg_sleep で「DBが遅い（待つ）が CPU はほぼ使わない」状況を作る。
/// 接続を ms ぶん占有するので、多数の待ち接続をランタイムがどう捌くかが出る。
async fn db_sleep(State(state): State<AppState>, Query(q): Query<SleepQuery>) -> Json<Value> {
    let ms = q.ms.unwrap_or(20);
    db::sqlx::query("select pg_sleep($1::float8 / 1000.0)")
        .bind(ms as i64)
        .execute(state.pool())
        .await
        .expect("pg_sleep");
    Json(json!({ "slept": ms }))
}

// ───────────────────────────────────────────────────────────
// パイプライン版 /db/light_pipe（ベンチ専用の生 tokio-postgres プール）
//
// なぜ sqlx と別に持つか:
//   sqlx は「1接続=1クエリ（往復ごとに待つ）」なので、多コア×点SELECTでは
//   Node(postgres.js のパイプライン)に届かない（実測 ~77k 止まり）。
//   tokio-postgres は「同一接続に複数クエリを並行に流す＝パイプライン」を
//   futures の並行 poll で自動的に行う（docs: 並行 poll で自動パイプライン、
//   ただしサーバ側の実行は接続内で直列）。
//
// 仕組み:
//   - 起動後の初回アクセス時に Client を PIPE_CLIENTS 本（既定 POOL_MAX）張る。
//     各 Client = 1 接続 = PG の 1 バックエンド。Arc で共有し、リクエストごとに
//     ラウンドロビンで割り当てる。同一 Client に並行リクエストが乗ると、その
//     接続上でクエリがパイプラインされる（C 同時 / N 接続 ≒ 接続あたり C/N 深さ）。
//   - プリペアドステートメントは接続ごとに1回だけ用意（postgres.js と条件を揃える）。
// ───────────────────────────────────────────────────────────

struct PgPipe {
    /// 共有する接続群（各々が独立に並行クエリをパイプラインする）。
    clients: Vec<Arc<Client>>,
    /// 接続ごとに用意した prepared statement（clients と同じ並び）。
    stmts: Vec<Statement>,
    /// ラウンドロビン用カウンタ。
    next: AtomicUsize,
}

static PIPE: OnceCell<PgPipe> = OnceCell::const_new();

/// 初回アクセス時に接続群を張って共有する（OnceCell なので初期化は1回だけ）。
async fn pipe() -> &'static PgPipe {
    PIPE.get_or_init(|| async {
        let user = std::env::var("USER").unwrap_or_else(|_| "postgres".to_string());
        let database = std::env::var("PGDATABASE").unwrap_or_else(|_| "lastshot".to_string());
        // 接続本数。既定は POOL_MAX（Node/sqlx と DB 接続予算を揃える）。
        // PIPE_CLIENTS で別途上書き可（本数を減らすと接続あたりのパイプライン深さが増える）。
        let n = std::env::var("PIPE_CLIENTS")
            .or_else(|_| std::env::var("POOL_MAX"))
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(8)
            .max(1);

        let mut clients = Vec::with_capacity(n);
        let mut stmts = Vec::with_capacity(n);
        for _ in 0..n {
            let (client, connection) = tokio_postgres::Config::new()
                .host_path("/tmp") // sqlx の .socket("/tmp") と同じ unix ソケット
                .port(5432)
                .user(&user)
                .dbname(&database)
                .connect(NoTls)
                .await
                .expect("tokio-postgres connect (pipe)");
            // 接続の駆動タスク。これが回ることでパイプライン処理が進む。
            tokio::spawn(async move {
                let _ = connection.await;
            });
            let stmt = client
                .prepare("select value from counter where id = 1")
                .await
                .expect("prepare light_pipe");
            clients.push(Arc::new(client));
            stmts.push(stmt);
        }
        PgPipe {
            clients,
            stmts,
            next: AtomicUsize::new(0),
        }
    })
    .await
}

/// /db/light と同一の点取得を、パイプライン可能な共有接続で実行する。
async fn db_light_pipe() -> Json<Value> {
    let p = pipe().await;
    let i = p.next.fetch_add(1, Ordering::Relaxed) % p.clients.len();
    let row = p.clients[i]
        .query_one(&p.stmts[i], &[])
        .await
        .expect("light_pipe query");
    let value: i32 = row.get(0);
    Json(json!({ "value": value }))
}
