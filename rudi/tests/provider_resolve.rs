mod components;

use std::{any::TypeId, rc::Rc};

use rudi::{
    components, modules, providers, singleton, singleton_async, transient, transient_async,
    Context, FutureExt, Module, Transient,
};

use crate::components::{Component1, Component2, ComponentA, Trait1};

#[test]
fn resolve_singleton() {
    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![singleton(|_| Rc::new(ComponentA))]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    let a = cx.resolve::<Rc<ComponentA>>();
    let b = cx.resolve::<Rc<ComponentA>>();

    assert!(std::ptr::eq(&*a, &*b));
}

#[test]
fn resolve_singleton_with_dyn_trait1() {
    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![
                singleton(|_| Component1)
                    .name("1")
                    .bind(Component1::into_trait1),
                singleton(|_| Component2)
                    .name("2")
                    .bind(Component2::into_trait1)
            ]
        }
    }

    let cx = Context::create(modules![MyModule]);

    let providers = cx.get_providers_by_type::<Rc<dyn Trait1>>();

    assert!(providers.len() == 2);

    assert!(providers.iter().any(|provider| {
        provider.definition().origin.as_ref().unwrap().id == TypeId::of::<Component1>()
    }));

    assert!(providers.iter().any(|provider| {
        provider.definition().origin.as_ref().unwrap().id == TypeId::of::<Component2>()
    }));
}

#[test]
fn resolve_singleton_by_name() {
    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            let a = Rc::new(ComponentA);
            providers![
                singleton({
                    let a = a.clone();
                    move |_| a.clone()
                })
                .name("A"),
                singleton(move |_| a.clone()).name("B"),
            ]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    let a = cx.resolve_with_name::<Rc<ComponentA>>("A");
    let b = cx.resolve_with_name::<Rc<ComponentA>>("B");

    assert!(std::ptr::eq(&*a, &*b));
}

#[test]
fn resolve_transient_by_name() {
    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            let a = Rc::new(ComponentA);
            providers![
                transient({
                    let a = a.clone();
                    move |_| a.clone()
                })
                .name("A"),
                transient(move |_| a.clone()).name("B"),
            ]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    let a = cx.resolve_with_name::<Rc<ComponentA>>("A");
    let b = cx.resolve_with_name::<Rc<ComponentA>>("B");

    assert!(std::ptr::eq(&*a, &*b));
}

#[test]
fn resolve_transient() {
    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![transient(|_| Rc::new(ComponentA))]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    let a = cx.resolve::<Rc<ComponentA>>();
    let b = cx.resolve::<Rc<ComponentA>>();

    assert!(!std::ptr::eq(&*a, &*b));
}

#[test]
fn resolve_default_and_named() {
    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![
                singleton(|_| Component1).bind(Component1::into_trait1),
                singleton(|_| Component2)
                    .name("2")
                    .bind(Component2::into_trait1),
            ]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    let a = cx.resolve::<Rc<dyn Trait1>>();
    let b = cx.resolve_with_name::<Rc<dyn Trait1>>("2");

    let a_provider = cx.get_provider::<Rc<dyn Trait1>>();
    let b_provider = cx.get_provider_with_name::<Rc<dyn Trait1>>("2");

    assert!(a_provider.is_some());
    assert!(
        a_provider.unwrap().definition().origin.as_ref().unwrap().id == TypeId::of::<Component1>()
    );

    assert!(b_provider.is_some());
    assert!(
        b_provider.unwrap().definition().origin.as_ref().unwrap().id == TypeId::of::<Component2>()
    );

    assert!(a.as_any().is::<Component1>());
    assert!(b.as_any().is::<Component2>());
}

#[test]
#[should_panic]
fn resolve_async_instance_in_sync_context() {
    #[Transient(async)]
    struct A;

    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            components![A]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    cx.resolve::<A>();
}

#[test]
fn resolve_instances_by_type() {
    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![
                transient(|_| ComponentA).name("A"),
                transient(|_| ComponentA).name("B"),
            ]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    assert!(cx.resolve_by_type::<ComponentA>().len() == 2);
}

#[tokio::test]
async fn resolve_singleton_async() {
    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![singleton_async(|_| async { Rc::new(ComponentA) }.boxed())]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    let a = cx.resolve_async::<Rc<ComponentA>>().await;
    let b = cx.resolve_async::<Rc<ComponentA>>().await;

    assert!(std::ptr::eq(&*a, &*b));
}

#[tokio::test]
async fn resolve_singleton_with_dyn_trait1_async() {
    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![
                singleton_async(|_| async { Component1 }.boxed())
                    .name("1")
                    .bind(Component1::into_trait1),
                singleton_async(|_| async { Component2 }.boxed())
                    .name("2")
                    .bind(Component2::into_trait1)
            ]
        }
    }

    let cx = Context::create(modules![MyModule]);

    let providers = cx.get_providers_by_type::<Rc<dyn Trait1>>();

    assert!(providers.len() == 2);

    assert!(providers.iter().any(|provider| {
        provider.definition().origin.as_ref().unwrap().id == TypeId::of::<Component1>()
    }));

    assert!(providers.iter().any(|provider| {
        provider.definition().origin.as_ref().unwrap().id == TypeId::of::<Component2>()
    }));
}

#[tokio::test]
async fn resolve_singleton_by_name_async() {
    thread_local! {
        static A: Rc<ComponentA> = Rc::new(ComponentA);
    }

    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![
                singleton_async(move |_| async { A.with(|a| a.clone()) }.boxed()).name("A"),
                singleton_async(move |_| async { A.with(|a| a.clone()) }.boxed()).name("B"),
            ]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    let a = cx.resolve_with_name_async::<Rc<ComponentA>>("A").await;
    let b = cx.resolve_with_name_async::<Rc<ComponentA>>("B").await;

    assert!(std::ptr::eq(&*a, &*b));
}

#[tokio::test]
async fn resolve_transient_by_name_async() {
    thread_local! {
        static A: Rc<ComponentA> = Rc::new(ComponentA);
    }

    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![
                transient_async(move |_| async { A.with(|a| a.clone()) }.boxed()).name("A"),
                transient_async(move |_| async { A.with(|a| a.clone()) }.boxed()).name("B"),
            ]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    let a = cx.resolve_with_name_async::<Rc<ComponentA>>("A").await;
    let b = cx.resolve_with_name_async::<Rc<ComponentA>>("B").await;

    assert!(std::ptr::eq(&*a, &*b));
}

#[tokio::test]
async fn resolve_transient_async() {
    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![transient_async(|_| async { Rc::new(ComponentA) }.boxed())]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    let a = cx.resolve_async::<Rc<ComponentA>>().await;
    let b = cx.resolve_async::<Rc<ComponentA>>().await;

    assert!(!std::ptr::eq(&*a, &*b));
}

#[tokio::test]
async fn resolve_default_and_named_async() {
    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![
                singleton_async(|_| async { Component1 }.boxed()).bind(Component1::into_trait1),
                singleton_async(|_| async { Component2 }.boxed())
                    .name("2")
                    .bind(Component2::into_trait1),
            ]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    let a = cx.resolve_async::<Rc<dyn Trait1>>().await;
    let b = cx.resolve_with_name_async::<Rc<dyn Trait1>>("2").await;

    let a_provider = cx.get_provider::<Rc<dyn Trait1>>();
    let b_provider = cx.get_provider_with_name::<Rc<dyn Trait1>>("2");

    assert!(a_provider.is_some());
    assert!(
        a_provider.unwrap().definition().origin.as_ref().unwrap().id == TypeId::of::<Component1>()
    );

    assert!(b_provider.is_some());
    assert!(
        b_provider.unwrap().definition().origin.as_ref().unwrap().id == TypeId::of::<Component2>()
    );

    assert!(a.as_any().is::<Component1>());
    assert!(b.as_any().is::<Component2>());
}

#[tokio::test]
async fn resolve_sync_instance_in_async_context() {
    #[Transient]
    struct A;

    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            components![A]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    cx.resolve_async::<A>().await;
}

#[tokio::test]
async fn resolve_instances_by_type_async() {
    struct MyModule;
    impl Module for MyModule {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![
                transient_async(|_| async { ComponentA }.boxed()).name("A"),
                transient_async(|_| async { ComponentA }.boxed()).name("B"),
            ]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    assert!(cx.resolve_by_type_async::<ComponentA>().await.len() == 2);
}
