mod components;

use std::cell::RefCell;

use rudi::{
    components, modules, Context, DefaultProvider, DynProvider, Module, Singleton, Transient,
};

#[test]
fn eager_create_context() {
    #[derive(Clone)]
    #[Singleton]
    struct A;

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A]
        }
    }

    let cx = Context::options()
        .eager_create(true)
        .create(modules![MyModule]);

    assert!(cx.single_registry().len() == 1);

    let provider = cx.get_provider::<A>();
    assert!(provider.is_some());
    assert!(cx.contains_single::<A>())
}

#[test]
fn eager_create_provider() {
    thread_local! {
        static CREATED: RefCell<bool> = const { RefCell::new(false) };
    }

    #[derive(Clone)]
    struct A;

    #[Singleton(eager_create)]
    impl A {
        #[di]
        fn new() -> A {
            CREATED.set(true);

            A
        }
    }

    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A]
        }
    }

    assert!(A::provider().eager_create());

    let cx = Context::create(modules![MyModule]);

    assert!(cx.single_registry().len() == 1);
    assert!(CREATED.with_borrow(|created| *created));
}

#[test]
fn eager_create_module() {
    thread_local! {
        static COUNT: RefCell<u32> = const { RefCell::new(0) };
    }

    #[derive(Clone)]
    struct A;

    #[Singleton(eager_create)]
    impl A {
        #[di]
        fn new() -> A {
            COUNT.with_borrow_mut(|c| {
                *c += 1;
            });

            A
        }
    }

    struct MyModule;

    impl Module for MyModule {
        fn eager_create() -> bool {
            true
        }

        fn providers() -> Vec<DynProvider> {
            components![A]
        }
    }

    assert!(A::provider().eager_create());

    let mut cx = Context::create(modules![MyModule]);

    assert!(cx.single_registry().len() == 1);
    assert!(COUNT.with_borrow(|created| *created == 1));

    cx.resolve::<A>();

    assert!(cx.single_registry().len() == 1);
    assert!(COUNT.with_borrow(|created| *created == 1));
}

#[test]
fn eager_create_module_twice() {
    thread_local! {
        static COUNT: RefCell<u32> = const { RefCell::new(0) };
    }

    #[derive(Clone)]
    struct A;

    #[Singleton(eager_create)]
    impl A {
        #[di]
        fn new() -> A {
            COUNT.with_borrow_mut(|c| {
                *c += 1;
            });

            A
        }
    }

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A]
        }
    }

    assert!(A::provider().eager_create());

    let mut cx = Context::create(modules![MyModule]);

    assert!(COUNT.with_borrow(|created| *created == 1));
    assert!(cx.single_registry().len() == 1);

    cx.flush();

    assert!(COUNT.with_borrow(|created| *created == 1));
    assert!(cx.single_registry().len() == 1);
}

#[test]
fn eager_create_two_modules() {
    thread_local! {
        static COUNT: RefCell<u32> = const { RefCell::new(0) };
    }

    #[derive(Clone)]
    struct A;

    #[Singleton(eager_create)]
    impl A {
        #[di]
        fn new() -> A {
            COUNT.with_borrow_mut(|c| {
                *c += 1;
            });

            A
        }
    }

    #[derive(Clone)]
    struct B(A);

    #[Singleton(eager_create)]
    impl B {
        #[di]
        fn new(a: A) -> B {
            COUNT.with_borrow_mut(|c| {
                *c += 1;
            });

            B(a)
        }
    }

    struct MyModule1;

    impl Module for MyModule1 {
        fn eager_create() -> bool {
            true
        }

        fn providers() -> Vec<DynProvider> {
            components![A]
        }
    }

    struct MyModule2;

    impl Module for MyModule2 {
        fn eager_create() -> bool {
            true
        }

        fn providers() -> Vec<DynProvider> {
            components![B]
        }
    }

    assert!(A::provider().eager_create());
    assert!(B::provider().eager_create());

    let mut cx = Context::create(modules![MyModule1, MyModule2]);

    assert!(COUNT.with_borrow(|created| *created == 2));
    assert!(cx.single_registry().len() == 2);

    cx.resolve::<A>();
    cx.resolve::<B>();

    assert!(COUNT.with_borrow(|created| *created == 2));
    assert!(cx.single_registry().len() == 2);
}

#[test]
#[should_panic]
fn create_eager_instances_async_in_sync_context() {
    #[Transient(async)]
    struct A;

    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A]
        }
    }

    Context::options()
        .allow_only_single_eager_create(false)
        .eager_create(true)
        .create(modules![MyModule]);
}

#[test]
fn only_singleton_or_all_scope_eager_create() {
    thread_local! {
        static COUNT: RefCell<u32> = const { RefCell::new(0) };
    }

    #[derive(Clone)]
    struct A;

    #[Singleton]
    impl A {
        #[di]
        fn new() -> A {
            COUNT.with_borrow_mut(|c| {
                *c += 1;
            });

            A
        }
    }

    struct B;

    #[Transient]
    impl B {
        #[di]
        fn new() -> B {
            COUNT.with_borrow_mut(|c| {
                *c += 1;
            });

            B
        }
    }

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A, B]
        }
    }

    Context::options()
        .eager_create(true)
        .create(modules![MyModule]);
    assert!(COUNT.with_borrow(|created| *created == 1));

    Context::options()
        .allow_only_single_eager_create(false)
        .eager_create(true)
        .create(modules![MyModule]);
    assert!(COUNT.with_borrow(|created| *created == 3));
}

#[tokio::test]
async fn eager_create_context_async() {
    #[derive(Clone)]
    #[Singleton(async)]
    struct A;

    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A]
        }
    }

    let cx = Context::options()
        .eager_create(true)
        .create_async(modules![MyModule])
        .await;

    assert!(cx.single_registry().len() == 1);

    let provider = cx.get_provider::<A>();
    assert!(provider.is_some());
    assert!(cx.contains_single::<A>())
}

#[tokio::test]
async fn eager_create_provider_async() {
    thread_local! {
        static CREATED: RefCell<bool> = const { RefCell::new(false) };
    }

    #[derive(Clone)]
    struct A;

    #[Singleton(eager_create)]
    impl A {
        #[di]
        async fn new() -> A {
            CREATED.set(true);

            A
        }
    }

    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A]
        }
    }

    assert!(A::provider().eager_create());

    let cx = Context::create_async(modules![MyModule]).await;

    assert!(cx.single_registry().len() == 1);
    assert!(CREATED.with_borrow(|created| *created));
}

#[tokio::test]
async fn eager_create_module_async() {
    thread_local! {
        static COUNT: RefCell<u32> = const { RefCell::new(0) };
    }

    #[derive(Clone)]
    struct A;

    #[Singleton(eager_create)]
    impl A {
        #[di]
        async fn new() -> A {
            COUNT.with_borrow_mut(|c| {
                *c += 1;
            });

            A
        }
    }

    struct MyModule;

    impl Module for MyModule {
        fn eager_create() -> bool {
            true
        }

        fn providers() -> Vec<DynProvider> {
            components![A]
        }
    }

    assert!(A::provider().eager_create());

    let mut cx = Context::create_async(modules![MyModule]).await;

    assert!(cx.single_registry().len() == 1);
    assert!(COUNT.with_borrow(|created| *created == 1));

    cx.resolve_async::<A>().await;

    assert!(cx.single_registry().len() == 1);
    assert!(COUNT.with_borrow(|created| *created == 1));
}

#[tokio::test]
async fn eager_create_module_twice_async() {
    thread_local! {
        static COUNT: RefCell<u32> = const { RefCell::new(0) };
    }

    #[derive(Clone)]
    struct A;

    #[Singleton(eager_create)]
    impl A {
        #[di]
        async fn new() -> A {
            COUNT.with_borrow_mut(|c| {
                *c += 1;
            });

            A
        }
    }

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A]
        }
    }

    assert!(A::provider().eager_create());

    let mut cx = Context::create_async(modules![MyModule]).await;

    assert!(COUNT.with_borrow(|created| *created == 1));
    assert!(cx.single_registry().len() == 1);

    cx.flush_async().await;

    assert!(COUNT.with_borrow(|created| *created == 1));
    assert!(cx.single_registry().len() == 1);
}

#[tokio::test]
async fn eager_create_two_modules_async() {
    thread_local! {
        static COUNT: RefCell<u32> = const { RefCell::new(0) };
    }

    #[derive(Clone)]
    struct A;

    #[Singleton(eager_create)]
    impl A {
        #[di]
        async fn new() -> A {
            COUNT.with_borrow_mut(|c| {
                *c += 1;
            });

            A
        }
    }

    #[derive(Clone)]
    struct B(A);

    #[Singleton(eager_create)]
    impl B {
        #[di]
        async fn new(a: A) -> B {
            COUNT.with_borrow_mut(|c| {
                *c += 1;
            });

            B(a)
        }
    }

    struct MyModule1;

    impl Module for MyModule1 {
        fn eager_create() -> bool {
            true
        }

        fn providers() -> Vec<DynProvider> {
            components![A]
        }
    }

    struct MyModule2;

    impl Module for MyModule2 {
        fn eager_create() -> bool {
            true
        }

        fn providers() -> Vec<DynProvider> {
            components![B]
        }
    }

    assert!(A::provider().eager_create());
    assert!(B::provider().eager_create());

    let mut cx = Context::create_async(modules![MyModule1, MyModule2]).await;

    assert!(COUNT.with_borrow(|created| *created == 2));
    assert!(cx.single_registry().len() == 2);

    cx.resolve_async::<A>().await;
    cx.resolve_async::<B>().await;

    assert!(COUNT.with_borrow(|created| *created == 2));
    assert!(cx.single_registry().len() == 2);
}

#[tokio::test]
async fn create_eager_instances_sync_in_async_context() {
    #[Transient]
    struct A;

    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A]
        }
    }

    Context::options()
        .allow_only_single_eager_create(false)
        .eager_create(true)
        .create_async(modules![MyModule])
        .await;
}

#[tokio::test]
async fn only_singleton_or_all_scope_eager_create_async() {
    thread_local! {
        static COUNT: RefCell<u32> = const { RefCell::new(0) };
    }

    #[derive(Clone)]
    struct A;

    #[Singleton]
    impl A {
        #[di]
        async fn new() -> A {
            COUNT.with_borrow_mut(|c| {
                *c += 1;
            });

            A
        }
    }

    struct B;

    #[Transient]
    impl B {
        #[di]
        async fn new() -> B {
            COUNT.with_borrow_mut(|c| {
                *c += 1;
            });

            B
        }
    }

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A, B]
        }
    }

    Context::options()
        .eager_create(true)
        .create_async(modules![MyModule])
        .await;
    assert!(COUNT.with_borrow(|created| *created == 1));

    Context::options()
        .allow_only_single_eager_create(false)
        .eager_create(true)
        .create_async(modules![MyModule])
        .await;
    assert!(COUNT.with_borrow(|created| *created == 3));
}
