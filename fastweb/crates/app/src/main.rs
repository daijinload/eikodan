//! 起動バイナリ。ルーター組み立て・ソケット引き継ぎ起動・開発用ライブリロードのみ。

use std::path::PathBuf;

use axum::Router;
use webcore::AppState;

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
        None => tokio::net::TcpListener::bind("127.0.0.1:3000")
            .await
            .unwrap(),
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
        for dir in template_dirs() {
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
