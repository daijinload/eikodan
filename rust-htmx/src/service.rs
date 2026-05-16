use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use crate::model::Todo;

#[derive(Clone, Default)]
pub struct TodoService {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    todos: RwLock<BTreeMap<u64, Todo>>,
    next_id: AtomicU64,
}

impl TodoService {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                todos: RwLock::new(BTreeMap::new()),
                next_id: AtomicU64::new(1),
            }),
        }
    }

    pub fn list(&self) -> Vec<Todo> {
        self.inner.todos.read().unwrap().values().cloned().collect()
    }

    pub fn get(&self, id: u64) -> Option<Todo> {
        self.inner.todos.read().unwrap().get(&id).cloned()
    }

    pub fn create(&self, title: String) -> Todo {
        let id = self.inner.next_id.fetch_add(1, Ordering::SeqCst);
        let todo = Todo {
            id,
            title,
            done: false,
        };
        self.inner.todos.write().unwrap().insert(id, todo.clone());
        todo
    }

    pub fn update_title(&self, id: u64, title: String) -> Option<Todo> {
        let mut map = self.inner.todos.write().unwrap();
        let t = map.get_mut(&id)?;
        t.title = title;
        Some(t.clone())
    }

    pub fn toggle(&self, id: u64) -> Option<Todo> {
        let mut map = self.inner.todos.write().unwrap();
        let t = map.get_mut(&id)?;
        t.done = !t.done;
        Some(t.clone())
    }

    pub fn delete(&self, id: u64) -> bool {
        self.inner.todos.write().unwrap().remove(&id).is_some()
    }
}
