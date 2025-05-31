pub mod todo;
pub mod label;

use thiserror::Error;

#[derive(Debug, Error)]
enum RepositoryError {
    #[error("NotFound, id is {0}")]
    NotFound(i32),
    #[error("Unexpected error: {0}")]
    Unexpected(String),
    #[error("Duplicate ID error: {0}")]
    Duplicate(i32),
}
