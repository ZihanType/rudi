use std::{rc::Rc, sync::Arc};

use rudi::Singleton;

use crate::third_components::{DatabaseConnection, RedisClient};

pub trait Service {}

#[derive(Clone)]
#[Singleton(binds = [Self::into_svc], async)]
pub struct ServiceImpl(RedisClient, DatabaseConnection);

impl Service for ServiceImpl {}

impl ServiceImpl {
    pub fn into_svc(self) -> Arc<dyn Service> {
        Arc::new(self)
    }
}

#[derive(Clone)]
#[Singleton(async)]
pub struct Controller(Arc<dyn Service>);

#[derive(Clone)]
pub struct ApplicationConfig;

#[Singleton(binds = [Rc::new])]
impl ApplicationConfig {
    fn load() -> Self {
        ApplicationConfig
    }
}

#[derive(Clone)]
pub struct RedisConfig {
    pub url: String,
}

#[Singleton]
impl RedisConfig {
    fn load(#[di(ref)] _application_config: &ApplicationConfig) -> Self {
        RedisConfig {
            url: "redis://localhost:6379".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

#[Singleton]
impl DatabaseConfig {
    fn load(#[di(ref)] _application_config: &ApplicationConfig) -> Self {
        DatabaseConfig {
            url: "postgres://localhost:5432".to_string(),
        }
    }
}
