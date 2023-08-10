# Rudi

[![Crates.io version](https://img.shields.io/crates/v/rudi.svg?style=flat-square)](https://crates.io/crates/rudi)
[![docs.rs docs](https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square)](https://docs.rs/rudi)

Rudi - an out-of-the-box dependency injection framework for Rust.

```rust
use rudi::{Context, Singleton, Transient};

// Register `fn(cx) -> A { A }` as the constructor for `A`
#[derive(Debug)]
#[Transient]
struct A;

#[derive(Debug)]
struct B(A);

// Register `fn(cx) -> B { B(cx.resolve::<A>()) }` as the constructor for `B`
#[Transient]
impl B {
    fn new(a: A) -> B {
        B(a)
    }
}

// Register `fn(cx) -> () { Run(cx.resolve::<B>()) }` as the constructor for `()`
#[Singleton]
fn Run(b: B) {
    println!("{:?}", b);
}

fn main() {
    // Automatically register all types and functions with the `#[Singleton]` or `#[Transient]` attribute.
    let mut cx = Context::auto_register();

    // Get an instance of `()` from the `Context`, which will call the `Run` function.
    // This is equivalent to `cx.resolve::<()>();`
    cx.resolve()
}
```

## Features

- Two lifetimes: `singleton` and `transient`.
- Async functions and async constructors.
- Manual and automatic registration (thanks to [inventory](https://github.com/dtolnay/inventory)).
- Easy binding of trait implementations and trait objects.
- Distinguishing different instances with types and names.
- Generics (but must be monomorphized and manually registered)

## More complex example

```rust
use std::{fmt::Debug, rc::Rc};

use rudi::{Context, Singleton, Transient};

// Register async function and specify name
#[Singleton(name = "number")]
async fn Number() -> i32 {
    42
}

#[derive(Debug, Clone)]
#[Singleton(async_constructor, name = "foo")] // Register async constructor and specify name
struct Foo {
    #[di("number")] // Specify the name of the dependency
    number: i32,
}

#[derive(Debug)]
struct Bar(Foo);

impl Bar {
    fn into_debug(self) -> Rc<dyn Debug> {
        Rc::new(self)
    }
}

#[Transient(binds = [Self::into_debug])] // Bind the implementation of the `Debug` trait and the trait object of the `Debug` trait
impl Bar {
    async fn new(#[di("foo")] f: Foo) -> Bar { // Register async constructor
        Bar(f)
    }
}

#[Singleton]
async fn run(bar: Bar, debug: Rc<dyn Debug>, #[di("foo")] f: Foo) {
    println!("{:?}", bar);
    assert_eq!(format!("{:?}", bar), format!("{:?}", debug));
    assert_eq!(format!("{:?}", bar.0.number), format!("{:?}", f.number));
}

#[tokio::main]
async fn main() {
    let mut cx = Context::auto_register();

    cx.resolve_async().await
}
```

More examples can be found in the [examples](./examples/) and [tests](./rudi/tests/) directories.

## Credits

- [Koin](https://github.com/InsertKoinIO/koin): This project's API design and test cases were inspired by Koin.
- [inventory](https://github.com/dtolnay/inventory): This project uses inventory to implement automatic registration, making Rust's automatic registration very simple.

## Contributing

Thanks for your help improving the project! We are so happy to have you!

## License

Licensed under either of

- Apache License, Version 2.0,([LICENSE-APACHE](./LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](./LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
  at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
