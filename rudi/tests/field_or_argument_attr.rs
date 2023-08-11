use rudi::{components, modules, Context, DynProvider, Module, Singleton};

#[test]
fn name() {
    #[Singleton]
    fn One() -> i32 {
        1
    }

    #[Singleton(name = "Two")]
    fn Two() -> i32 {
        2
    }

    #[derive(Clone)]
    #[Singleton]
    struct A(i32);

    #[derive(Clone)]
    #[Singleton]
    struct B(#[di(name = "Two")] i32);

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![One, Two, A, B]
        }
    }

    let mut cx = Context::create(modules![MyModule]);
    assert_eq!(cx.resolve::<A>().0, 1);
    assert_eq!(cx.resolve::<B>().0, 2);
}

#[test]
fn option() {
    #[Singleton]
    fn One() -> Option<i32> {
        Some(1)
    }

    #[Singleton]
    fn Two() -> i32 {
        2
    }

    #[derive(Clone)]
    #[Singleton]
    struct A(Option<i32>);

    #[derive(Clone)]
    #[Singleton]
    struct B(#[di(option(i32))] Option<i32>);

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![One, Two, A, B]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    assert_eq!(cx.resolve::<A>().0.unwrap(), 1);
    assert_eq!(cx.resolve::<B>().0.unwrap(), 2);
}

#[test]
fn default() {
    #[derive(Clone)]
    #[Singleton]
    struct A(#[di(default)] i32);

    #[derive(Clone)]
    #[Singleton]
    struct B(#[di(default = 42)] i32);

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A, B]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    assert_eq!(cx.resolve::<A>().0, 0);
    assert_eq!(cx.resolve::<B>().0, 42);
}

#[test]
fn vector() {
    #[Singleton]
    fn One() -> Vec<i32> {
        vec![1]
    }

    #[Singleton]
    fn Two() -> i32 {
        2
    }

    #[derive(Clone)]
    #[Singleton]
    struct A(Vec<i32>);

    #[derive(Clone)]
    #[Singleton]
    struct B(#[di(vector(i32))] Vec<i32>);

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![One, Two, A, B]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    assert_eq!(cx.resolve::<A>().0[0], 1);
    assert_eq!(cx.resolve::<B>().0[0], 2);
}
