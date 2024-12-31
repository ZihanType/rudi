use leptos::prelude::*;
use rudi::{components, modules, Context, DynProvider, Module, Singleton};

#[component]
pub fn SimpleCounter(initial_value: i32, step: i32) -> impl IntoView {
    let value = RwSignal::new(initial_value);

    view! {
        <div>
            <button on:click=move |_| value.set(0)>"Clear"</button>
            <button on:click=move |_| value.update(|value| *value -= step)>"-1"</button>
            <span>"Value: " { move || value.get() } "!"</span>
            <button on:click=move |_| value.update(|value| *value += step)>"+1"</button>
        </div>
    }
}

#[Singleton]
fn Number() -> i32 {
    42
}

#[Singleton]
fn Run(initial_value: i32) {
    mount_to_body(move || {
        view! {
            <SimpleCounter
                initial_value=initial_value
                step=1
            />
        }
    })
}

struct MyModule;

impl Module for MyModule {
    fn providers() -> Vec<DynProvider> {
        components![Number, Run]
    }
}

fn main() {
    let mut cx = Context::create(modules![MyModule]);
    cx.resolve()
}
