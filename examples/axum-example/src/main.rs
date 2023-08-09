use std::sync::Arc;

use axum::{
    async_trait,
    extract::{Path, State},
    routing::{delete, get, post},
    Router,
};
use rudi::{Context, Singleton};
use tokio::sync::Mutex;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

async fn insert(Path(name): Path<String>, State(svc): State<Arc<dyn Service>>) {
    svc.insert(name).await;
}

async fn search(Path(name): Path<String>, State(svc): State<Arc<dyn Service>>) -> String {
    svc.search(&name).await.unwrap_or("".to_string())
}

async fn del(Path(name): Path<String>, State(svc): State<Arc<dyn Service>>) {
    svc.delete(&name).await;
}

#[Singleton]
fn empty_vec() -> Arc<Mutex<Vec<String>>> {
    Arc::new(Mutex::new(Vec::new()))
}

#[Singleton]
async fn run(svc: Arc<dyn Service>) {
    let app = Router::new()
        .route("/insert/:name", post(insert))
        .route("/search/:name", get(search))
        .route("/delete/:name", delete(del))
        .with_state(svc);

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "axum_example=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mut cx = Context::auto_register();

    // cx.resolve_async::<()>().await;
    cx.resolve_async().await
}
