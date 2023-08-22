use std::rc::Rc;

use rudi::{components, modules, Context, DynProvider, Module, Singleton};

#[test]
#[should_panic]
fn circular_dependency() {
    trait A {}

    trait B {}

    fn a<T: A + 'static>(t: T) -> Rc<dyn A> {
        Rc::new(t)
    }

    fn b<T: B + 'static>(t: T) -> Rc<dyn B> {
        Rc::new(t)
    }

    #[derive(Clone)]
    #[Singleton(binds = [a])]
    struct AImpl(Rc<dyn B>);

    impl A for AImpl {}

    #[derive(Clone)]
    #[Singleton(binds = [b])]
    struct BImpl(Rc<dyn A>);

    impl B for BImpl {}

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![AImpl, BImpl]
        }
    }

    let mut cx = Context::create(modules![MyModule]);
    cx.resolve::<AImpl>();
}

#[tokio::test]
#[should_panic]
async fn circular_dependency_async() {
    trait A {}

    trait B {}

    fn a<T: A + 'static>(t: T) -> Rc<dyn A> {
        Rc::new(t)
    }

    fn b<T: B + 'static>(t: T) -> Rc<dyn B> {
        Rc::new(t)
    }

    #[derive(Clone)]
    #[Singleton(binds = [a], async)]
    struct AImpl(Rc<dyn B>);

    impl A for AImpl {}

    #[derive(Clone)]
    #[Singleton(binds = [b], async)]
    struct BImpl(Rc<dyn A>);

    impl B for BImpl {}

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![AImpl, BImpl]
        }
    }

    let mut cx = Context::create(modules![MyModule]);
    cx.resolve_async::<AImpl>().await;
}
