//! user 機能。サービス層関数 + HTMLルート + テンプレートをこのクレートに閉じる。
//!
//! `get_user` がロジックの本体（= 単一の真実）。HTML 経路はここで直接呼び、
//! Connect API 経路（`rpc` クレート）も**同じ関数**を呼ぶ。自分自身への gRPC
//! ループバックも protobuf シリアライズも経路上に存在しない。

use axum::extract::{Path, State};
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use schema::{Activity, UserPageView};
use webcore::AppState;

/// このfeatureが持つテンプレートディレクトリ（コンパイル時に絶対パス化）。
pub const TEMPLATE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/templates");

/// サービス層: ロジックの本体。protobuf も axum も HTTP も知らない素の関数。
/// 返すのはスキーマ生成型 [`UserPageView`] ── これが描画にもAPIにもそのまま流れる。
///
/// 本サンプルは DB の代わりに id から決定的なダミーを返す。
pub async fn get_user(id: u32) -> UserPageView {
    UserPageView {
        id,
        name: format!("ユーザー{id}"),
        email: format!("user{id}@example.com"),
        role: "member".into(),
        recent_activities: vec![
            Activity {
                action: "ログイン".into(),
                at: "2026-06-13T09:00:00Z".into(),
                ..Default::default()
            },
            Activity {
                action: "プロフィール更新".into(),
                at: "2026-06-13T09:05:00Z".into(),
                ..Default::default()
            },
            Activity {
                action: "記事を投稿".into(),
                at: "2026-06-13T10:30:00Z".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}

/// このfeatureのHTMLルート群。binはこれを `.merge()` するだけ。
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/users/{id}", get(user_page))
        .route("/users/{id}/activities", get(activities_fragment))
}

/// トップは id=1 のユーザー画面を出すデモ。
async fn index(State(state): State<AppState>) -> Html<String> {
    let view = get_user(1).await;
    state.render_view("user/page.html", &view)
}

async fn user_page(State(state): State<AppState>, Path(id): Path<u32>) -> Html<String> {
    let view = get_user(id).await;
    // 生成型インスタンスを1つ渡すと、描画 + 末尾への JSON 埋め込みが同時に走る。
    state.render_view("user/page.html", &view)
}

/// HTMX 部分更新: アクティビティ一覧だけを差し替えるフラグメント。
/// 部分HTMLだが、デバッグ時にデータを見たいので、フルページと同じく `<!-- view-data ... -->`
/// コメント形式で同じインスタンスを埋め込む（断片なので先頭に付く）。
async fn activities_fragment(State(state): State<AppState>, Path(id): Path<u32>) -> Html<String> {
    let view = get_user(id).await;
    state.render_view_fragment("user/activities.html", &view)
}
