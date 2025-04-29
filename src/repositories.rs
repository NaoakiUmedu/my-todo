use axum::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use thiserror::Error;
use validator::Validate;

#[derive(Debug, Error)]
enum RepositoryError {
    #[error("NotFound, id is {0}")]
    NotFound(i32),
}

/// TODOリポジトリ
#[async_trait]
pub trait TodoRepository: Clone + std::marker::Send + std::marker::Sync + 'static {
    async fn create(&self, payload: CreateTodo) -> anyhow::Result<Todo>;
    async fn find(&self, id: i32) -> anyhow::Result<Todo>;
    async fn all(&self) -> anyhow::Result<Vec<Todo>>;
    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo>;
    async fn delete(&self, id: i32) -> anyhow::Result<()>;
}

/// TODOデータ
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Todo {
    id: i32,
    text: String,
    completed: bool,
}

/// TODO作成用データ
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Validate)]
pub struct CreateTodo {
    #[validate(length(min = 1, message = "Can not be empty"))]
    #[validate(length(max = 100, message = "Over text length"))]
    text: String,
}

/// TODO更新用データ
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Validate)]
pub struct UpdateTodo {
    #[validate(length(min = 1, message = "Can not be empty"))]
    #[validate(length(max = 100, message = "Over text length"))]
    text: Option<String>,
    completed: Option<bool>,
}

//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
/// PostgreSQLリポジトリ
#[derive(Debug, Clone)]
pub struct TodoRepositoryForDb {
    pool: PgPool,
}

impl TodoRepositoryForDb {
    /// new
    pub fn new(pool: PgPool) -> Self {
        Self { pool: (pool) }
    }
}

#[async_trait]
impl TodoRepository for TodoRepositoryForDb {
    async fn create(&self, payload: CreateTodo) -> anyhow::Result<Todo> {
        todo!()
    }
    async fn find(&self, id: i32) -> anyhow::Result<Todo> {
        todo!()
    }
    async fn all(&self) -> anyhow::Result<Vec<Todo>> {
        todo!()
    }
    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo> {
        todo!()
    }
    async fn delete(&self, id: i32) -> anyhow::Result<()> {
        todo!()
    }
}

//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use anyhow::Context;
    use axum::async_trait;
    use std::{
        collections::HashMap,
        sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
    };

    impl CreateTodo {
        /// new object
        pub fn new(text: String) -> Self {
            Self { text }
        }
    }

    impl Todo {
        /// new object
        pub fn new(id: i32, text: String) -> Self {
            Self {
                id,
                text,
                completed: false,
            }
        }
    }

    type TodoDatas = HashMap<i32, Todo>;

    /// オンメモリリポジトリ
    #[derive(Debug, Clone)]
    pub struct TodoRepositoryForMemory {
        store: Arc<RwLock<TodoDatas>>,
    }

    impl TodoRepositoryForMemory {
        /// new object
        pub fn new() -> Self {
            TodoRepositoryForMemory {
                store: Arc::default(),
            }
        }

        /// スレッドセーフにstoreを取得
        fn write_store_ref(&self) -> RwLockWriteGuard<TodoDatas> {
            self.store.write().unwrap()
        }

        /// スレッドセーフにstoreを取得
        fn read_store_ref(&self) -> RwLockReadGuard<TodoDatas> {
            self.store.read().unwrap()
        }
    }

    // オンメモリリポジトリ
    #[async_trait]
    impl TodoRepository for TodoRepositoryForMemory {
        /// TODO作成
        async fn create(&self, payload: CreateTodo) -> anyhow::Result<Todo> {
            let mut store = self.write_store_ref();
            let id = (store.len() + 1) as i32;
            let todo = Todo::new(id, payload.text.clone());
            store.insert(id, todo.clone());
            Ok(todo)
        }
        /// TODO検索
        async fn find(&self, id: i32) -> anyhow::Result<Todo> {
            let store = self.read_store_ref();
            let todo = store
                .get(&id)
                .map(|todo| todo.clone())
                .ok_or(RepositoryError::NotFound(id))?;
            Ok(todo)
        }
        /// 全権取得
        async fn all(&self) -> anyhow::Result<Vec<Todo>> {
            let store = self.read_store_ref();
            Ok(Vec::from_iter(store.values().map(|todo| todo.clone())))
        }
        /// 更新
        async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo> {
            let mut store = self.write_store_ref();
            let todo = store.get(&id).context(RepositoryError::NotFound(id))?;
            let text = payload.text.unwrap_or(todo.text.clone());
            let completed = payload.completed.unwrap_or(todo.completed.clone());
            let todo = Todo {
                id,
                text,
                completed,
            };
            store.insert(id, todo.clone());
            Ok(todo)
        }
        /// 削除
        async fn delete(&self, id: i32) -> anyhow::Result<()> {
            let mut store = self.write_store_ref();
            store.remove(&id).ok_or(RepositoryError::NotFound(id))?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn todo_crud_scenario() {
        let text = "todo text".to_string();
        let id = 1;
        let expected = Todo::new(id, text.clone());

        // create
        let repository = TodoRepositoryForMemory::new();
        let todo = repository
            .create(CreateTodo { text })
            .await
            .expect("failed craete todo");
        assert_eq!(expected, todo);

        // find
        let todo = repository.find(todo.id).await.unwrap();
        assert_eq!(expected, todo);

        // all
        let todo = repository.all().await.unwrap();
        assert_eq!(vec![expected], todo);

        // update
        let text = "update todo text".to_string();
        let todo = repository
            .update(
                1,
                UpdateTodo {
                    text: Some(text.clone()),
                    completed: Some(true),
                },
            )
            .await
            .expect("failed update todo.");
        assert_eq!(
            Todo {
                id,
                text,
                completed: true
            },
            todo
        );

        // delete
        let res = repository.delete(id).await;
        assert!(res.is_ok());
    }
    mod test {
        use super::*;
        use std::vec;

        /// TODOを保持するための型
        type TodoDatas = HashMap<i32, Todo>;

        /// オンメモリリポジトリ
        #[derive(Debug, Clone)]
        pub struct TodoRepositoryForMemory {
            store: Arc<RwLock<TodoDatas>>,
        }

        impl TodoRepositoryForMemory {
            /// new object
            pub fn new() -> Self {
                TodoRepositoryForMemory {
                    store: Arc::default(),
                }
            }

            /// スレッドセーフにstoreを取得
            fn write_store_ref(&self) -> RwLockWriteGuard<TodoDatas> {
                self.store.write().unwrap()
            }

            /// スレッドセーフにstoreを取得
            fn read_store_ref(&self) -> RwLockReadGuard<TodoDatas> {
                self.store.read().unwrap()
            }
        }

        // オンメモリリポジトリ
        #[async_trait]
        impl TodoRepository for TodoRepositoryForMemory {
            /// TODO作成
            async fn create(&self, payload: CreateTodo) -> anyhow::Result<Todo> {
                let mut store = self.write_store_ref();
                let id = (store.len() + 1) as i32;
                let todo = Todo::new(id, payload.text.clone());
                store.insert(id, todo.clone());
                Ok(todo)
            }
            /// TODO検索
            async fn find(&self, id: i32) -> anyhow::Result<Todo> {
                let store = self.read_store_ref();
                let todo = store
                    .get(&id)
                    .map(|todo| todo.clone())
                    .ok_or(RepositoryError::NotFound(id))?;
                Ok(todo)
            }
            /// 全権取得
            async fn all(&self) -> anyhow::Result<Vec<Todo>> {
                let store = self.read_store_ref();
                Ok(Vec::from_iter(store.values().map(|todo| todo.clone())))
            }
            /// 更新
            async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo> {
                let mut store = self.write_store_ref();
                let todo = store.get(&id).context(RepositoryError::NotFound(id))?;
                let text = payload.text.unwrap_or(todo.text.clone());
                let completed = payload.completed.unwrap_or(todo.completed.clone());
                let todo = Todo {
                    id,
                    text,
                    completed,
                };
                store.insert(id, todo.clone());
                Ok(todo)
            }
            /// 削除
            async fn delete(&self, id: i32) -> anyhow::Result<()> {
                let mut store = self.write_store_ref();
                store.remove(&id).ok_or(RepositoryError::NotFound(id))?;
                Ok(())
            }
        }

        #[tokio::test]
        async fn todo_crud_scenario() {
            let text = "todo text".to_string();
            let id = 1;
            let expected = Todo::new(id, text.clone());

            // create
            let repository = TodoRepositoryForMemory::new();
            let todo = repository
                .create(CreateTodo { text })
                .await
                .expect("failed craete todo");
            assert_eq!(expected, todo);

            // find
            let todo = repository.find(todo.id).await.unwrap();
            assert_eq!(expected, todo);

            // all
            let todo = repository.all().await.unwrap();
            assert_eq!(vec![expected], todo);

            // update
            let text = "update todo text".to_string();
            let todo = repository
                .update(
                    1,
                    UpdateTodo {
                        text: Some(text.clone()),
                        completed: Some(true),
                    },
                )
                .await
                .expect("failed update todo.");
            assert_eq!(
                Todo {
                    id,
                    text,
                    completed: true
                },
                todo
            );

            // delete
            let res = repository.delete(id).await;
            assert!(res.is_ok());
        }
    }
}
