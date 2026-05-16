use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Todo {
    pub id: u64,
    pub title: String,
    pub done: bool,
}
