use rudi as ru_di;

#[cfg(test)]
mod tests {
    use crate::ru_di::{components, modules, Context, DynProvider, Module, Transient};

    #[test]
    fn one() {
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
