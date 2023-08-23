use rudi::{modules, Context, Scope};

#[test]
fn standalone_variables() {
    let mut cx = Context::options()
        .instance(42i32)
        .instance(true)
        .instance("Hello world")
        .create(modules![]);

    assert!(cx.singleton_registry().is_empty());

    assert_eq!(cx.resolve::<i32>(), 42);
    assert!(cx.resolve::<bool>());
    assert_eq!(cx.resolve::<&str>(), "Hello world");

    assert_eq!(cx.singleton_registry().len(), 3);

    cx.provider_registry().iter().for_each(|(_, provider)| {
        assert!(provider.definition().scope == Scope::Singleton);
        assert!(!provider.eager_create());
    });
}
