use crate::model::Todo;
use crate::service::TodoService;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UseCaseError {
    EmptyTitle,
    NotFound,
}

#[derive(Clone, Default)]
pub struct TodoUseCase {
    service: TodoService,
}

impl TodoUseCase {
    pub fn new(service: TodoService) -> Self {
        Self { service }
    }

    pub fn list(&self) -> Vec<Todo> {
        self.service.list()
    }

    pub fn get(&self, id: u64) -> Result<Todo, UseCaseError> {
        self.service.get(id).ok_or(UseCaseError::NotFound)
    }

    pub fn create(&self, title: String) -> Result<Todo, UseCaseError> {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err(UseCaseError::EmptyTitle);
        }
        Ok(self.service.create(title))
    }

    pub fn update_title(&self, id: u64, title: String) -> Result<Todo, UseCaseError> {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err(UseCaseError::EmptyTitle);
        }
        self.service
            .update_title(id, title)
            .ok_or(UseCaseError::NotFound)
    }

    pub fn toggle(&self, id: u64) -> Result<Todo, UseCaseError> {
        self.service.toggle(id).ok_or(UseCaseError::NotFound)
    }

    pub fn delete(&self, id: u64) -> Result<(), UseCaseError> {
        if self.service.delete(id) {
            Ok(())
        } else {
            Err(UseCaseError::NotFound)
        }
    }
}
