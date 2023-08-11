

Both [`Singleton`] and [`Transient`] are attribute macros used to define a [`Provider`], the difference between them is that a `Provider` defined by `#[Singleton]` has a constructor method that is executed only once, while a `Provider` defined by `#[Transient]` has its constructor method can be executed multiple times.

These two macros can be used on `struct`, `impl struct`, and `fn`.

- When used on `struct` and `impl struct`, a [`DefaultProvider`] implementation is generated for the struct, and the associated type [`DefaultProvider::Type`] is the struct itself.

- When used on `fn`, a struct with the same name as the function is created, and then a [`DefaultProvider`] implementation is generated for the struct, with the associated type [`DefaultProvider::Type`] being the return type of the function. As mentioned above, it is recommended to use `CamelCase` when defining functions. Of course, you can still use `snake_case`.

## Example

```rust
use rudi::{Context, Singleton, Transient};

#[derive(Clone)]
struct A;

#[Singleton(name = "a")]
impl A {
    fn new() -> Self {
        Self
    }
}

#[Singleton(name = "b")]
#[derive(Clone)]
struct B(#[di("a")] A);

#[Transient]
fn C(#[di("b")] b: B) -> i32 {
    let _ = b;
    42
}

fn main() {
    let mut cx = Context::auto_register();
    let number = cx.resolve::<i32>();
    println!("number = {}", number);
}
```

## Customization with attributes

### On `struct` and `function`

#### Generic attributes that can be used on `struct`, `impl struct`, and `fn`

- name
  - type: any expression that implements `Into<Cow<'static, str>>`.
  - example: `#[Singleton(name = "abc")]` / `#[Transient(name = a::b::NAME)]` / `#[Transient(name = nth(42))]`
  - optional: true
  - default: ""
  - description: Specifies the name of the defined `Provider`.
  - refer:
    - [`SingletonProvider::name`]
    - [`TransientProvider::name`]
    - [`SingletonAsyncProvider::name`]
    - [`TransientAsyncProvider::name`]

- eager_create
  - type: bool
  - example: `#[Singleton(eager_create)]`
  - optional: true
  - default: false
  - description: Specifies whether the defined `Provider` is eager created.
  - refer:
    - [`SingletonProvider::eager_create`]
    - [`TransientProvider::eager_create`]
    - [`SingletonAsyncProvider::eager_create`]
    - [`TransientAsyncProvider::eager_create`]

- binds
  - type: Array of paths to functions of type `fn(T) -> R`, where `T` is current struct type or current function return type and `R` can be any type.
  - example: `#[Singleton(binds = [Rc::new, Box::new])]`
  - optional: true
  - default: None
  - description: Specifies the field `binding_providers` and `binding_definitions` of the defined `Provider`.
  - refer:
    - [`SingletonProvider::bind`]
    - [`TransientProvider::bind`]
    - [`SingletonAsyncProvider::bind`]
    - [`TransientAsyncProvider::bind`]

- not_auto_register
  - type: bool
  - example: `#[Singleton(not_auto_register)]`
  - optional: true
  - default: false
  - description: Specifies whether a defined `Provider` should be auto-registered to [`AutoRegisterModule`](crate::AutoRegisterModule). When the `auto-register` feature is enabled (which is enabled by default), this attribute can be used if auto-registration is not desired, or if auto-registration is not possible due to the presence of generics.

#### An attribute that can only be used on `struct`

- async_constructor
  - type: bool
  - example: `#[Singleton(async_constructor)]`
  - optional: true
  - default: false
  - description: Specifies whether the constructor method of a defined `Provider` is asynchronous. Only valid when used on `struct`, for `impl struct` and `fn` cases use `async fn`.

### On `field` of struct and `parameter` of function

When adding attributes to `field` of struct and `parameter` of function, you need to use `#[di(...)] `. Since there is only one attribute at the moment, there is no attribute name, just use `#[di(...)] ` is sufficient.

- name (Actually there is no such name, here it is just for the sake of clarification)
  - type: any expression that implements `Into<Cow<'static, str>>`.
  - example: `#[di("abc")]` / `#[di(a::b::NAME)]` / `#[di(nth(42))]`
  - optional: true
  - default: ""
  - description: Specifies the name of the dependency to be taken out of `Context`.
  - refer:
    - [`Context::resolve_with_name`]
    - [`Context::resolve_option_with_name`]
    - [`Context::resolve_with_name_async`]
    - [`Context::resolve_with_name_async`]

## Attributes examples

```rust
use std::{any::Any, borrow::Cow, marker::PhantomData, rc::Rc};

use rudi::{components, modules, AutoRegisterModule, Context, Module, Singleton, Transient};

#[derive(Clone)]
#[Singleton(name = "a", eager_create)]
struct A;

const SOME_NAME: &str = "abc";

const fn some_name() -> &'static str {
    SOME_NAME
}

#[Transient(name = crate::some_name(), binds = [Self::any])]
struct B;

impl B {
    fn any(self) -> Rc<dyn Any> {
        Rc::new(self)
    }
}

fn dep_name() -> impl Into<Cow<'static, str>> {
    "dep"
}

#[Transient(name = crate::dep_name())]
async fn Dep() -> i32 {
    42
}

#[Transient(async_constructor)]
struct C(#[di(crate::dep_name())] i32);

#[Transient(not_auto_register)]
async fn D<T: 'static>(#[di(crate::dep_name())] t: T) -> bool {
    let _ = t;
    true
}

struct MyModule<T>(PhantomData<T>);

impl<T: 'static> Module for MyModule<T> {
    fn providers() -> Vec<rudi::DynProvider> {
        components![D::<T>]
    }
}

#[tokio::main]
async fn main() {
    let mut cx = Context::create(modules![AutoRegisterModule, MyModule<i32>]);

    assert!(cx.resolve_option_with_name::<A>("a").is_some());
    assert!(cx.resolve_option_with_name::<B>(some_name()).is_some());
    assert!(cx
        .resolve_option_with_name::<Rc<dyn Any>>(some_name())
        .is_some());
    assert!(cx.resolve_option_async::<C>().await.is_some());
    assert!(cx.resolve_option_async::<bool>().await.is_some());
}
```
