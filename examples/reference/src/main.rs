use std::cell::RefCell;

use rudi::{Context, Singleton};

// reference count
thread_local! {
    static COUNT: RefCell<usize> = RefCell::new(0);
}

fn inc_count() {
    COUNT.with(|c| {
        let mut c = c.borrow_mut();
        *c += 1;
    });
}

fn get_count() -> usize {
    COUNT.with(|c| {
        let c = c.borrow();
        *c
    })
}

// fake serialize trait
trait FakeSerialize: Default {}

impl<T: Default> FakeSerialize for T {}

// app config
#[derive(Debug)]
struct AppConfig;

impl Clone for AppConfig {
    fn clone(&self) -> Self {
        inc_count();
        Self
    }
}

#[Singleton]
impl AppConfig {
    // load config from file
    fn load_file() -> Self {
        Self
    }
}

// get other config by reference
impl AppConfig {
    fn get<T: FakeSerialize>(&self) -> T {
        T::default()
    }
}

#[derive(Default, Clone)]
struct DbConfig;

#[Singleton]
impl DbConfig {
    // simple example
    fn new(#[di(ref)] cfg: &AppConfig) -> Self {
        cfg.get()
    }
}

#[derive(Default, Clone)]
struct RedisConfig;

#[Singleton]
impl RedisConfig {
    // example with option
    fn new(#[di(option, ref)] cfg: Option<&AppConfig>) -> Self {
        match cfg {
            Some(cfg) => cfg.get(),
            None => Self,
        }
    }
}

#[Singleton]
fn Run(_db_config: DbConfig, _redis_config: RedisConfig) {
    println!("run!");
}

fn main() {
    let mut cx = Context::auto_register();
    assert_eq!(get_count(), 0);
    cx.resolve::<()>();
    assert_eq!(get_count(), 0);
}
