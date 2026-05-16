use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use minijinja::{context, Value};
use serde::Deserialize;

use crate::usecase::UseCaseError;
use crate::AppState;

#[derive(Deserialize)]
pub struct TitleForm {
    pub title: String,
}

fn render(state: &AppState, template: &str, ctx: Value) -> Response {
    let env = match state.reloader.acquire_env() {
        Ok(e) => e,
        Err(e) => return server_error(format!("acquire_env failed: {e}")),
    };
    let tmpl = match env.get_template(template) {
        Ok(t) => t,
        Err(e) => return server_error(format!("get_template({template}) failed: {e}")),
    };
    match tmpl.render(ctx) {
        Ok(html) => Html(html).into_response(),
        Err(e) => server_error(format!("render({template}) failed: {e}")),
    }
}

fn server_error(msg: String) -> Response {
    (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
}

fn map_usecase_error(e: UseCaseError) -> Response {
    match e {
        UseCaseError::EmptyTitle => (StatusCode::BAD_REQUEST, "title is required").into_response(),
        UseCaseError::NotFound => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn index(State(state): State<AppState>) -> Response {
    let todos = state.usecase.list();
    render(&state, "index.html", context! { todos })
}

pub async fn create(
    State(state): State<AppState>,
    Form(form): Form<TitleForm>,
) -> Response {
    match state.usecase.create(form.title) {
        Ok(t) => render(&state, "partials/todo_row.html", context! { t }),
        Err(e) => map_usecase_error(e),
    }
}

pub async fn show(State(state): State<AppState>, Path(id): Path<u64>) -> Response {
    match state.usecase.get(id) {
        Ok(t) => render(&state, "partials/todo_row.html", context! { t }),
        Err(e) => map_usecase_error(e),
    }
}

pub async fn edit(State(state): State<AppState>, Path(id): Path<u64>) -> Response {
    match state.usecase.get(id) {
        Ok(t) => render(&state, "partials/todo_edit_row.html", context! { t }),
        Err(e) => map_usecase_error(e),
    }
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Form(form): Form<TitleForm>,
) -> Response {
    match state.usecase.update_title(id, form.title) {
        Ok(t) => render(&state, "partials/todo_row.html", context! { t }),
        Err(e) => map_usecase_error(e),
    }
}

pub async fn toggle(State(state): State<AppState>, Path(id): Path<u64>) -> Response {
    match state.usecase.toggle(id) {
        Ok(t) => render(&state, "partials/todo_row.html", context! { t }),
        Err(e) => map_usecase_error(e),
    }
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<u64>) -> Response {
    match state.usecase.delete(id) {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => map_usecase_error(e),
    }
}
