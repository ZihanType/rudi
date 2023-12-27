use std::marker::PhantomData;

use rudi::{components, modules, Context, DynProvider, Module, Singleton};

trait Service {
    fn hello(&self) -> &str;
}

#[derive(Clone)]
#[Singleton]
struct ServiceImpl;

impl Service for ServiceImpl {
    fn hello(&self) -> &str {
        "Hello World!"
    }
}

#[derive(Clone)]
#[Singleton]
struct Controller<T>
where
    T: Clone + 'static,
{
    s: T,
}

impl<T> Controller<T>
where
    T: Service + Clone,
{
    fn hello(&self) -> &str {
        self.s.hello()
    }
}

#[Singleton]
fn Run<T>(controller: Controller<T>)
where
    T: Service + Clone + 'static,
{
    println!("{}", controller.hello());
}

struct MyModule<T>(PhantomData<T>);

impl<T> Module for MyModule<T>
where
    T: Service + Clone + 'static,
{
    fn providers() -> Vec<DynProvider> {
        components![ServiceImpl, Controller<T>, Run<T>]
    }
}

fn main() {
    let mut cx = Context::create(modules![MyModule<ServiceImpl>]);

    // cx.resolve::<()>();
    cx.resolve()
}
