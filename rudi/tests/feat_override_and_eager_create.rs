use std::{cell::RefCell, rc::Rc};

use rudi::{components, modules, Context, Module, Singleton};

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
    COUNT.with(|c| {
        let mut c = c.borrow_mut();
        *c += 1;
    });

    NAME.with(|n| {
        let mut n = n.borrow_mut();
        *n = "A";
    });

    Rc::new(A)
}

#[derive(Clone)]
struct B;

impl Trait for B {}

#[Singleton(eager_create)]
fn NewB() -> Rc<dyn Trait> {
    COUNT.with(|c| {
        let mut c = c.borrow_mut();
        *c += 1;
    });

    NAME.with(|n| {
        let mut n = n.borrow_mut();
        *n = "B";
    });

    Rc::new(B)
}

#[test]
fn test() {
    struct MyModule1;

    impl Module for MyModule1 {
        fn providers() -> Vec<rudi::DynProvider> {
            components![NewA]
        }
    }

    struct MyModule2;

    impl Module for MyModule2 {
        fn providers() -> Vec<rudi::DynProvider> {
            components![NewB]
        }
    }

    Context::create(modules![MyModule1, MyModule2]);

    assert!(COUNT.with(|c| *c.borrow()) == 1);
    assert!(NAME.with(|n| *n.borrow()) == "B");
}
