use axum::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use super::RepositoryError;

/// ラベルリポジトリ
#[async_trait]
pub trait LabelRepository: Clone + Send + Sync + 'static {
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
        let optional_label = sqlx::query_as::<_, Label>(

            r#" select * from labels where name = $1 "#
        ).bind(name.clone())
            .fetch_optional(&self.pool)
            .await?;

        if let Some(label) = optional_label {
            return Err(RepositoryError::Duplicate(label.id).into());
        }

        let label = sqlx::query_as::<_, Label>(
            r#" insert into labels ( name ) values ($1) returning * "#,
        )
            .bind(name.clone())
            .fetch_one(&self.pool)
            .await?;

        Ok(label)
    }
    /// 全件取得
    async fn all(&self) -> anyhow::Result<Vec<Label>> {
        let labels = sqlx::query_as::<_, Label>(
            r#" select * from labels order by labels.id asc "#,
        ).fetch_all(&self.pool).await?;

        Ok(labels)
    }
    /// 削除
    async fn delete(&self, id: i32) -> anyhow::Result<()> {
        sqlx::query(
            r#" delete from labels where id = $1 "#,
        ).bind(id).execute(&self.pool).await.map_err(|e| match e {
            sqlx::Error::RowNotFound => RepositoryError::NotFound(id),
            _ => RepositoryError::Unexpected(e.to_string()),
        })?;

        Ok(())
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
    async fn crud_scenario() {
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
    use std::{collections::HashMap, sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard}};
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
            let label = Label{id, name };
            store.insert(id, label.clone());
            Ok(label)
        }
        /// 全件取得
        async fn all(&self) -> anyhow::Result<Vec<Label>> {
            let store: RwLockReadGuard<LabelData> = self.read_store_ref();
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
    async fn crud_scenario() {

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
