use std::{collections::HashMap, env, fmt::Debug, rc::Rc};

use rudi::{Context, Singleton, Transient};

#[derive(Clone)]
struct Environment {
    map: HashMap<String, String>,
}

#[Singleton(eager_create)]
impl Environment {
    fn new() -> Self {
        Self {
            map: env::vars().collect(),
        }
    }
}

trait Service: Debug {}

fn transfer<T: Service + 'static>(t: T) -> Rc<dyn Service> {
    Rc::new(t)
}

fn condition(cx: &Context, value: &str) -> bool {
    cx.get_single::<Environment>()
        .map
        .get("env")
        .map(|a| a == value)
        .unwrap_or(false)
}

#[derive(Debug)]
#[Transient(condition = |cx| condition(cx, "dev"), binds = [transfer])]
struct A;

impl Service for A {}

#[derive(Debug)]
#[Transient(condition = |cx| condition(cx, "prod"), binds = [transfer])]
struct B;

impl Service for B {}

fn main() {
    env::set_var("env", "dev");
    let mut cx = Context::auto_register();
    let svc = cx.resolve::<Rc<dyn Service>>();
    assert_eq!(format!("{:?}", svc), "A");

    env::set_var("env", "prod");
    let mut cx = Context::auto_register();
    let svc = cx.resolve::<Rc<dyn Service>>();
    assert_eq!(format!("{:?}", svc), "B");
}
