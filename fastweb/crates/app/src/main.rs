//! 起動バイナリ。ルーター組み立て・ソケット引き継ぎ起動・開発用ライブリロードのみ。

use std::path::PathBuf;

use axum::{response::IntoResponse, routing::get, Router};
use webcore::AppState;

/// CLI生成のCSSを `/static/app.css` で配信する。テンプレ同様ディスクから読む
/// （tailwind --watch の出力を即反映。納品時の埋め込みはテンプレと足並みを揃えて将来対応）。
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
            // 本番のキャッシュ方針はテンプレ埋め込み対応時にまとめて決める。
            #[cfg(debug_assertions)]
            headers.insert(
                axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-cache"),
            );
            (headers, bytes).into_response()
        }
        Err(_) => (
            axum::http::StatusCode::NOT_FOUND,
            "app.css 未生成: `bash assets/setup-css.sh` 後に tailwind --watch を回してください",
        )
            .into_response(),
    }
}

/// MiniJinjaのテンプレートルート: appのshell + 各featureの templates/。
/// featureを増やしたらここに1行足す（テンプレ名は feature名/部品名.html 規約）。
fn template_dirs() -> Vec<PathBuf> {
    vec![
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/templates")),
        PathBuf::from(feature_hello::TEMPLATE_DIR),
    ]
}

fn build_router() -> Router {
    let state = AppState::new(template_dirs());
    Router::new()
        .route("/static/app.css", get(app_css))
        .merge(feature_hello::routes())
        // Connect RPC: 未マッチのパス（例 POST /greet.v1.GreetService/Greet）を
        // Connectサービスへ流す。HTML(HTMX)とRPCを1プロセス・1ポートで同居させる。
        .fallback_service(rpc::connect_service())
        .with_state(state)
}

#[tokio::main]
async fn main() {
    let router = with_live_reload(build_router());

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
            tokio::net::TcpListener::bind(("127.0.0.1", port))
                .await
                .unwrap()
        }
    };
    println!("listening on http://{}", listener.local_addr().unwrap());
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
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if res.is_ok() {
                    let _ = tx.send(());
                }
            })
            .expect("create file watcher");
        // テンプレHTML + CLI生成CSSの出力先を監視。
        // テンプレ保存 → tailwind --watch が app.css を書き直す → そのCSS変更で再読込（最新CSSで反映）。
        let mut watch_dirs = template_dirs();
        watch_dirs.push(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/static")));
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
