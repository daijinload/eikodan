//! 起動済みサーバーをHTTPで外から叩くブラックボックステスト。
//!
//! 設計（議論の結論）:
//! - アプリ本体をリンクしない → アプリを変更してもこのクレートは再ビルド不要。
//! - 起動は「同梱」ではなく「待機」で解く: 先に `cargo run -p app` で起動しておき、
//!   テストはヘルスチェックをリトライしてから叩く（bacon再起動レースに強い）。
//! - HTMXのレスポンスは部分HTMLなので、型共有なしの緩いHTML検証で十分。

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
async fn html_index_renders() {
    let base = base_url();
    let client = reqwest::Client::new();
    wait_until_up(&client, &base).await;

    let body = client
        .get(base.as_str())
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(body.contains("fastweb"), "index should contain app title");
    assert!(body.contains("hx-get"), "index should contain an HTMX attribute");
}

#[tokio::test]
async fn connect_greet_returns_greeting() {
    let base = base_url();
    let client = reqwest::Client::new();
    wait_until_up(&client, &base).await;

    let resp: serde_json::Value = client
        .post(format!("{base}/greet.v1.GreetService/Greet"))
        .json(&serde_json::json!({ "name": "tester" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["greeting"], "Hello, tester!");
}
