use std::{any::TypeId, fmt::Debug, rc::Rc};

use rudi::{components, modules, Context, DynProvider, Module, Singleton};

#[test]
fn name() {
    #[derive(Clone)]
    #[Singleton]
    struct A;

    #[derive(Clone)]
    #[Singleton(name = "b")]
    struct B;

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A, B]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    assert!(cx.resolve_option::<A>().is_some());

    assert!(cx.resolve_option::<B>().is_none());
    assert!(cx.resolve_option_with_name::<B>("b").is_some());
}

#[test]
fn eager_create() {
    #[derive(Clone)]
    #[Singleton]
    struct A;

    #[derive(Clone)]
    #[Singleton(eager_create)]
    struct B;

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A, B]
        }
    }

    let cx = Context::create(modules![MyModule]);

    assert_eq!(cx.singletons_len(), 1);
    assert!(!cx.contains_singleton::<A>());
    assert!(cx.contains_singleton::<B>());
}

#[test]
fn binds() {
    fn transform<T: Debug + 'static>(t: T) -> Rc<dyn Debug> {
        Rc::new(t)
    }

    #[derive(Clone, Debug)]
    #[Singleton]
    struct A;

    #[derive(Clone, Debug)]
    #[Singleton(binds = [transform])]
    struct B;

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A, B]
        }
    }

    let cx = Context::create(modules![MyModule]);
    assert_eq!(cx.get_providers_by_type::<Rc<dyn Debug>>().len(), 1);

    let provider = cx.get_provider::<Rc<dyn Debug>>().unwrap();
    assert_eq!(
        provider.definition().origin.as_ref().unwrap().id,
        TypeId::of::<B>()
    );
}

#[cfg(feature = "auto-register")]
#[test]
fn not_auto_register() {
    #[derive(Clone)]
    #[Singleton]
    struct A;

    #[derive(Clone)]
    #[Singleton(not_auto_register)]
    struct B;

    let cx = Context::auto_register();

    assert!(cx.get_provider::<A>().is_some());
    assert!(cx.get_provider::<B>().is_none());
}

#[test]
fn no_async_constructor() {
    #[Singleton]
    fn One() -> i32 {
        1
    }

    #[derive(Clone)]
    #[Singleton]
    struct A(i32);

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![One, A]
        }
    }

    let mut cx = Context::create(modules![MyModule]);
    assert_eq!(cx.resolve::<A>().0, 1);
}

#[test]
#[should_panic]
fn panicky_async_constructor() {
    #[Singleton]
    async fn One() -> i32 {
        1
    }

    #[derive(Clone)]
    #[Singleton(async_constructor)]
    struct A(i32);

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![One, A]
        }
    }

    let mut cx = Context::create(modules![MyModule]);
    assert_eq!(cx.resolve::<A>().0, 1);
}

#[tokio::test]
async fn successful_async_constructor() {
    #[Singleton]
    async fn One() -> i32 {
        1
    }

    #[derive(Clone)]
    #[Singleton(async_constructor)]
    struct A(i32);

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![One, A]
        }
    }

    let mut cx = Context::create(modules![MyModule]);
    assert_eq!(cx.resolve_async::<A>().await.0, 1);
}
