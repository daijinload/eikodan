//! 起動済みサーバーをHTTPで外から叩くブラックボックステスト。
//!
//! 設計（fastweb 由来）:
//! - アプリ本体をリンクしない → アプリを変更してもこのクレートは再ビルド不要。
//! - 起動は「同梱」ではなく「待機」で解く: 先に `cargo run -p app` で起動しておき、
//!   テストはヘルスチェックをリトライしてから叩く（bacon再起動レースに強い）。
//!   DB が要るので `./run db-setup` 済みのサーバを起動しておくこと。
//!
//! lastshot 固有の検証（DB保存カウンター）:
//! - トップHTMLにカウンター要素と「同じインスタンスの埋め込みJSON」(view-data)があること。
//! - Connect API の Increment が「現在値 + 1」を返すこと（= DB が効いている）。

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
    panic!("server not reachable at {base} — 先に `./run db-setup && ./run dev` で起動しておくこと");
}

#[tokio::test]
async fn home_renders_counter_with_embedded_view() {
    let base = base_url();
    let client = reqwest::Client::new();
    wait_until_up(&client, &base).await;

    let body = client
        .get(&base)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    // カウンター画面の要素（HTMX で中身を差し替える対象）
    assert!(body.contains("id=\"count\""), "page should render the counter element");
    assert!(body.contains("hx-post"), "page should contain an HTMX attribute");
    // 末尾に埋め込まれた「この画面が使ったデータ」（デバッグ用コメント）。
    // proto3 JSON は 0 値フィールドを省くことがあるので、キー名ではなくコメントの存在で確認する。
    assert!(
        body.contains("<!-- view-data"),
        "page should embed the source view JSON as a debug comment"
    );
}

#[tokio::test]
async fn connect_increment_advances_value() {
    let base = base_url();
    let client = reqwest::Client::new();
    wait_until_up(&client, &base).await;

    // GetCount → 現在値。Increment → +1 後の値。HTML と同じサービス層・同じ DB を共有する。
    let before: serde_json::Value = client
        .post(format!("{base}/counter.v1.CounterService/GetCount"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let after: serde_json::Value = client
        .post(format!("{base}/counter.v1.CounterService/Increment"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // proto3 JSON では value=0 が省略され得るので、欠落は 0 とみなす。
    let before_v = before["value"].as_i64().unwrap_or(0);
    let after_v = after["value"].as_i64().unwrap_or(0);
    assert_eq!(
        after_v,
        before_v + 1,
        "Increment は現在値 + 1 を返すはず（DB に永続化されている証拠）"
    );
}
