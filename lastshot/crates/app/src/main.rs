//! 起動バイナリ。ルーター組み立て・ソケット引き継ぎ起動・開発用ライブリロードのみ。
//!
//! HTML(HTMX) と Connect API を 1プロセス・1ポートで同居させる:
//! - 既知のパス（`/`, `/users/{id}` …）は feature の HTML ハンドラ。
//! - 未マッチのパス（例 `POST /user.v1.UserService/GetUser`）は Connect サービスへ。

use std::path::PathBuf;

use axum::{response::IntoResponse, routing::get, Router};
use db::PgPool;
use webcore::AppState;

/// CLI生成のCSSを `/static/app.css` で配信する（builtモード／release時のみ参照される）。
/// テンプレ同様ディスクから直読みするので、クリーンビルドの出力を即反映できる。
const CSS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/static/app.css");

async fn app_css() -> impl IntoResponse {
    match tokio::fs::read(CSS_PATH).await {
        Ok(bytes) => {
            let mut headers = axum::http::HeaderMap::new();
            headers.insert(
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("text/css; charset=utf-8"),
            );
            // debugビルドのみ no-cache: ライブリロードのたびに必ず取り直させ stale CSS を防ぐ。
            #[cfg(debug_assertions)]
            headers.insert(
                axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-cache"),
            );
            (headers, bytes).into_response()
        }
        Err(_) => (
            axum::http::StatusCode::NOT_FOUND,
            "app.css 未生成: builtモードには生成が要る。`bash assets/setup-css.sh` 後に `bash assets/check-css.sh`（日常はCDNなので CSS=built を外せば不要）",
        )
            .into_response(),
    }
}

/// MiniJinjaのテンプレートルート: appのshell + 各featureの templates/。
/// featureを増やしたらここに1行足す（テンプレ名は feature名/部品名.html 規約）。
fn template_dirs() -> Vec<PathBuf> {
    vec![
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/templates")),
        PathBuf::from(feature_counter::TEMPLATE_DIR),
        PathBuf::from(feature_report::TEMPLATE_DIR),
    ]
}

/// CSSの配信モードを実行時に決める（コンパイルプロファイルには紐付けない）。
/// - release は常にCLI生成CSS（本番＝パージ済み）。
/// - debug は既定でCDN（ビルド/watch不要の軽量開発）。`CSS=built` で最終確認モードに切替。
///   env変数の実行時読みなので debug↔最終確認の往復で再ビルドは走らない。
fn css_built() -> bool {
    cfg!(not(debug_assertions)) || matches!(std::env::var("CSS").as_deref(), Ok("built"))
}

fn build_router(pool: PgPool) -> Router {
    let state = AppState::new(template_dirs(), pool.clone(), css_built());
    Router::new()
        .route("/static/app.css", get(app_css))
        .merge(feature_counter::routes())
        .merge(feature_report::routes())
        // Connect API: 未マッチのパスを Connect サービスへ流す。
        // HTML と同じポートに同居（同じサービス層関数を裏で共有する）。
        .fallback_service(rpc::connect_service(pool))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    // 起動時に Postgres プールを作る（接続できなければここで panic して気付ける）。
    let pool = db::connect().await;
    let router = with_live_reload(build_router(pool));

    // listenfd: systemfd経由ならリッスンソケットを引き継ぎ、再起動でも接続が切れない。
    let mut listenfd = listenfd::ListenFd::from_env();
    let listener = match listenfd.take_tcp_listener(0).unwrap() {
        Some(std) => {
            std.set_nonblocking(true).unwrap();
            tokio::net::TcpListener::from_std(std).unwrap()
        }
        None => {
            // 既定 3000。別worktreeを並列起動する等で衝突するときは PORT で上書き（例: PORT=3001）。
            let port = std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(3000);
            // 既定 127.0.0.1（dev は外部公開しない）。コンテナ（compose/CI）では HOST=0.0.0.0 を
            // 渡して公開ポート/他サービスから到達できるようにする（127.0.0.1 だとコンテナ内 loopback
            // 止まりで、ポートフォワードされた外部トラフィックが届かない）。
            let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
            tokio::net::TcpListener::bind(format!("{host}:{port}").as_str())
                .await
                .unwrap()
        }
    };
    let css_mode = if css_built() {
        "built (/static/app.css)"
    } else {
        "cdn"
    };
    println!(
        "listening on http://{}  [css: {css_mode}]",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, router).await.unwrap();
}

// HTMXの部分更新(hx-request)ではリロードを発火させない。
#[cfg(debug_assertions)]
fn not_htmx_predicate<T>(req: &axum::http::Request<T>) -> bool {
    !req.headers().contains_key("hx-request")
}

/// debugビルド: テンプレートディレクトリを監視し、変更でブラウザを自動リロードする。
#[cfg(debug_assertions)]
fn with_live_reload(router: Router) -> Router {
    use notify::{RecursiveMode, Watcher};
    use tower_livereload::LiveReloadLayer;

    let livereload = LiveReloadLayer::new();
    let reloader = livereload.reloader();

    tokio::spawn(async move {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if res.is_ok() {
                let _ = tx.send(());
            }
        })
        .expect("create file watcher");
        // テンプレHTMLを常に監視（保存→自動リロード）。
        // builtモードのときだけCLI生成CSSの出力先 static/ も監視する
        // （tailwind --watch が app.css を書き直す → そのCSS変更で再読込）。CDN既定では static/ は不要。
        let mut watch_dirs = template_dirs();
        if css_built() {
            watch_dirs.push(PathBuf::from(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/static"
            )));
        }
        for dir in watch_dirs {
            if let Err(e) = watcher.watch(&dir, RecursiveMode::Recursive) {
                eprintln!("watch {dir:?} failed: {e}");
            }
        }
        while rx.recv().await.is_some() {
            reloader.reload();
        }
        drop(watcher);
    });

    router.layer(livereload.request_predicate(not_htmx_predicate))
}

#[cfg(not(debug_assertions))]
fn with_live_reload(router: Router) -> Router {
    router
}
