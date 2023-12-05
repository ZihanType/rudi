use std::marker::PhantomData;

use rudi::{components, modules, Context, DynProvider, Module, Transient};

#[test]
fn generic_provider() {
    #[derive(Default)]
    #[Transient(auto_register = false)]
    struct A<T: Default + 'static>(T);

    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A<i32>]
        }
    }

    let cx = Context::create(modules![MyModule]);
    assert!(cx.get_provider::<A<i32>>().is_some());
}

#[test]
fn generic_module() {
    #[derive(Default)]
    #[Transient(auto_register = false)]
    struct A<T: Default + 'static>(T);

    struct MyModule<T>(PhantomData<T>);
    impl<T: Default + 'static> Module for MyModule<T> {
        fn providers() -> Vec<DynProvider> {
            components![A<T>]
        }
    }

    let cx = Context::create(modules![MyModule::<i32>]);
    assert!(cx.get_provider::<A<i32>>().is_some());
}

#[test]
fn generic_provider_async() {
    #[derive(Default)]
    #[Transient(async, auto_register = false)]
    struct B<T: Default + 'static>(T);

    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![B<i32>]
        }
    }

    let cx = Context::create(modules![MyModule]);
    assert!(cx.get_provider::<B<i32>>().is_some());
}

#[test]
fn generic_module_async() {
    #[derive(Default)]
    #[Transient(async, auto_register = false)]
    struct B<T: Default + 'static>(T);

    struct MyModule<T>(PhantomData<T>);
    impl<T: Default + 'static> Module for MyModule<T> {
        fn providers() -> Vec<DynProvider> {
            components![B<T>]
        }
    }

    let cx = Context::create(modules![MyModule::<i32>]);
    assert!(cx.get_provider::<B<i32>>().is_some());
}
