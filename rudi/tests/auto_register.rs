#[cfg(feature = "auto-register")]
#[cfg(test)]
mod tests {
    use rudi::*;

    #[test]
    fn auto_register() {
        #[Transient]
        struct A;

        #[Singleton]
        fn a() -> i32 {
            1
        }

        struct B(i32);

        #[Transient]
        impl B {
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
        #[Transient(async_constructor)]
        struct A;

        #[Singleton]
        async fn a() -> i32 {
            1
        }

        struct B(i32);

        #[Transient]
        impl B {
            async fn new(i: i32) -> B {
                B(i)
            }
        }

        let mut cx = Context::auto_register_async().await;
        assert!(cx.resolve_option_async::<A>().await.is_some());
        assert!(cx.resolve_option_async::<i32>().await.is_some());
        assert!(cx.resolve_option_async::<B>().await.is_some());
    }
}
