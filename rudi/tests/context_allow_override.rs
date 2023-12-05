use rudi::{components, modules, Context, DynProvider, Module, Transient};

#[test]
fn allow_override_in_same_module() {
    #[Transient]
    struct A;

    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A, A]
        }
    }

    let cx = Context::create(modules![MyModule]);
    assert_eq!(cx.provider_registry().len(), 1);
}

#[test]
fn allow_override_in_defferent_module() {
    #[Transient]
    struct A;

    struct MyModule1;
    impl Module for MyModule1 {
        fn providers() -> Vec<DynProvider> {
            components![A]
        }
    }

    struct MyModule2;
    impl Module for MyModule2 {
        fn providers() -> Vec<DynProvider> {
            components![A]
        }
    }

    let cx = Context::create(modules![MyModule1, MyModule2]);
    assert_eq!(cx.provider_registry().len(), 1);
}

#[test]
#[should_panic]
fn disallow_override_in_same_module() {
    #[Transient]
    struct A;

    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A, A]
        }
    }

    let cx = Context::options()
        .allow_override(false)
        .create(modules![MyModule]);
    assert_eq!(cx.provider_registry().len(), 1);
}

#[test]
#[should_panic]
fn disallow_override_in_defferent_module() {
    #[Transient]
    struct A;

    struct MyModule1;
    impl Module for MyModule1 {
        fn providers() -> Vec<DynProvider> {
            components![A]
        }
    }

    struct MyModule2;
    impl Module for MyModule2 {
        fn providers() -> Vec<DynProvider> {
            components![A]
        }
    }

    let cx = Context::options()
        .allow_override(false)
        .create(modules![MyModule1, MyModule2]);
    assert_eq!(cx.provider_registry().len(), 1);
}
