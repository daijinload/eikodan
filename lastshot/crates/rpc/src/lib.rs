//! Connect API レイヤー。proto で定義した `counter.v1.CounterService` を
//! axum にマウントできる形（`ConnectRpcService`）で公開する。
//!
//! 肝は、ハンドラが feature 側のサービス層関数（[`feature_counter::get_count`] /
//! [`feature_counter::increment`]）をそのまま呼ぶこと。HTML 経路とまったく同じ
//! 関数・同じ生成型・同じ DB プールを使うので、「API を叩いた値」と「画面が使った値」が
//! 同じロジック・同じ DB から生まれる。同一プロセス内なので自分への通信も
//! シリアライズも発生しない。

use connectrpc::{
    handler_fn, ConnectRpcService, RequestContext, Response, Router as ConnectRouter,
};
use db::PgPool;
use schema::{GetCountRequest, IncrementRequest};

/// Connect RPC サービスを構築して axum 用サービスに変換する。
/// app 側は `.fallback_service(rpc::connect_service(pool))` でマウントするだけ。
///
/// `pool` は各ルートのハンドラがリクエストごとに clone して使う（`PgPool` の
/// clone は内部 `Arc` のため安価）。
pub fn connect_service(pool: PgPool) -> ConnectRpcService {
    let get_pool = pool.clone();
    let inc_pool = pool;

    ConnectRouter::new()
        .route(
            "counter.v1.CounterService",
            "GetCount",
            handler_fn(move |_ctx: RequestContext, _req: GetCountRequest| {
                let pool = get_pool.clone();
                async move {
                    // HTML 経路と同じサービス層関数を呼ぶ。返り値の CounterView が
                    // そのまま Connect レスポンス（= proto3 JSON）になる。
                    Response::ok(feature_counter::get_count(&pool).await)
                }
            }),
        )
        .route(
            "counter.v1.CounterService",
            "Increment",
            handler_fn(move |_ctx: RequestContext, _req: IncrementRequest| {
                let pool = inc_pool.clone();
                async move { Response::ok(feature_counter::increment(&pool).await) }
            }),
        )
        .into_axum_service()
}
