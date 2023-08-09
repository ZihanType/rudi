use std::sync::Arc;

use poem::{
    async_trait,
    listener::TcpListener,
    middleware::{Cors, Tracing},
    EndpointExt, Route, Server,
};
use poem_openapi::{param::Query, payload::PlainText, OpenApi, OpenApiService};
use rudi::{Context, Singleton};
use tokio::sync::Mutex;

#[async_trait]
trait Service: Send + Sync {
    async fn insert(&self, name: String);
    async fn search(&self, name: &str) -> Option<String>;
    async fn delete(&self, name: &str);
}

#[derive(Clone)]
#[Singleton(binds = [Self::into_service])]
struct ServiceImpl {
    db: Arc<Mutex<Vec<String>>>,
}

impl ServiceImpl {
    fn into_service(self) -> Arc<dyn Service> {
        Arc::new(self)
    }
}

#[async_trait]
impl Service for ServiceImpl {
    async fn insert(&self, name: String) {
        self.db.lock().await.push(name);
    }

    async fn search(&self, name: &str) -> Option<String> {
        self.db
            .lock()
            .await
            .iter()
            .find(|n| n.contains(name))
            .cloned()
    }

    async fn delete(&self, name: &str) {
        self.db.lock().await.retain(|n| n != name);
    }
}

#[derive(Clone)]
#[Singleton]
struct Controller {
    svc: Arc<dyn Service>,
}

#[OpenApi]
impl Controller {
    #[oai(path = "/insert", method = "post")]
    async fn insert(&self, Query(name): Query<String>) {
        self.svc.insert(name).await;
    }

    #[oai(path = "/search", method = "get")]
    async fn search(&self, Query(name): Query<String>) -> PlainText<String> {
        PlainText(self.svc.search(&name).await.unwrap_or("".to_string()))
    }

    #[oai(path = "/delete", method = "delete")]
    async fn delete(&self, Query(name): Query<String>) {
        self.svc.delete(&name).await;
    }
}

#[Singleton]
fn empty_vec() -> Arc<Mutex<Vec<String>>> {
    Arc::new(Mutex::new(Vec::new()))
}

#[Singleton]
async fn run(controller: Controller) {
    let api_service =
        OpenApiService::new(controller, "Hello World", "1.0").server("http://localhost:3000/api");
    let ui = api_service.swagger_ui();

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(
            Route::new()
                .nest("/api", api_service)
                .nest("/", ui)
                .with(Cors::new())
                .with(Tracing),
        )
        .await
        .unwrap()
}

#[tokio::main]
async fn main() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let mut cx = Context::auto_register();

    // cx.resolve_async::<()>().await;
    cx.resolve_async().await
}
