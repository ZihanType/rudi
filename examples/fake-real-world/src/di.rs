use rudi::{Singleton, Transient};

use crate::{
    self_components::{DatabaseConfig, RedisConfig},
    third_components::{DatabaseConnection, Middleware, RedisClient},
};

pub const fn log_name() -> &'static str {
    "log"
}

pub const fn migrator_name() -> &'static str {
    "migrator"
}

#[Singleton(name = log_name())]
fn InitLog() {}

#[Singleton]
async fn NewRedis(config: RedisConfig) -> RedisClient {
    RedisClient::open(&config.url).await
}

#[Singleton]
async fn NewDatabase(config: DatabaseConfig) -> DatabaseConnection {
    DatabaseConnection::open(&config.url).await
}

#[Transient(name = migrator_name())]
async fn Migrator(_database_connection: DatabaseConnection) {}

#[Transient]
async fn NewMiddleware(
    redis_client: RedisClient,
    database_connection: DatabaseConnection,
) -> Middleware {
    Middleware::new(redis_client, database_connection)
}
