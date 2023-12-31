use rudi::{Context, SingleOwner, Singleton, Transient};

#[SingleOwner] // SingleOwner scope
struct NotCloneable;

#[Singleton] // Singleton scope
struct Cloneable;

// Singleton must implement Clone
impl Clone for Cloneable {
    fn clone(&self) -> Self {
        unimplemented!("actually this method will not be called")
    }
}

struct DbConfig;

#[Transient]
impl DbConfig {
    // from reference
    #[di]
    fn from_single_owner_reference(#[di(ref)] _: &NotCloneable) -> Self {
        Self
    }
}

struct RedisConfig;

#[Transient]
impl RedisConfig {
    // from option reference
    #[di]
    fn from_singleton_reference(#[di(option, ref)] _: Option<&Cloneable>) -> Self {
        Self
    }
}

#[Singleton]
fn Run(_: DbConfig, _: RedisConfig) {
    println!("run!");
}

fn main() {
    let mut cx = Context::auto_register();
    cx.resolve()
}
