use axum::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// ラベルリポジトリ
#[async_trait]
pub trait LabelRepository: Clone + std::marker::Send + std::marker::Sync + 'static {
    async fn create(&self, name: String) -> anyhow::Result<Label>;
    async fn all(&self) -> anyhow::Result<Vec<Label>>;
    async fn delete(&self, id: i32) -> anyhow::Result<()>;
}

/// ラベル
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Label {
    pub id: i32,
    pub name: String,
}
impl Label {
    pub fn new(id: i32, name: String) -> Self {
        Self { id, name }
    }
}

/// ラベル(Update用)
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct UpdateLabel {
    pub id: i32,
    pub name: String,
}


//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
/// PgSqlリポジトリ
#[derive(Debug, Clone)]
pub struct LabelRepositoryForDb {
    pool: PgPool,
}
impl LabelRepositoryForDb {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
#[async_trait]
impl LabelRepository for LabelRepositoryForDb {
    /// 新規作成
    async fn create(&self, name: String) -> anyhow::Result<Label> {
        todo!()
    }
    /// 全件取得
    async fn all(&self) -> anyhow::Result<Vec<Label>> {
        todo!()
    }
    /// 削除
    async fn delete(&self, id: i32) -> anyhow::Result<()> {
        todo!()
    }
}

//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
/// DB用リポジトリのためのテスト
#[cfg(test)]
#[cfg(feature = "database-test")]
mod test {
    use super::*;
    use dotenv::dotenv;
    use sqlx::PgPool;
    use std::env;

    const DB_URL_ENV: &str = "DATABASE_URL";

    #[tokio::test]
    async fn crud_scneario() {
        dotenv().ok();
        let database_url = &env::var(DB_URL_ENV).expect(&format!("undefined [{}]", DB_URL_ENV));
        let pool = PgPool::connect(database_url).await.expect(&format!("fail connect database, url is [{}]", database_url));

        let repository = LabelRepositoryForDb::new(pool);
        let label_text = "test_label";

        // C
        let label = repository.create(label_text.to_string()).await.expect("[create] returned Err");
        assert_eq!(label.name, label_text);

        // all
        let labels = repository.all().await.expect("[all] returned Err");
        let label = labels.last().unwrap();
        assert_eq!(label.name, label_text);

        // d
        repository.delete(label.id).await.expect("[delete] returned Err");
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
    use std::{collections::HashMap, env, sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard}};
    use dotenv::dotenv;
    use crate::repositories::RepositoryError;

    type LabelData = HashMap<i32, Label>;

    /// オンメモリリポジトリ
    #[derive(Debug, Clone)]
    pub struct LabelRepositoryForMemory {
        store: Arc<RwLock<LabelData>>,
    }
    impl LabelRepositoryForMemory {
        /// new object
        pub fn new() -> Self {
            LabelRepositoryForMemory {
                store: Arc::default(),
            }
        }
        /// スレッドセーフにstoreを取得(write)
        fn write_store_ref(&self) -> RwLockWriteGuard<LabelData> { self.store.write().unwrap() }
        /// スレッドセーフにstoreを取得(read)
        fn read_store_ref(&self) -> RwLockReadGuard<LabelData> { self.store.read().unwrap() }
    }
    impl LabelRepositoryForMemory {
        /// 新規作成
        async fn create(&self, name: String) -> anyhow::Result<Label> {
            let mut store = self.write_store_ref();
            let id = (store.len() + 1) as i32;
            let label = Label::new(id, name.to_string());
            store.insert(id, label.clone());
            Ok(label)
        }
        /// 全件取得
        async fn all(&self) -> anyhow::Result<Vec<Label>> {
            let store: RwLockWriteGuard<LabelData> = self.write_store_ref();
            Ok(Vec::from_iter(store.values().map(|label| label.clone())))
        }
        /// 削除
        async fn delete(&self, id: i32) -> anyhow::Result<()> {
            let mut store = self.write_store_ref();
            store.remove(&id).ok_or(RepositoryError::NotFound(id))?;
            Ok(())
        }
    }
    /// CRUD シナリオ
    #[tokio::test]
    async fn crud_scneario() {

        let repository = LabelRepositoryForMemory::new();
        let label_text = "test_label";

        // C
        let label = repository.create(label_text.to_string()).await.expect("[create] returned Err");
        assert_eq!(label.name, label_text);

        // all
        let labels = repository.all().await.expect("[all] returned Err");
        let label = labels.last().unwrap();
        assert_eq!(label.name, label_text);

        // d
        repository.delete(label.id).await.expect("[delete] returned Err");
    }
}
