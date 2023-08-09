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
fn init_log() {}

#[Singleton]
async fn new_redis(config: RedisConfig) -> RedisClient {
    RedisClient::open(&config.url).await
}

#[Singleton]
async fn new_database(config: DatabaseConfig) -> DatabaseConnection {
    DatabaseConnection::open(&config.url).await
}

#[Transient(name = migrator_name())]
async fn migrator(_database_connection: DatabaseConnection) {}

#[Transient]
async fn new_middleware(
    redis_client: RedisClient,
    database_connection: DatabaseConnection,
) -> Middleware {
    Middleware::new(redis_client, database_connection)
}
