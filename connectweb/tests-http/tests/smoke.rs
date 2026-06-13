//! 起動済みサーバーをHTTPで外から叩くブラックボックステスト。
//!
//! 設計（fastweb 由来）:
//! - アプリ本体をリンクしない → アプリを変更してもこのクレートは再ビルド不要。
//! - 起動は「同梱」ではなく「待機」で解く: 先に `cargo run -p app` で起動しておき、
//!   テストはヘルスチェックをリトライしてから叩く（bacon再起動レースに強い）。
//!
//! connectweb 固有の検証:
//! - HTMLページに「同じインスタンスの埋め込みJSON」(view-data) が入っていること。
//! - 同じ型を返す Connect API が、HTML と同じ値（camelCase proto3 JSON）を返すこと。

use std::time::Duration;

fn base_url() -> String {
    std::env::var("BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string())
}

/// サーバーが応答するまでヘルスチェックをリトライする。
async fn wait_until_up(client: &reqwest::Client, base: &str) {
    for _ in 0..50 {
        if let Ok(resp) = client.get(base).send().await {
            if resp.status().is_success() {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("server not reachable at {base} — 先に `cargo run -p app` で起動しておくこと");
}

#[tokio::test]
async fn html_page_renders_with_embedded_view_json() {
    let base = base_url();
    let client = reqwest::Client::new();
    wait_until_up(&client, &base).await;

    let body = client
        .get(format!("{base}/users/7"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    // 描画された画面の値
    assert!(body.contains("ユーザー7"), "page should render the user name");
    assert!(body.contains("hx-get"), "page should contain an HTMX attribute");
    // 末尾に埋め込まれた「この画面が使ったデータ」（デバッグ用コメント）
    assert!(
        body.contains("<!-- view-data"),
        "page should embed the source view JSON as a debug comment"
    );
    // 埋め込みJSONは proto3 JSON の camelCase
    assert!(
        body.contains("recentActivities"),
        "embedded JSON should use proto3 camelCase keys"
    );
}

#[tokio::test]
async fn connect_get_user_returns_same_view() {
    let base = base_url();
    let client = reqwest::Client::new();
    wait_until_up(&client, &base).await;

    let resp: serde_json::Value = client
        .post(format!("{base}/user.v1.UserService/GetUser"))
        .json(&serde_json::json!({ "id": 7 }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // HTML 経路と同じ get_user・同じ生成型なので、値も一致する。
    assert_eq!(resp["name"], "ユーザー7");
    assert_eq!(resp["email"], "user7@example.com");
    assert!(
        resp["recentActivities"].is_array(),
        "API should return the same camelCase view shape as the HTML embed"
    );
}
