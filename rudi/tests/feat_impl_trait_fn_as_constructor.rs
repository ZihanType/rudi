use rudi::{components, modules, Context, DynProvider, Module, Singleton};

#[test]
fn test() {
    #[derive(Clone)]
    #[Singleton]
    struct Material;

    trait FromMaterial {
        fn from_material(m: &Material) -> Self;
    }

    #[derive(Clone)]
    struct Product;

    #[Singleton]
    impl FromMaterial for Product {
        #[di]
        fn from_material(#[di(ref)] _: &Material) -> Self {
            Product
        }
    }

    struct MyModule;

    impl Module for MyModule {
        fn providers() -> Vec<DynProvider> {
            components![Material, Product]
        }
    }

    let mut cx = Context::create(modules![MyModule]);

    assert!(cx.resolve_option::<Product>().is_some());
}
