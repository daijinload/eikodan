//! スキーマ（.proto）から生成した buffa 型。アプリ全体の「単一の真実」。
//!
//! HTML テンプレート(serde 経由) ・ 末尾の埋め込みJSON ・ Connect API が、
//! この同じ生成型を共有する。型を変えたいときは proto を変える ── それだけで
//! 3経路すべてに反映される。
//!
//! 注意: json feature の serde 実装は **proto3 JSON 準拠の camelCase**。
//! つまりテンプレ側も埋め込みJSONも camelCase で参照する
//! （例: `recent_activities` → `view.recentActivities`）。

// build.rs(connectrpc-build)が $OUT_DIR に生成したコードを取り込む。
pub mod proto {
    connectrpc::include_generated!();
}

// よく使う型はクレート直下に再エクスポートしておく。
pub use proto::user::v1::{Activity, GetUserRequest, UserPageView};
