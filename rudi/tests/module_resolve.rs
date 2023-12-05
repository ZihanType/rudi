use std::any::{self, TypeId};

use rudi::{DynProvider, Module, ResolveModule};

struct MyModule;

impl Module for MyModule {
    fn providers() -> Vec<DynProvider> {
        vec![]
    }
}

#[test]
fn resolve_module() {
    let m = ResolveModule::new::<MyModule>();
    assert!(m.ty().id == TypeId::of::<MyModule>());
    assert!(m.ty().name == any::type_name::<MyModule>());
}
