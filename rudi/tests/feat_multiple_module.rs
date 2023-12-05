mod components;

use std::rc::Rc;

use rudi::{
    components, modules, providers, singleton, singleton_async, Context, DynProvider, FutureExt,
    Module, ResolveModule, Singleton,
};

use crate::components::{ComponentA, ComponentB};

#[test]
fn resolve_with_several_modules() {
    struct MyModule1;

    impl Module for MyModule1 {
        fn providers() -> Vec<DynProvider> {
            providers![singleton(|_| Rc::new(ComponentA))]
        }
    }

    #[derive(Clone)]
    #[Singleton]
    struct Holder(Rc<ComponentA>);

    struct MyModule2;
    impl Module for MyModule2 {
        fn providers() -> Vec<DynProvider> {
            components![Holder]
        }
    }

    let mut cx = Context::create(modules![MyModule1, MyModule2]);
    let a = cx.resolve::<Rc<ComponentA>>();
    let b = cx.resolve::<Holder>();
    assert!(std::ptr::eq(&*a, &*b.0));
}

#[test]
fn single_module() {
    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![ComponentA]
        }
    }

    let cx = Context::create(modules![MyModule]);
    assert_eq!(cx.provider_registry().len(), 1);
}

#[test]
fn multiple_module() {
    struct MyModule1;

    impl Module for MyModule1 {
        fn providers() -> Vec<DynProvider> {
            components![ComponentA]
        }
    }

    struct MyModule2;

    impl Module for MyModule2 {
        fn providers() -> Vec<DynProvider> {
            components![ComponentB]
        }
    }

    let cx = Context::create(modules![MyModule1, MyModule2]);
    assert_eq!(cx.provider_registry().len(), 2);
}

#[test]
fn nested_module() {
    struct MyModule1;

    impl Module for MyModule1 {
        fn providers() -> Vec<DynProvider> {
            components![ComponentA]
        }
    }

    struct MyModule2;

    impl Module for MyModule2 {
        fn submodules() -> Option<Vec<ResolveModule>> {
            Some(modules![MyModule1])
        }

        fn providers() -> Vec<DynProvider> {
            components![ComponentB]
        }
    }

    let cx = Context::create(modules![MyModule2]);
    assert_eq!(cx.provider_registry().len(), 2);
}

#[test]
fn duplicate_nested_module() {
    struct DataModule;

    impl Module for DataModule {
        fn providers() -> Vec<DynProvider> {
            components![ComponentA]
        }
    }

    struct DomainModule;

    impl Module for DomainModule {
        fn providers() -> Vec<DynProvider> {
            components![ComponentB]
        }
    }

    struct FeatureModule1;

    impl Module for FeatureModule1 {
        fn submodules() -> Option<Vec<ResolveModule>> {
            Some(modules![DomainModule, DataModule])
        }

        fn providers() -> Vec<DynProvider> {
            components![]
        }
    }

    struct FeatureModule2;

    impl Module for FeatureModule2 {
        fn submodules() -> Option<Vec<ResolveModule>> {
            Some(modules![DomainModule, DataModule])
        }

        fn providers() -> Vec<DynProvider> {
            components![]
        }
    }

    let cx = Context::create(modules![FeatureModule1, FeatureModule2]);
    assert_eq!(cx.provider_registry().len(), 2);
}

#[tokio::test]
async fn resolve_with_several_modules_async() {
    struct MyModule1;

    impl Module for MyModule1 {
        fn providers() -> Vec<DynProvider> {
            providers![singleton_async(|_| async { Rc::new(ComponentA) }.boxed())]
        }
    }

    #[derive(Clone)]
    #[Singleton(async)]
    struct Holder(Rc<ComponentA>);

    struct MyModule2;
    impl Module for MyModule2 {
        fn providers() -> Vec<DynProvider> {
            components![Holder]
        }
    }

    let mut cx = Context::create(modules![MyModule1, MyModule2]);
    let a = cx.resolve_async::<Rc<ComponentA>>().await;
    let b = cx.resolve_async::<Holder>().await;
    assert!(std::ptr::eq(&*a, &*b.0));
}

#[tokio::test]
async fn single_module_async() {
    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            providers![singleton_async(|_| async { ComponentA }.boxed())]
        }
    }

    let cx = Context::create(modules![MyModule]);
    assert_eq!(cx.provider_registry().len(), 1);
}

#[tokio::test]
async fn multiple_module_async() {
    struct MyModule1;

    impl Module for MyModule1 {
        fn providers() -> Vec<DynProvider> {
            providers![singleton_async(|_| async { ComponentA }.boxed())]
        }
    }

    struct MyModule2;

    impl Module for MyModule2 {
        fn providers() -> Vec<DynProvider> {
            providers![singleton_async(|cx| async {
                ComponentB {
                    a: cx.resolve_async().await,
                }
            }
            .boxed())]
        }
    }

    let cx = Context::create(modules![MyModule1, MyModule2]);
    assert_eq!(cx.provider_registry().len(), 2);
}

#[tokio::test]
async fn nested_module_async() {
    struct MyModule1;

    impl Module for MyModule1 {
        fn providers() -> Vec<DynProvider> {
            providers![singleton_async(|_| async { ComponentA }.boxed())]
        }
    }

    struct MyModule2;

    impl Module for MyModule2 {
        fn submodules() -> Option<Vec<ResolveModule>> {
            Some(modules![MyModule1])
        }

        fn providers() -> Vec<DynProvider> {
            providers![singleton_async(|cx| async {
                ComponentB {
                    a: cx.resolve_async().await,
                }
            }
            .boxed())]
        }
    }

    let cx = Context::create(modules![MyModule2]);
    assert_eq!(cx.provider_registry().len(), 2);
}

#[tokio::test]
async fn duplicate_nested_module_async() {
    struct DataModule;

    impl Module for DataModule {
        fn providers() -> Vec<DynProvider> {
            providers![singleton_async(|_| async { ComponentA }.boxed())]
        }
    }

    struct DomainModule;

    impl Module for DomainModule {
        fn providers() -> Vec<DynProvider> {
            providers![singleton_async(|cx| async {
                ComponentB {
                    a: cx.resolve_async().await,
                }
            }
            .boxed())]
        }
    }

    struct FeatureModule1;

    impl Module for FeatureModule1 {
        fn submodules() -> Option<Vec<ResolveModule>> {
            Some(modules![DomainModule, DataModule])
        }

        fn providers() -> Vec<DynProvider> {
            components![]
        }
    }

    struct FeatureModule2;

    impl Module for FeatureModule2 {
        fn submodules() -> Option<Vec<ResolveModule>> {
            Some(modules![DomainModule, DataModule])
        }

        fn providers() -> Vec<DynProvider> {
            components![]
        }
    }

    let cx = Context::create(modules![FeatureModule1, FeatureModule2]);
    assert_eq!(cx.provider_registry().len(), 2);
}
