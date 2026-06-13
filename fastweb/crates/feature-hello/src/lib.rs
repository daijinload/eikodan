//! サンプルfeature。1機能 = ハンドラ + テンプレート + (テスト) をこのフォルダに閉じる。
//!
//! AIや人間が「この機能を直して」と言われたとき、読むべき文脈がこの1クレートに収まる。

use axum::extract::State;
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use minijinja::context;
use webcore::AppState;

/// このfeatureが持つテンプレートディレクトリ（コンパイル時に絶対パス化）。
/// appがこれを集めてMiniJinjaのローダールートにする。
pub const TEMPLATE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/templates");

/// このfeatureのルート群。binはこれを `.merge()` するだけ。
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/clicked", get(clicked))
}

async fn index(State(state): State<AppState>) -> Html<String> {
    state.render("hello/index.html", context! { title => "fastweb" })
}

/// HTMXの部分HTML差し替えデモ。
async fn clicked() -> Html<String> {
    Html(r#"<span class="badge badge-success badge-lg">clicked! ✨</span>"#.to_string())
}
