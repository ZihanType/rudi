use rudi::{Context, Singleton, Transient};

#[test]
fn auto_register() {
    #[Transient]
    struct A;

    #[Singleton]
    fn Number() -> i32 {
        1
    }

    #[allow(dead_code)]
    struct B(i32);

    #[Transient]
    impl B {
        #[di]
        fn new(i: i32) -> B {
            B(i)
        }
    }

    let mut cx = Context::auto_register();
    assert!(cx.resolve_option::<A>().is_some());
    assert!(cx.resolve_option::<i32>().is_some());
    assert!(cx.resolve_option::<B>().is_some());
}

#[tokio::test]
async fn auto_register_async() {
    #[Transient(async)]
    struct A;

    #[Singleton]
    async fn Number() -> i32 {
        1
    }

    #[allow(dead_code)]
    struct B(i32);

    #[Transient]
    impl B {
        #[di]
        async fn new(i: i32) -> B {
            B(i)
        }
    }

    let mut cx = Context::auto_register_async().await;
    assert!(cx.resolve_option_async::<A>().await.is_some());
    assert!(cx.resolve_option_async::<i32>().await.is_some());
    assert!(cx.resolve_option_async::<B>().await.is_some());
}
