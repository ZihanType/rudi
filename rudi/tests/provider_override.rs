mod components;

use std::{any::TypeId, rc::Rc};

use rudi::{modules, providers, singleton, singleton_async, Context, FutureExt, Module};

use crate::components::{Component1, Component2, Trait1};

#[test]
fn allow_override_by_type() {
    struct Module1;
    impl Module for Module1 {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![singleton(|_| Component2).bind(Component2::into_trait1)]
        }
    }

    struct Module2;
    impl Module for Module2 {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![singleton(|_| Component1).bind(Component1::into_trait1)]
        }
    }

    let mut cx = Context::create(modules![Module1, Module2]);
    // Component1, Component2, Rc<dyn Trait1>
    assert_eq!(cx.provider_registry().len(), 3);

    let instance = cx.resolve::<Rc<dyn Trait1>>();
    assert!(instance.as_any().is::<Component1>());

    let provider = cx.get_provider::<Rc<dyn Trait1>>();
    assert!(provider.is_some());
    assert!(
        provider.unwrap().definition().origin.as_ref().unwrap().id == TypeId::of::<Component1>()
    );
}

#[test]
fn allow_override_by_name() {
    struct Module1;
    impl Module for Module1 {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![singleton(|_| Component2)
                .name("hello")
                .bind(Component2::into_trait1)]
        }
    }

    struct Module2;
    impl Module for Module2 {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![singleton(|_| Component1)
                .name("hello")
                .bind(Component1::into_trait1)]
        }
    }

    let mut cx = Context::create(modules![Module1, Module2]);
    // Component1, Component2, Rc<dyn Trait1>
    assert_eq!(cx.provider_registry().len(), 3);

    let instance = cx.resolve_with_name::<Rc<dyn Trait1>>("hello");
    assert!(instance.as_any().is::<Component1>());
    let provider = cx.get_provider_with_name::<Rc<dyn Trait1>>("hello");
    assert!(provider.is_some());
    assert!(
        provider.unwrap().definition().origin.as_ref().unwrap().id == TypeId::of::<Component1>()
    );
}

#[tokio::test]
async fn allow_override_by_type_async() {
    struct Module1;
    impl Module for Module1 {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![
                singleton_async(|_| async { Component2 }.boxed()).bind(Component2::into_trait1)
            ]
        }
    }

    struct Module2;
    impl Module for Module2 {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![
                singleton_async(|_| async { Component1 }.boxed()).bind(Component1::into_trait1)
            ]
        }
    }

    let mut cx = Context::create(modules![Module1, Module2]);
    // Component1, Component2, Rc<dyn Trait1>
    assert_eq!(cx.provider_registry().len(), 3);

    let instance = cx.resolve_async::<Rc<dyn Trait1>>().await;
    assert!(instance.as_any().is::<Component1>());

    let provider = cx.get_provider::<Rc<dyn Trait1>>();
    assert!(provider.is_some());
    assert!(
        provider.unwrap().definition().origin.as_ref().unwrap().id == TypeId::of::<Component1>()
    );
}

#[tokio::test]
async fn allow_override_by_name_async() {
    struct Module1;
    impl Module for Module1 {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![singleton_async(|_| async { Component2 }.boxed())
                .name("hello")
                .bind(Component2::into_trait1)]
        }
    }

    struct Module2;
    impl Module for Module2 {
        fn providers() -> Vec<rudi::DynProvider> {
            providers![singleton_async(|_| async { Component1 }.boxed())
                .name("hello")
                .bind(Component1::into_trait1)]
        }
    }

    let mut cx = Context::create(modules![Module1, Module2]);
    // Component1, Component2, Rc<dyn Trait1>
    assert_eq!(cx.provider_registry().len(), 3);

    let instance = cx.resolve_with_name_async::<Rc<dyn Trait1>>("hello").await;
    assert!(instance.as_any().is::<Component1>());
    let provider = cx.get_provider_with_name::<Rc<dyn Trait1>>("hello");
    assert!(provider.is_some());
    assert!(
        provider.unwrap().definition().origin.as_ref().unwrap().id == TypeId::of::<Component1>()
    );
}
