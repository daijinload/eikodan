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
//! - HTMX フラグメント経路 `POST /increment` が、フルページでなく count 断片(+view-data)を返し、
//!   値が単調増加すること（= ブラウザを動かさずに HTMX のサーバ側出力を直接検証）。
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

/// HTMX のボタン（`hx-post="/increment" hx-target="#count" hx-swap="innerHTML"`）が叩く経路を、
/// ブラウザを動かさずに HTTP で直接検証する。HTMX エンドポイントは「HTML 断片を返すただの POST」
/// なので、返ってきた断片の不変条件を assert すれば API レベルでテストできる。
#[tokio::test]
async fn htmx_increment_returns_growing_fragment() {
    let base = base_url();
    let client = reqwest::Client::new();
    wait_until_up(&client, &base).await;

    // 2 回叩く。2 回目は 1 回目より大きい＝毎回 DB に永続化され増えている（単調増加）。
    // nextest は各テストを並列実行するので、他テストの increment が間に挟まり得る。
    // よって「厳密に +1」ではなく「増えている」を見る（increment しか無いので単調）。
    let first = post_increment_fragment(&client, &base).await;
    let second = post_increment_fragment(&client, &base).await;
    assert!(
        second > first,
        "HTMX フラグメントの値は増え続けるはず（DB 永続のカウンター）: first={first}, second={second}"
    );
}

/// `POST /increment` を HTMX として叩き、返ってきたフラグメントの不変条件を検証して
/// 可視カウント値（i64）を返す。
async fn post_increment_fragment(client: &reqwest::Client, base: &str) -> i64 {
    let resp = client
        .post(format!("{base}/increment"))
        .header("HX-Request", "true") // 本物の HTMX が付けるリクエストヘッダを再現
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success(), "POST /increment should succeed");
    let frag = resp.text().await.unwrap();

    // フルページではなくフラグメント（#count の innerHTML に差し込む断片）であること。
    let lower = frag.to_ascii_lowercase();
    assert!(!lower.contains("<html"), "should be a fragment, not a full page");
    assert!(!lower.contains("<!doctype"), "should be a fragment, not a full page");

    // 描画に使った同じインスタンスを覗くデバッグ窓（render_view_fragment が先頭に付ける）。
    assert!(
        frag.contains("<!-- view-data"),
        "fragment should embed the source view JSON as a debug comment"
    );

    // view-data コメント（先頭）を除いた可視部分がカウント値そのもの（count.html = `{{ view.value }}`）。
    let visible = frag.rsplit("-->").next().unwrap_or(&frag).trim();
    visible
        .parse()
        .unwrap_or_else(|_| panic!("fragment body should be the counter value, got: {visible:?}"))
}
