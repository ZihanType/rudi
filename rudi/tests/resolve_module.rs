use std::any::{self, TypeId};

use rudi::{Module, ResolveModule};

struct MyModule;

impl Module for MyModule {
    fn providers() -> Vec<rudi::DynProvider> {
        vec![]
    }
}

#[test]
fn resolve_module() {
    let m = ResolveModule::new::<MyModule>();
    assert!(m.id() == TypeId::of::<MyModule>());
    assert!(m.name() == any::type_name::<MyModule>());
}
