mod controller;
mod model;
mod service;
mod usecase;

use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use minijinja::{path_loader, Environment};
use minijinja_autoreload::AutoReloader;

use crate::service::TodoService;
use crate::usecase::TodoUseCase;

#[derive(Clone)]
pub struct AppState {
    pub usecase: TodoUseCase,
    pub reloader: Arc<AutoReloader>,
}

pub fn app() -> Router {
    let usecase = TodoUseCase::new(TodoService::new());

    let reloader = Arc::new(AutoReloader::new(move |notifier| {
        let mut env = Environment::new();
        let template_path = PathBuf::from("templates");
        notifier.set_fast_reload(true);
        notifier.watch_path(&template_path, true);
        env.set_loader(path_loader(template_path));
        Ok(env)
    }));

    let state = AppState { usecase, reloader };

    let router = Router::new()
        .route("/", get(controller::index))
        .route("/todos", post(controller::create))
        .route(
            "/todos/{id}",
            get(controller::show)
                .put(controller::update)
                .delete(controller::delete),
        )
        .route("/todos/{id}/edit", get(controller::edit))
        .route("/todos/{id}/toggle", post(controller::toggle))
        .with_state(state);

    with_live_reload(router)
}

#[cfg(all(debug_assertions, not(test)))]
fn not_htmx_predicate<T>(req: &axum::http::Request<T>) -> bool {
    !req.headers().contains_key("hx-request")
}

#[cfg(all(debug_assertions, not(test)))]
fn with_live_reload(router: Router) -> Router {
    use notify::{RecursiveMode, Watcher};
    use tower_livereload::LiveReloadLayer;

    let livereload = LiveReloadLayer::new();
    let reloader = livereload.reloader();

    tokio::spawn(async move {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
        let mut watcher = notify::recommended_watcher(
            move |res: notify::Result<notify::Event>| {
                if res.is_ok() {
                    let _ = tx.send(());
                }
            },
        )
        .expect("create file watcher");
        watcher
            .watch(std::path::Path::new("templates"), RecursiveMode::Recursive)
            .expect("watch templates");

        while rx.recv().await.is_some() {
            reloader.reload();
        }
        drop(watcher);
    });

    router.layer(livereload.request_predicate(not_htmx_predicate))
}

#[cfg(not(all(debug_assertions, not(test))))]
fn with_live_reload(router: Router) -> Router {
    router
}

#[tokio::main]
async fn main() {
    let app = app();
    let addr = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("listening on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn body_string(resp: axum::response::Response) -> String {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    fn get(uri: &str) -> Request<Body> {
        Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    fn form_request(method: Method, uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    fn delete_request(uri: &str) -> Request<Body> {
        Request::builder()
            .method(Method::DELETE)
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn get_index_returns_html_with_table() {
        let app = app();
        let resp = app.oneshot(get("/")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        assert!(body.contains("<table"), "body should contain <table: {body}");
    }

    #[tokio::test]
    async fn post_todos_returns_fragment_with_title() {
        let app = app();
        let resp = app
            .oneshot(form_request(Method::POST, "/todos", "title=buy+milk"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        assert!(body.contains("buy milk"), "fragment should contain title: {body}");
        assert!(body.trim_start().starts_with("<tr"), "fragment should be a <tr row: {body}");
    }

    #[tokio::test]
    async fn created_todo_appears_in_index() {
        let app = app();
        let _ = app
            .clone()
            .oneshot(form_request(Method::POST, "/todos", "title=integration+test"))
            .await
            .unwrap();
        let resp = app.oneshot(get("/")).await.unwrap();
        let body = body_string(resp).await;
        assert!(body.contains("integration test"));
    }

    #[tokio::test]
    async fn toggle_flips_done_state() {
        let app = app();
        let _ = app
            .clone()
            .oneshot(form_request(Method::POST, "/todos", "title=ToggleMe"))
            .await
            .unwrap();
        let resp = app
            .oneshot(form_request(Method::POST, "/todos/1/toggle", ""))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        assert!(body.contains("checked"), "toggled row should have checked attr: {body}");
        assert!(body.contains("line-through"), "toggled row should have line-through class: {body}");
    }

    #[tokio::test]
    async fn put_updates_title() {
        let app = app();
        let _ = app
            .clone()
            .oneshot(form_request(Method::POST, "/todos", "title=before"))
            .await
            .unwrap();
        let resp = app
            .oneshot(form_request(Method::PUT, "/todos/1", "title=after"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        assert!(body.contains("after"));
        assert!(!body.contains(">before<"));
    }

    #[tokio::test]
    async fn delete_removes_todo_from_index() {
        let app = app();
        let _ = app
            .clone()
            .oneshot(form_request(Method::POST, "/todos", "title=will+be+deleted"))
            .await
            .unwrap();
        let resp = app.clone().oneshot(delete_request("/todos/1")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let index = app.oneshot(get("/")).await.unwrap();
        let body = body_string(index).await;
        assert!(!body.contains("will be deleted"), "deleted todo must be gone: {body}");
    }
}
