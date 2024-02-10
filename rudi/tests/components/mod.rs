use std::{any::Any, rc::Rc};

use rudi::Singleton;

#[derive(Clone, Debug)]
#[Singleton]
pub(crate) struct ComponentA;

#[derive(Clone)]
#[Singleton]
pub(crate) struct ComponentB {
    #[allow(dead_code)]
    pub(crate) a: ComponentA,
}

pub(crate) trait Trait1 {
    #[allow(dead_code)] // false positive
    fn as_any(&self) -> &dyn Any;
}

pub(crate) trait Trait2 {}

#[derive(Clone)]
#[Singleton(binds = [Self::into_trait1, Self::into_trait2])]
pub(crate) struct Component1;

impl Component1 {
    pub(crate) fn into_trait1(self) -> Rc<dyn Trait1> {
        Rc::new(self)
    }

    fn into_trait2(self) -> Rc<dyn Trait2> {
        Rc::new(self)
    }
}

impl Trait1 for Component1 {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Trait2 for Component1 {}

#[derive(Clone)]
#[Singleton(binds = [Self::into_trait1])]
pub(crate) struct Component2;

impl Component2 {
    pub(crate) fn into_trait1(self) -> Rc<dyn Trait1> {
        Rc::new(self)
    }
}

impl Trait1 for Component2 {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[allow(dead_code)] // false positive
#[derive(Clone)]
pub(crate) struct Holder {
    pub(crate) id: usize,
}
