//! スキーマ（.proto）から生成した buffa 型。アプリ全体の「単一の真実」。
//!
//! HTML テンプレート(serde 経由) ・ 末尾の埋め込みJSON ・ Connect API が、
//! この同じ生成型を共有する。型を変えたいときは proto を変える ── それだけで
//! 3経路すべてに反映される。
//!
//! 注意: json feature の serde 実装は **proto3 JSON 準拠の camelCase**。
//! 単語フィールド(`value`)はそのままだが、複合語は camelCase で参照する。

// build.rs(connectrpc-build)が $OUT_DIR に生成したコードを取り込む。
pub mod proto {
    connectrpc::include_generated!();
}

// よく使う型はクレート直下に再エクスポートしておく。
pub use proto::counter::v1::{CounterView, GetCountRequest, IncrementRequest};
// 重い一覧画面(3スタック比較)用のビュー型。
pub use proto::report::v1::{ReportRow, ReportView};
