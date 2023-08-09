#[derive(Clone)]
pub struct RedisClient;

impl RedisClient {
    pub async fn open(_url: &str) -> Self {
        RedisClient
    }
}

#[derive(Clone)]
pub struct DatabaseConnection;

impl DatabaseConnection {
    pub async fn open(_url: &str) -> Self {
        DatabaseConnection
    }
}

pub struct Middleware;

impl Middleware {
    pub fn new(_redis_client: RedisClient, _database_connection: DatabaseConnection) -> Self {
        Middleware
    }
}
