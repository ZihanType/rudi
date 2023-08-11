mod di;
mod self_components;
mod third_components;

use rudi::{Context, Singleton};
use self_components::Controller;
use third_components::Middleware;

#[allow(unused_variables)]
#[Singleton]
async fn Run(
    #[di(name = di::LOG_NAME)] (): (),
    #[di(name = di::migrator_name())] (): (),
    controller: Controller,
    middleware: Middleware,
) {
    println!("Hello, world!")
}

#[tokio::main]
async fn main() {
    let mut cx = Context::auto_register();

    cx.resolve_async().await
}
