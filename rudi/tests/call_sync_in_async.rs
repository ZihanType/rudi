use rudi::*;

#[tokio::test]
async fn resolve_in_async_context() {
    #[Transient]
    struct A;

    #[derive(Clone)]
    struct B;

    #[Singleton]
    impl B {
        fn new() -> B {
            B
        }
    }

    #[Singleton]
    fn Number() -> i32 {
        42
    }

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A, B, Number]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    assert!(cx.resolve_option_async::<A>().await.is_some());
    assert!(cx.resolve_option_async::<B>().await.is_some());
    assert!(cx.resolve_option_async::<i32>().await.is_some());
}

#[tokio::test]
async fn create_eager_instance_in_async_context() {
    #[Transient]
    struct A;

    #[derive(Clone)]
    struct B;

    #[Singleton]
    impl B {
        fn new() -> B {
            B
        }
    }

    #[Singleton]
    fn Number() -> i32 {
        42
    }

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A, B, Number]
        }
    }

    let cx = Context::options()
        .eager_create(true)
        .create_async(modules![MyModule])
        .await;

    assert_eq!(cx.providers_len(), 3);
    assert_eq!(cx.singletons_len(), 2);
}
