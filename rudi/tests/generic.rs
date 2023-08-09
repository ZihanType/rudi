use std::marker::PhantomData;

use rudi::{components, modules, Context, Module, Transient};

#[test]
fn generic_provider() {
    #[derive(Default)]
    #[Transient(not_auto_register)]
    struct A<T: Default + 'static>(T);

    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            components![A<i32>]
        }
    }

    let cx = Context::create(modules![MyModule]);
    assert!(cx.get_provider::<A<i32>>().is_some());
}

#[test]
fn generic_module() {
    #[derive(Default)]
    #[Transient(not_auto_register)]
    struct A<T: Default + 'static>(T);

    struct MyModule<T>(PhantomData<T>);
    impl<T: Default + 'static> Module for MyModule<T> {
        fn providers() -> Vec<rudi::DynProvider> {
            components![A<T>]
        }
    }

    let cx = Context::create(modules![MyModule::<i32>]);
    assert!(cx.get_provider::<A<i32>>().is_some());
}

#[test]
fn generic_provider_async() {
    #[derive(Default)]
    #[Transient(async_constructor, not_auto_register)]
    struct B<T: Default + 'static>(T);

    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            components![B<i32>]
        }
    }

    let cx = Context::create(modules![MyModule]);
    assert!(cx.get_provider::<B<i32>>().is_some());
}

#[test]
fn generic_module_async() {
    #[derive(Default)]
    #[Transient(async_constructor, not_auto_register)]
    struct B<T: Default + 'static>(T);

    struct MyModule<T>(PhantomData<T>);
    impl<T: Default + 'static> Module for MyModule<T> {
        fn providers() -> Vec<rudi::DynProvider> {
            components![B<T>]
        }
    }

    let cx = Context::create(modules![MyModule::<i32>]);
    assert!(cx.get_provider::<B<i32>>().is_some());
}
