//! Postgres 接続プール。アプリ全体で 1 つ共有する薄いレイヤ。
//!
//! 方針:
//! - クエリは sqlx のランタイム API(`sqlx::query`)で書く。`query!` 系の
//!   コンパイル時検証マクロは使わない ── ビルド時に DB 接続を要求せず、
//!   fastweb 由来のビルド速度を保つため。
//! - 接続先は pg-bench の結論(unix ソケットが最速)に従い、既定でネイティブPGの
//!   unix ソケットへ繋ぐ。本番/CI は `DATABASE_URL` で TCP(compose 同一網)へ上書き。
//!
//! `sqlx` を `pub use` で再公開しているので、利用側(feature クレート)は自分の
//! Cargo.toml に sqlx を書かず `db::sqlx::query(...)` / `db::Row` で使える。

pub use sqlx;
pub use sqlx::postgres::PgPool;
pub use sqlx::Row;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

/// 接続プールを作る。
///
/// `DATABASE_URL` があればそれを使う(本番/CI の compose 用)。無ければ
/// 開発既定としてネイティブ Postgres の unix ソケット(`/tmp`)・DB 名 `PGDATABASE`
/// (既定 `lastshot` / worktree ごとに `lastshot_dan2` 等へ分けられる)・
/// ロールは OS ユーザー(initdb が作る既定ロール)へ繋ぐ。
///
/// 失敗時は理由を添えて panic する(起動時に必ず気付けるように)。
pub async fn connect() -> PgPool {
    let options = match std::env::var("DATABASE_URL") {
        Ok(url) => url
            .parse::<PgConnectOptions>()
            .expect("DATABASE_URL の形式が不正"),
        Err(_) => {
            let user = std::env::var("USER").unwrap_or_else(|_| "postgres".to_string());
            // DB 名は PGDATABASE で上書き可(worktree ごとに lastshot_dan2 等へ分けるため)。
            // 既定は lastshot。run スクリプトが worktree 名からスロットを決めて export する。
            let database = std::env::var("PGDATABASE").unwrap_or_else(|_| "lastshot".to_string());
            PgConnectOptions::new()
                .socket("/tmp") // pg-bench の結論: unix ソケットが最速
                .username(&user)
                .database(&database)
        }
    };

    PgPoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "Postgres へ接続できない: {e}\n  \
                 ヒント: `./run db-start && ./run db-setup` を実行したか / \
                 DATABASE_URL を設定したか確認"
            )
        })
}
