use std::rc::Rc;

use rudi::{modules, providers, singleton, Context, Module};

trait Service {
    fn hello(&self) -> &str;
}

#[derive(Clone)]
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
struct Controller {
    s: Rc<dyn Service>,
}

impl Controller {
    fn hello(&self) -> &str {
        self.s.hello()
    }
}

#[derive(Clone)]
struct Hello;

impl Hello {
    fn new() -> Hello {
        println!("Hello::new");
        Hello
    }
}

fn run(controller: Controller, num: i32, success: bool, _: Hello) {
    println!("{}", controller.hello());

    println!("i32: {}", num);

    println!("bool: {}", success);
}

struct MyModule;

impl Module for MyModule {
    fn providers() -> Vec<rudi::DynProvider> {
        providers![
            singleton(|_| ServiceImpl)
                .name("hello")
                .bind(ServiceImpl::into_service),
            singleton(|cx| Controller {
                s: cx.resolve_with_name("hello")
            })
            .name("controller"),
            singleton(|_| Hello::new()),
            singleton(|cx| run(
                cx.resolve_with_name("controller"),
                cx.resolve(),
                cx.resolve(),
                cx.resolve(),
            ))
        ]
    }
}

fn main() {
    let mut cx = Context::options()
        .instance(42)
        .instance(true)
        .create(modules![MyModule]);

    // cx.resolve::<()>();
    cx.resolve()
}
