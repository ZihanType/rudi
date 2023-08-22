use std::{any::TypeId, fmt::Debug, rc::Rc};

use rudi as ru_di;
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
fn condition() {
    #[derive(Clone)]
    #[Singleton]
    struct A;

    #[derive(Clone)]
    #[Singleton(condition = |_| false)]
    struct B;

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![A, B]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    assert_eq!(cx.providers_len(), 1);
    assert!(cx.resolve_option::<A>().is_some());
    assert!(cx.resolve_option::<B>().is_none());
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
fn auto_register() {
    #[derive(Clone)]
    #[Singleton]
    struct A;

    #[derive(Clone)]
    #[Singleton(auto_register = false)]
    struct B;

    let cx = Context::auto_register();

    assert!(cx.get_provider::<A>().is_some());
    assert!(cx.get_provider::<B>().is_none());
}

#[test]
fn no_async() {
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
fn panicky_async() {
    #[Singleton]
    async fn One() -> i32 {
        1
    }

    #[derive(Clone)]
    #[Singleton(async)]
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
async fn successful_async() {
    #[Singleton]
    async fn One() -> i32 {
        1
    }

    #[derive(Clone)]
    #[Singleton(async)]
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

#[cfg(test)]
mod tests {
    use crate::ru_di::{components, modules, Context, DynProvider, Module, Transient};

    #[test]
    fn rudi_path() {
        #[Transient(rudi_path = crate::ru_di)]
        struct A;

        struct MyModule;

        impl Module for MyModule {
            fn providers() -> Vec<DynProvider> {
                components![A]
            }
        }

        let mut cx = Context::create(modules![MyModule]);
        assert!(cx.resolve_option::<A>().is_some());
    }
}
