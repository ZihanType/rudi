use std::{cell::RefCell, rc::Rc};

use rudi::{components, modules, Context, DynProvider, Module, Singleton};

thread_local! {
    static COUNT: RefCell<u32> = RefCell::new(0);
    static NAME: RefCell<&'static str> = RefCell::new("");
}

trait Trait {}

#[derive(Clone)]
struct A;

impl Trait for A {}

#[Singleton(eager_create)]
fn NewA() -> Rc<dyn Trait> {
    COUNT.with_borrow_mut(|c| {
        *c += 1;
    });

    NAME.set("A");

    Rc::new(A)
}

#[derive(Clone)]
struct B;

impl Trait for B {}

#[Singleton(eager_create)]
fn NewB() -> Rc<dyn Trait> {
    COUNT.with_borrow_mut(|c| {
        *c += 1;
    });

    NAME.set("B");

    Rc::new(B)
}

#[test]
fn test() {
    struct MyModule1;

    impl Module for MyModule1 {
        fn providers() -> Vec<DynProvider> {
            components![NewA]
        }
    }

    struct MyModule2;

    impl Module for MyModule2 {
        fn providers() -> Vec<DynProvider> {
            components![NewB]
        }
    }

    Context::create(modules![MyModule1, MyModule2]);

    assert!(COUNT.with_borrow(|c| *c == 1));
    assert!(NAME.with_borrow(|n| *n == "B"));
}
