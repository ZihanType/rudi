use std::{any::Any, rc::Rc};

use rudi::Singleton;

#[derive(Clone, Debug)]
#[Singleton]
pub struct ComponentA;

#[derive(Clone)]
#[Singleton]
pub struct ComponentB {
    pub a: ComponentA,
}

pub trait Trait1 {
    fn as_any(&self) -> &dyn Any;
}

pub trait Trait2 {}

#[derive(Clone)]
#[Singleton(binds = [Self::into_trait1, Self::into_trait2])]
pub struct Component1;

impl Component1 {
    pub fn into_trait1(self) -> Rc<dyn Trait1> {
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
pub struct Component2;

impl Component2 {
    pub fn into_trait1(self) -> Rc<dyn Trait1> {
        Rc::new(self)
    }
}

impl Trait1 for Component2 {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone)]
pub struct Holder {
    pub id: usize,
}
