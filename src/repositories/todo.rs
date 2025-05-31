use axum::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use validator::Validate;
use super::RepositoryError;

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
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, FromRow)]
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
    /// 作成
    async fn create(&self, payload: CreateTodo) -> anyhow::Result<Todo> {
        let todo = sqlx::query_as::<_, Todo>(
            r#"
            insert into todos (text, completed)
            values ($1, false)
            returning *
            "#,
        )
        .bind(payload.text.clone())
        .fetch_one(&self.pool)
        .await?;

        Ok(todo)
    }

    /// idをもとに1件取得(主キーなので必ず1件のみ取れる)
    async fn find(&self, id: i32) -> anyhow::Result<Todo> {
        let todo = sqlx::query_as::<_, Todo>(r#"select * from todos where id=$1"#)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => RepositoryError::NotFound(id),
                _ => RepositoryError::Unexpected(e.to_string()),
            })?;

        Ok(todo)
    }

    /// 全件取得
    async fn all(&self) -> anyhow::Result<Vec<Todo>> {
        let todos = sqlx::query_as::<_, Todo>(r#"select * from todos"#)
            .fetch_all(&self.pool)
            .await?;

        Ok(todos)
    }

    /// 更新
    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo> {
        let old_todo = self.find(id).await?;
        let todo = sqlx::query_as(
            r#"
            update todos set text = $1, completed = $2
            where id=$3
            returning *
            "#,
        )
        .bind(payload.text.unwrap_or(old_todo.text))
        .bind(payload.completed.unwrap_or(old_todo.completed))
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(todo)
    }

    /// 削除
    async fn delete(&self, id: i32) -> anyhow::Result<()> {
        sqlx::query(r#"delete from todos where id=$1"#)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => RepositoryError::NotFound(id),
                _ => RepositoryError::Unexpected(e.to_string()),
            })?;

        Ok(())
    }
}

/// DB用リポジトリのためのテスト
#[cfg(test)]
#[cfg(feature = "database-test")]
mod test {
    use super::*;
    use dotenv::dotenv;
    use sqlx::PgPool;
    use std::env;

    /// シナリオテスト(DBが起動している必要がある)
    #[tokio::test]
    async fn todo_crud_scenario() {
        // point 1
        dotenv().ok();
        let database_url = &env::var("DATABASE_URL").expect("undefined [DATABASE_URL");
        let pool = PgPool::connect(database_url)
            .await
            .expect(&format!("fail connect database, url is [{}]", database_url));
        let repository = TodoRepositoryForDb::new(pool.clone());
        let todo_text = "[crud_scenario] text";

        // create
        let created = repository
            .create(CreateTodo::new(todo_text.to_string()))
            .await
            .expect("[create] returned Err");
        assert_eq!(created.text, todo_text);
        assert!(!created.completed);

        // find
        let todo = repository
            .find(created.id)
            .await
            .expect("[find] returned Err");
        assert_eq!(created, todo);

        // all
        let todos = repository.all().await.expect("[all] returned Err");
        let mut is_ok = false;
        for todo in todos {
            if created == todo {
                is_ok = true;
            }
        }
        assert!(is_ok);

        // update
        let updated_text = "[crud_scenario] updated text";
        let todo = repository
            .update(
                todo.id,
                UpdateTodo {
                    text: Some(updated_text.to_string()),
                    completed: Some(true),
                },
            )
            .await
            .expect("[update] returned Err");
        assert_eq!(updated_text, todo.text);
        assert!(todo.completed);

        // delete
        let _ = repository
            .delete(todo.id)
            .await
            .expect("[delete] returned Err");
        // deleteで消えていること
        let res = repository.find(created.id).await;
        assert!(res.is_err());

        let todo_rows = sqlx::query(
            r#"
               select * from todos where id=$1
            "#,
        )
        .bind(todo.id)
        .fetch_all(&pool)
        .await
        .expect("[delete] todo_labels fetch error");
        assert!(todo_rows.len() == 0);
    }
}

//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------

/// テスト用便利屋さん
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

    type TodoData = HashMap<i32, Todo>;

    /// オンメモリリポジトリ
    #[derive(Debug, Clone)]
    pub struct TodoRepositoryForMemory {
        store: Arc<RwLock<TodoData>>,
    }

    impl TodoRepositoryForMemory {
        /// new object
        pub fn new() -> Self {
            TodoRepositoryForMemory {
                store: Arc::default(),
            }
        }

        /// スレッドセーフにstoreを取得
        fn write_store_ref(&self) -> RwLockWriteGuard<TodoData> {
            self.store.write().unwrap()
        }

        /// スレッドセーフにstoreを取得
        fn read_store_ref(&self) -> RwLockReadGuard<TodoData> {
            self.store.read().unwrap()
        }
    }

    /// オンメモリリポジトリ
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
    mod test {
        use super::*;
        use std::vec;

        /// TODOを保持するための型
        type TodoData = HashMap<i32, Todo>;

        /// オンメモリリポジトリ
        #[derive(Debug, Clone)]
        pub struct TodoRepositoryForMemory {
            store: Arc<RwLock<TodoData>>,
        }

        impl TodoRepositoryForMemory {
            /// new object
            pub fn new() -> Self {
                TodoRepositoryForMemory {
                    store: Arc::default(),
                }
            }

            /// スレッドセーフにstoreを取得
            fn write_store_ref(&self) -> RwLockWriteGuard<TodoData> {
                self.store.write().unwrap()
            }

            /// スレッドセーフにstoreを取得
            fn read_store_ref(&self) -> RwLockReadGuard<TodoData> {
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
                .expect("failed create todo");
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
