use std::rc::Rc;

use rudi::{components, modules, Context, DynProvider, Module, Singleton};

trait Service {
    fn hello(&self) -> &str;
}

#[derive(Clone)]
#[Singleton(name = "hello", binds = [Self::into_service])]
struct ServiceImpl;

impl ServiceImpl {
    fn into_service(self) -> Rc<dyn Service> {
        Rc::new(self)
    }
}

impl Service for ServiceImpl {
    fn hello(&self) -> &str {
        "Hello World!"
    }
}

#[derive(Clone)]
#[Singleton(name = "controller")]
struct Controller {
    #[di(name = "hello")]
    s: Rc<dyn Service>,
}

impl Controller {
    fn hello(&self) -> &str {
        self.s.hello()
    }
}

#[derive(Clone)]
struct Hello;

#[Singleton]
impl Hello {
    #[di]
    fn new() -> Hello {
        println!("Hello::new");
        Hello
    }
}

#[Singleton]
fn Run(#[di(name = "controller")] controller: Controller, num: i32, success: bool, _: Hello) {
    println!("{}", controller.hello());

    println!("num: {}", num);

    println!("success: {}", success);
}

struct MyModule;

impl Module for MyModule {
    fn providers() -> Vec<DynProvider> {
        components![ServiceImpl, Controller, Hello, Run]
    }
}

fn main() {
    let mut cx = Context::options()
        .singleton(42)
        .singleton(true)
        .create(modules![MyModule]);

    // cx.resolve::<()>();
    cx.resolve()
}
