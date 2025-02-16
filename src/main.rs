mod handlers;
mod repositories;

use crate::repositories::{TodoRepository, TodoRepositoryForMemory};
use axum::{
    extract::Extension,
    routing::{get, post},
    Router,
};

use handlers::{all_todo, create_todo, delete_todo, find_todo, update_todo};
use std::net::SocketAddr;
use std::{env, sync::Arc};

#[tokio::main]
async fn main() {
    // loggingの初期化
    let leg_level = env::var("RUST_LOG").unwrap_or("info".to_string());
    env::set_var("RUST_LOG", leg_level);
    tracing_subscriber::fmt::init();

    // サーバ立ち上げ
    let repository = TodoRepositoryForMemory::new();
    let app = create_app(repository);
    let addr = SocketAddr::from(([192, 168, 11, 9], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn create_app<T: TodoRepository>(repository: T) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/todos", post(create_todo::<T>))
        .route("/todos", post(create_todo::<T>).get(all_todo::<T>))
        .route(
            "/todos/:id",
            get(find_todo::<T>)
                .delete(delete_todo::<T>)
                .patch(update_todo::<T>),
        )
        .layer(Extension(Arc::new(repository)))
}

async fn root() -> &'static str {
    "Hello! axum!!"
}

#[cfg(test)]
mod test {
    // **point 1**
    use super::*;
    use crate::repositories::{CreateTodo, Todo};
    use axum::response::Response;
    use axum::{
        body::Body,
        http::{header, Method, Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn should_return_hello_world() {
        let repository = TodoRepositoryForMemory::new();
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let res = create_app(repository).oneshot(req).await.unwrap();
        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body: String = String::from_utf8(bytes.to_vec()).unwrap();
        assert_eq!(body, "Hello! axum!!");
    }

    #[tokio::test]
    async fn should_created_todo() {
        let expected = Todo::new(1, "should_return_created_todo".to_string());

        let repository = TodoRepositoryForMemory::new();
        let req = build_todo_req_with_json(
            "/todos",
            Method::POST,
            r#"{ "text": "should_return_created_todo" }"#.to_string(),
        );
        let res = create_app(repository).oneshot(req).await.unwrap();
        let todo = res_to_todo(res).await;
        assert_eq!(expected, todo);
    }

    #[tokio::test]
    async fn should_find_todo() {
        let expected = Todo::new(1, "should_find_todo".to_string());

        let repository = TodoRepositoryForMemory::new();
        repository.create(CreateTodo::new("should_find_todo".to_string()));
        let req = build_todo_req_with_empty(Method::GET, "/todos/1");
        let res = create_app(repository).oneshot(req).await.unwrap();
        let todo = res_to_todo(res).await;
        assert_eq!(expected, todo);
    }

    // TODO should_get_all_todos()から
}
