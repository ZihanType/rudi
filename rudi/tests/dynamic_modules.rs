mod components;

use std::rc::Rc;

use rudi::{
    components, modules, providers, singleton, singleton_async, transient, transient_async,
    Context, FutureExt, Module, Scope, Singleton, Transient,
};

use crate::components::{Component1, Holder, Trait1};

#[test]
fn empty_module() {
    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![]
        }
    }

    let mut cx = Context::create(modules![]);
    assert!(cx.providers_len() == 0);
    assert!(cx.singletons_len() == 0);

    cx.unload_modules(modules![]);
    assert!(cx.providers_len() == 0);
    assert!(cx.singletons_len() == 0);

    cx.load_modules(modules![MyModule]);
    assert!(cx.providers_len() == 0);
    assert!(cx.singletons_len() == 0);

    cx.unload_modules(modules![MyModule]);
    assert!(cx.providers_len() == 0);
    assert!(cx.singletons_len() == 0);
}

#[test]
fn unload_singleton() {
    #[derive(Clone)]
    #[Singleton]
    struct A;

    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            components![A]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    let provider = cx.get_provider::<A>();
    assert!(provider.is_some());
    assert!(provider.unwrap().definition().scope == Scope::Singleton);
    assert!(cx.resolve_option::<A>().is_some());

    cx.unload_modules(modules![MyModule]);

    assert!(cx.get_provider::<A>().is_none());
    assert!(cx.resolve_option::<A>().is_none());
}

#[test]
fn unload_singleton_with_bind() {
    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![singleton(|_| Component1).bind(Component1::into_trait1)]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    assert!(cx.get_provider::<Rc<dyn Trait1>>().is_some());

    let provider = cx.get_provider::<Component1>();
    assert!(provider.is_some());
    assert!(provider.unwrap().definition().scope == Scope::Singleton);

    assert!(cx.resolve_option::<Component1>().is_some());
    assert!(cx.resolve_option::<Rc<dyn Trait1>>().is_some());

    cx.unload_modules(modules![MyModule]);

    assert!(cx.get_provider::<Component1>().is_none());

    assert!(cx.resolve_option::<Component1>().is_none());
    assert!(cx.resolve_option::<Rc<dyn Trait1>>().is_none());
}

#[test]
fn unload_module() {
    #[Transient]
    struct A;

    #[Transient]
    struct B;

    struct Module1;
    impl Module for Module1 {
        fn providers() -> Vec<rudi::DynProvider> {
            components![A]
        }
    }

    struct Module2;
    impl Module for Module2 {
        fn providers() -> Vec<rudi::DynProvider> {
            components![B]
        }
    }

    let mut cx = Context::create(modules![Module1, Module2]);

    assert!(cx.get_provider::<A>().is_some());
    assert!(cx.get_provider::<B>().is_some());

    assert!(cx.resolve_option::<A>().is_some());
    assert!(cx.resolve_option::<B>().is_some());

    cx.unload_modules(modules![Module2]);

    assert!(cx.get_provider::<B>().is_none());
    assert!(cx.resolve_option::<B>().is_none());
}

#[test]
fn unload_module_with_transient() {
    #[Transient]
    struct A;

    #[Transient]
    struct B(A);

    struct Module1;
    impl Module for Module1 {
        fn providers() -> Vec<rudi::DynProvider> {
            components![A]
        }
    }

    struct Module2;
    impl Module for Module2 {
        fn providers() -> Vec<rudi::DynProvider> {
            components![B]
        }
    }

    let mut cx = Context::create(modules![Module1, Module2]);

    assert!(cx.get_provider::<A>().is_some());
    assert!(cx.get_provider::<B>().is_some());

    assert!(cx.resolve_option::<A>().is_some());
    assert!(cx.resolve_option::<B>().is_some());

    cx.unload_modules(modules![Module2]);

    assert!(cx.get_provider::<B>().is_none());
    assert!(cx.resolve_option::<B>().is_none());
}

#[test]
fn unload_module_with_override() {
    struct MyModule1;
    impl Module for MyModule1 {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![transient(|_| Holder { id: 42 })]
        }
    }

    struct MyModule2;
    impl Module for MyModule2 {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![transient(|_| Holder { id: 24 })]
        }
    }

    let mut cx = Context::create(modules![MyModule1, MyModule2]);

    assert!(cx.get_provider::<Holder>().is_some());
    assert_eq!(cx.resolve::<Holder>().id, 24);

    cx.unload_modules(modules![MyModule2]);

    assert!(cx.get_provider::<Holder>().is_none());
    assert!(cx.resolve_option::<Holder>().is_none());
}

#[test]
fn reload_module() {
    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![singleton(|_| Holder { id: 42 })]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    assert!(cx.get_provider::<Holder>().is_some());
    assert_eq!(cx.resolve::<Holder>().id, 42);

    cx.unload_modules(modules![MyModule]);
    cx.load_modules(modules![MyModule]);

    assert!(cx.get_provider::<Holder>().is_some());
    assert_eq!(cx.resolve::<Holder>().id, 42);
}

#[tokio::test]
async fn unload_singleton_async() {
    #[derive(Clone)]
    #[Singleton(async_constructor)]
    struct A;

    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            components![A]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    let provider = cx.get_provider::<A>();
    assert!(provider.is_some());
    assert!(provider.unwrap().definition().scope == Scope::Singleton);
    assert!(cx.resolve_option_async::<A>().await.is_some());

    cx.unload_modules(modules![MyModule]);

    assert!(cx.get_provider::<A>().is_none());
    assert!(cx.resolve_option_async::<A>().await.is_none());
}

#[tokio::test]
async fn unload_singleton_with_bind_async() {
    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![
                singleton_async(|_| async { Component1 }.boxed()).bind(Component1::into_trait1)
            ]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    assert!(cx.get_provider::<Rc<dyn Trait1>>().is_some());

    let provider = cx.get_provider::<Component1>();
    assert!(provider.is_some());
    assert!(provider.unwrap().definition().scope == Scope::Singleton);

    assert!(cx.resolve_option_async::<Component1>().await.is_some());
    assert!(cx.resolve_option_async::<Rc<dyn Trait1>>().await.is_some());

    cx.unload_modules(modules![MyModule]);

    assert!(cx.get_provider::<Component1>().is_none());

    assert!(cx.resolve_option_async::<Component1>().await.is_none());
    assert!(cx.resolve_option_async::<Rc<dyn Trait1>>().await.is_none());
}

#[tokio::test]
async fn unload_module_async() {
    #[Transient(async_constructor)]
    struct A;

    #[Transient(async_constructor)]
    struct B;

    struct Module1;
    impl Module for Module1 {
        fn providers() -> Vec<rudi::DynProvider> {
            components![A]
        }
    }

    struct Module2;
    impl Module for Module2 {
        fn providers() -> Vec<rudi::DynProvider> {
            components![B]
        }
    }

    let mut cx = Context::create(modules![Module1, Module2]);

    assert!(cx.get_provider::<A>().is_some());
    assert!(cx.get_provider::<B>().is_some());

    assert!(cx.resolve_option_async::<A>().await.is_some());
    assert!(cx.resolve_option_async::<B>().await.is_some());

    cx.unload_modules(modules![Module2]);

    assert!(cx.get_provider::<B>().is_none());
    assert!(cx.resolve_option_async::<B>().await.is_none());
}

#[tokio::test]
async fn unload_module_with_transient_async() {
    #[Transient(async_constructor)]
    struct A;

    #[Transient(async_constructor)]
    struct B(A);

    struct Module1;
    impl Module for Module1 {
        fn providers() -> Vec<rudi::DynProvider> {
            components![A]
        }
    }

    struct Module2;
    impl Module for Module2 {
        fn providers() -> Vec<rudi::DynProvider> {
            components![B]
        }
    }

    let mut cx = Context::create(modules![Module1, Module2]);

    assert!(cx.get_provider::<A>().is_some());
    assert!(cx.get_provider::<B>().is_some());

    assert!(cx.resolve_option_async::<A>().await.is_some());
    assert!(cx.resolve_option_async::<B>().await.is_some());

    cx.unload_modules(modules![Module2]);

    assert!(cx.get_provider::<B>().is_none());
    assert!(cx.resolve_option_async::<B>().await.is_none());
}

#[tokio::test]
async fn unload_module_with_override_async() {
    struct MyModule1;
    impl Module for MyModule1 {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![transient_async(|_| async { Holder { id: 42 } }.boxed())]
        }
    }

    struct MyModule2;
    impl Module for MyModule2 {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![transient_async(|_| async { Holder { id: 24 } }.boxed())]
        }
    }

    let mut cx = Context::create(modules![MyModule1, MyModule2]);

    assert!(cx.get_provider::<Holder>().is_some());
    assert_eq!(cx.resolve_async::<Holder>().await.id, 24);

    cx.unload_modules(modules![MyModule2]);

    assert!(cx.get_provider::<Holder>().is_none());
    assert!(cx.resolve_option_async::<Holder>().await.is_none());
}

#[tokio::test]
async fn reload_module_async() {
    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![singleton_async(|_| async { Holder { id: 42 } }.boxed())]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    assert!(cx.get_provider::<Holder>().is_some());
    assert_eq!(cx.resolve_async::<Holder>().await.id, 42);

    cx.unload_modules(modules![MyModule]);
    cx.load_modules(modules![MyModule]);

    assert!(cx.get_provider::<Holder>().is_some());
    assert_eq!(cx.resolve_async::<Holder>().await.id, 42);
}
