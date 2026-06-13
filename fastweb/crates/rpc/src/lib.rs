//! Connect RPC レイヤー。proto から生成した型付きスキーマでハンドラを書き、
//! `connect_service()` を axum にマウントできる形（`ConnectRpcService`）で返す。

use connectrpc::{handler_fn, ConnectRpcService, RequestContext, Response, Router as ConnectRouter};

// build.rs(connectrpc-build)が $OUT_DIR に生成したコードを取り込む。
pub mod proto {
    connectrpc::include_generated!();
}

use proto::greet::v1::{GreetRequest, GreetResponse};

/// Connect RPC サービスを構築して axum 用サービスに変換する。
/// app 側は `.fallback_service(rpc::connect_service())` でマウントするだけ。
pub fn connect_service() -> ConnectRpcService {
    ConnectRouter::new()
        .route(
            "greet.v1.GreetService",
            "Greet",
            handler_fn(|_ctx: RequestContext, req: GreetRequest| async move {
                Response::ok(GreetResponse {
                    greeting: format!("Hello, {}!", req.name),
                    ..Default::default()
                })
            }),
        )
        .into_axum_service()
}
