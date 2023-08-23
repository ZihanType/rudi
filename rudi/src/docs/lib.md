# Rudi

Rudi - an out-of-the-box dependency injection framework for Rust.

## Quick Links

Here are links to the most important sections of the docs:

- [`Context`](crate::Context): The core of the entire dependency injection framework, responsible for managing all providers.
- [`#[Singleton]`](crate::Singleton) / [`#[Transient]`](crate::Transient): Two attribute macros used to generate the implementation of [`DefaultProvider`](crate::DefaultProvider), thus registering providers.

## Feature Flags

- `auto-register` (*Default*): Enables automatic registration of types and functions with `#[Singleton]` and `#[Transient]` attributes.
- `debug-print`: Enables logging of all providers registered in the `Context` on creation via the [tracing](https://github.com/tokio-rs/tracing) crate.

## Example

```rust
use rudi::{Context, Singleton, Transient};

// Register `fn(cx) -> A { A }` as the constructor for `A`
#[derive(Debug)]
#[Transient]
struct A;

#[derive(Debug)]
struct B(A);

// Register `fn(cx) -> B { B::new(cx.resolve::<A>()) }` as the constructor for `B`
#[Transient]
impl B {
    fn new(a: A) -> B {
        B(a)
    }
}

// Register `fn(cx) -> C { C::B(cx.resolve::<B>()) }` as the constructor for `C`
#[allow(dead_code)]
#[Transient]
enum C {
    A(A),

    #[di]
    B(B),
}

// Register `fn(cx) -> () { Run(cx.resolve::<B>()) }` as the constructor for `()`
#[Singleton]
fn Run(b: B, c: C) {
    println!("{:?}", b);
    assert!(matches!(c, C::B(_)));
}

fn main() {
    // Automatically register all types and functions with the `#[Singleton]` or `#[Transient]` attribute.
    let mut cx = Context::auto_register();

    // Get an instance of `()` from the `Context`, which will call the `Run` function.
    // This is equivalent to `cx.resolve::<()>();`
    cx.resolve()
}
```
