//! Connect API レイヤー。proto で定義した `user.v1.UserService/GetUser` を
//! axum にマウントできる形（`ConnectRpcService`）で公開する。
//!
//! 肝は、ハンドラが feature 側のサービス層関数 [`feature_user::get_user`] を
//! そのまま呼ぶこと。HTML 経路とまったく同じ関数・同じ生成型を使うので、
//! 「API を叩いた値」と「画面が使った値」が同じロジックから生まれる。
//! 同一プロセス内なので自分への通信もシリアライズも発生しない。

use connectrpc::{handler_fn, ConnectRpcService, RequestContext, Response, Router as ConnectRouter};
use schema::GetUserRequest;

/// Connect RPC サービスを構築して axum 用サービスに変換する。
/// app 側は `.fallback_service(rpc::connect_service())` でマウントするだけ。
pub fn connect_service() -> ConnectRpcService {
    ConnectRouter::new()
        .route(
            "user.v1.UserService",
            "GetUser",
            handler_fn(|_ctx: RequestContext, req: GetUserRequest| async move {
                // HTML 経路と同じサービス層関数を呼ぶ。返り値の UserPageView は
                // そのまま Connect レスポンス（= proto3 JSON）になる。
                let view = feature_user::get_user(req.id).await;
                Response::ok(view)
            }),
        )
        .into_axum_service()
}
